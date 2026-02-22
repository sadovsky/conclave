use crate::cap_dispatcher::CapabilityDispatcher;
use crate::clock::VirtualClock;
use crate::dispatch::Value;
use crate::error::RuntimeError;
use crate::rate_limiter::TokenBucket;
use crate::trace::TraceEmitter;
use conclave_ir::{Node, NodeKind, PlanIr};
use conclave_manifest::SchedulerPolicy;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Node execution state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum NodeState {
    Pending,
    Ready,
    Running { completion_t: u64 },
    Completed,
    Failed,
}

impl PartialEq for NodeState {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (NodeState::Pending, NodeState::Pending)
                | (NodeState::Ready, NodeState::Ready)
                | (NodeState::Completed, NodeState::Completed)
        )
    }
}

struct NodeData {
    state: NodeState,
    output: Option<Value>,
    /// Pre-computed dispatch result for capability calls; set when dispatched.
    pending_result: Option<Result<(Value, u64), RuntimeError>>,
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

pub struct Scheduler {
    policy: SchedulerPolicy,
}

impl Scheduler {
    pub fn new(policy: SchedulerPolicy) -> Self {
        Scheduler { policy }
    }

    /// Execute the Plan IR deterministically, returning outputs from completed nodes.
    pub fn run(
        &mut self,
        plan_ir: &PlanIr,
        dispatcher: &CapabilityDispatcher<'_>,
        trace: &mut TraceEmitter,
    ) -> Result<BTreeMap<String, Value>, RuntimeError> {
        // Build kind priority map: kind_label (uppercase) -> priority index.
        let kind_priority: BTreeMap<String, usize> = self
            .policy
            .node_kind_order
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();

        // Rate limiters keyed by capability signature.
        let mut rate_limiters: BTreeMap<String, TokenBucket> = BTreeMap::new();

        let mut clock = VirtualClock::new();

        // Initialize per-node state (BTreeMap ensures deterministic iteration).
        let mut nodes: BTreeMap<String, NodeData> = plan_ir
            .nodes
            .iter()
            .map(|n| {
                (
                    n.node_id.clone(),
                    NodeData { state: NodeState::Pending, output: None, pending_result: None },
                )
            })
            .collect();

        // Build dependency map: node_id -> [node_ids that must complete first].
        let mut deps_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        // Build edge-source map: edge_id -> from_node_id (for resolving input values).
        let edge_source_map: BTreeMap<String, String> = plan_ir
            .edges
            .iter()
            .map(|e| (e.edge_id.clone(), e.from.node_id.clone()))
            .collect();

        for node in &plan_ir.nodes {
            let mut deps = Vec::new();
            for input in &node.inputs {
                if let Some(source) = &input.source {
                    if let Some(edge) = plan_ir.edges.iter().find(|e| e.edge_id == source.edge_id) {
                        deps.push(edge.from.node_id.clone());
                    }
                }
            }
            deps_map.insert(node.node_id.clone(), deps);
        }

        // Lookup map for IR nodes by id.
        let node_lookup: BTreeMap<&str, &Node> =
            plan_ir.nodes.iter().map(|n| (n.node_id.as_str(), n)).collect();

        // Main scheduler loop.
        loop {
            // 1. Collect set of completed node_ids.
            let completed: Vec<String> = nodes
                .iter()
                .filter(|(_, d)| matches!(d.state, NodeState::Completed))
                .map(|(id, _)| id.clone())
                .collect();

            // 2. Promote Pending -> Ready when all deps are Completed.
            for (node_id, data) in nodes.iter_mut() {
                if data.state != NodeState::Pending {
                    continue;
                }
                let all_done = deps_map
                    .get(node_id)
                    .map(|deps| deps.iter().all(|d| completed.contains(d)))
                    .unwrap_or(true);
                if all_done {
                    data.state = NodeState::Ready;
                }
            }

            // 3. Collect Ready node_ids as owned Strings (avoids holding borrows
            //    into `nodes` while we later mutably borrow it during dispatch).
            let mut ready_ids: Vec<String> = nodes
                .iter()
                .filter(|(_, d)| d.state == NodeState::Ready)
                .map(|(id, _)| id.clone())
                .collect();

            ready_ids.sort_by(|a, b| {
                let na = node_lookup[a.as_str()];
                let nb = node_lookup[b.as_str()];
                let ua = na.attrs.url_index.unwrap_or(u32::MAX);
                let ub = nb.attrs.url_index.unwrap_or(u32::MAX);
                if ua != ub { return ua.cmp(&ub); }
                let ka = node_kind_priority(na, &kind_priority);
                let kb = node_kind_priority(nb, &kind_priority);
                if ka != kb { return ka.cmp(&kb); }
                a.cmp(b)
            });

            // 4. Count currently inflight (Running) nodes.
            let inflight = nodes
                .values()
                .filter(|d| matches!(d.state, NodeState::Running { .. }))
                .count() as u32;

            // 5. Dispatch ready nodes up to max_inflight.
            let mut dispatched_any = false;
            let mut current_inflight = inflight;

            for node_id in &ready_ids {
                let node_id: &str = node_id.as_str();
                if current_inflight >= self.policy.max_inflight {
                    break;
                }

                let ir_node = node_lookup[node_id];
                let cap_sig = &ir_node.op.signature;

                // Rate-limit capability calls.
                if matches!(ir_node.kind, NodeKind::CapabilityCall) {
                    let bucket = rate_limiters.entry(cap_sig.clone()).or_insert_with(|| {
                        TokenBucket::new(1000, 2) // 2 req/s default; v0.1 conformance
                    });
                    if bucket.try_consume(clock.now()).is_err() {
                        continue; // Window exhausted; skip.
                    }
                }

                // Resolve upstream dependency outputs as extra inputs for capability calls.
                // For each input port that has a source edge, look up the completed upstream
                // node's output and inject it with a type-derived key.
                let extra_inputs = if matches!(ir_node.kind, NodeKind::CapabilityCall) {
                    let mut extras: BTreeMap<String, serde_json::Value> = BTreeMap::new();
                    for input_port in &ir_node.inputs {
                        if let Some(source) = &input_port.source {
                            if let Some(upstream_node_id) = edge_source_map.get(&source.edge_id) {
                                if let Some(upstream_data) = nodes.get(upstream_node_id) {
                                    if let Some(upstream_output) = &upstream_data.output {
                                        // Use the lowercase type name as the input key.
                                        let key = input_port.type_name.to_lowercase();
                                        let value_str = String::from_utf8_lossy(&upstream_output.data).to_string();
                                        extras.insert(key, serde_json::Value::String(value_str));
                                    }
                                }
                            }
                        }
                    }
                    extras
                } else {
                    BTreeMap::new()
                };

                // Resolve the result and duration now (single-threaded simulation).
                let result: Result<(Value, u64), RuntimeError> = if matches!(ir_node.kind, NodeKind::CapabilityCall) {
                    dispatcher.dispatch(node_id, cap_sig, ir_node.attrs.url_index, clock.now(), extra_inputs)
                } else {
                    let dur = local_duration_for(ir_node);
                    Ok((Value { type_name: "()".into(), data: vec![] }, dur))
                };

                let completion_t = match &result {
                    Ok((_, dur)) => clock.now() + dur,
                    Err(_) => clock.now(), // Errors complete immediately.
                };

                trace.dispatch(clock.now(), node_id);

                let data = nodes.get_mut(node_id).unwrap();
                data.state = NodeState::Running { completion_t };
                data.pending_result = Some(result);
                current_inflight += 1;
                dispatched_any = true;
            }

            // 6. Find earliest-completing Running node.
            let running: Vec<(&str, u64)> = nodes
                .iter()
                .filter_map(|(id, d)| {
                    if let NodeState::Running { completion_t } = d.state {
                        Some((id.as_str(), completion_t))
                    } else {
                        None
                    }
                })
                .collect();

            if running.is_empty() {
                // Nothing running. Check if we can advance time to unlock rate limits.
                if !dispatched_any && ready_ids.is_empty() {
                    break; // Fully done or deadlock.
                }

                // Advance to next rate window.
                let next_window = rate_limiters
                    .values()
                    .filter_map(|b| b.next_window_start_if_exhausted(clock.now()))
                    .min();
                if let Some(t) = next_window {
                    clock.advance_to(t);
                    continue;
                }
                break;
            }

            // 7. Advance to the earliest completion, with deterministic tie-breaking.
            let min_ct = running.iter().map(|(_, ct)| *ct).min().unwrap();
            let mut earliest: Vec<&str> = running
                .iter()
                .filter(|(_, ct)| *ct == min_ct)
                .map(|(id, _)| *id)
                .collect();
            earliest.sort_by(|a, b| {
                let na = node_lookup[a];
                let nb = node_lookup[b];
                let ua = na.attrs.url_index.unwrap_or(u32::MAX);
                let ub = nb.attrs.url_index.unwrap_or(u32::MAX);
                if ua != ub { return ua.cmp(&ub); }
                let ka = node_kind_priority(na, &kind_priority);
                let kb = node_kind_priority(nb, &kind_priority);
                if ka != kb { return ka.cmp(&kb); }
                a.cmp(b)
            });
            let complete_node_id = earliest[0].to_string();

            clock.advance_to(min_ct);
            trace.complete(clock.now(), &complete_node_id);

            let data = nodes.get_mut(&complete_node_id).unwrap();
            let result = data.pending_result.take().unwrap_or(Ok((
                Value { type_name: "()".into(), data: vec![] },
                0,
            )));
            match result {
                Ok((output, _)) => {
                    data.output = Some(output);
                    data.state = NodeState::Completed;
                }
                Err(e) => {
                    data.state = NodeState::Failed;
                    return Err(e);
                }
            }
        }

        // Collect outputs.
        Ok(nodes
            .into_iter()
            .filter_map(|(id, d)| d.output.map(|v| (id, v)))
            .collect())
    }
}

fn node_kind_priority(node: &Node, kind_priority: &BTreeMap<String, usize>) -> usize {
    let label = match node.kind {
        NodeKind::CapabilityCall => node.op.name.to_uppercase(),
        NodeKind::Intrinsic => node.op.name.to_uppercase(),
        NodeKind::Control => "CONTROL".to_string(),
        NodeKind::Aggregate => "ASSEMBLE".to_string(),
    };
    *kind_priority.get(&label).unwrap_or(&usize::MAX)
}

/// Fixed virtual duration for local (non-replayable) operations.
fn local_duration_for(node: &Node) -> u64 {
    match node.kind {
        NodeKind::Intrinsic | NodeKind::CapabilityCall => match node.op.name.as_str() {
            "extract_text" => 15,
            "summarize" => 85,
            "assemble_json" | "assemble" => 4,
            _ => 0,
        },
        NodeKind::Aggregate => 4,
        NodeKind::Control => 0,
    }
}
