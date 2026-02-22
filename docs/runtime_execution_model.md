# Conclave Runtime Execution Model (v0.1)

This document defines Conclave’s v0.1 runtime: how a sealed program executes deterministically.

---

## 1. Runtime responsibilities

Given:

- Sealed binary containing embedded Plan IR (canonical) and Build Manifest
- Input values

Runtime must:

1. Instantiate deterministic scheduler
2. Execute Plan IR graph
3. Enforce policies (I/O, rate limits, token limits)
4. Dispatch capability calls to bound artifacts
5. Produce goal outputs
6. Emit deterministic traces (if enabled)

---

## 2. Execution semantics

### 2.1 Data model

- Values are immutable blobs tagged with TypeShape.
- Node outputs are single-assignment: written once.
- Edges transport value IDs (optionally content-addressed).

### 2.2 Node lifecycle

Nodes have states:

- `Pending` (waiting for inputs)
- `Ready` (all inputs available, constraints satisfied)
- `Running`
- `Completed`
- `Failed` (deterministic error code)

### 2.3 Readiness rule

A node becomes `Ready` when:

- all inbound edges have completed values
- node-scoped constraints are satisfiable
- global policies allow execution (e.g., I/O permitted)

---

## 3. Deterministic scheduler

### 3.1 Determinism inputs

Scheduler decisions are a function of:

- `plan_ir_hash`
- `scheduler_policy` from manifest
- virtual clock rules
- optional tie-break seed

### 3.2 Ready queue ordering

At each scheduling tick:

1. Collect all `Ready` nodes.
2. Sort using `scheduler_policy.ready_queue_order`, typically:
   - `url_index` (derived from node attrs inside a map)
   - `node_kind` using `node_kind_order`
   - `node_id` lexicographically
3. Dispatch in order until `max_inflight` is reached and gates permit.

### 3.3 Virtual time

Runtime maintains `t_virtual`.

For v0.1, simplest approach:

- `t_virtual` advances on events (node completion).
- Each completion may advance time by:
  - deterministic intrinsic duration (if modeled)
  - replay-recorded duration for replayable capabilities
  - otherwise 0

Rate limit windows derive from `t_virtual` boundaries, not wall-clock.

### 3.4 Rate limiting

Implement deterministic token bucket:

- default window: 1s virtual
- tokens reset at each window boundary
- dispatch of rate-limited nodes consumes tokens
- if no tokens, nodes remain `Ready` but not dispatchable until next window

---

## 4. Capability dispatch

### 4.1 Binding

At startup:

- load capability artifacts by `artifact_hash` from embedded store or content-addressed cache
- verify signatures if required
- instantiate with `config`

### 4.2 Invocation ABI

`invoke(signature, inputs, config, deterministic_context) -> outputs | error`

`deterministic_context` includes:

- seed
- virtual time
- replay handles
- tracing sink

### 4.3 Replayable network

In `sealed_replay`:

- `fetch` consults replay store keyed by normalized request
- if not found: deterministic failure `ERR_REPLAY_MISS`

---

## 5. Intrinsics

Intrinsics are runtime-provided and pinned by toolchain hashes:

- `assemble_json`
- `collect`
- `map` / `join` (lowered as control nodes)
- `validate_json`

---

## 6. Policy enforcement

Enforce manifest `io_policy`:

- network: deny / replay_only / live
- filesystem: deny / sandboxed / host
- env: frozen / host

In `strict` or `sealed_replay`, any violation is deterministic failure:

- `ERR_IO_POLICY_VIOLATION`

---

## 7. Failure model

Errors are structured:

```json
{
  "code": "ERR_REPLAY_MISS",
  "node_id": "nid:...",
  "capability": "fetch(Url)->Html",
  "details": { "url": "..." }
}
```

No nondeterministic stack traces. Debug metadata may exist but MUST NOT affect execution.

---

## 8. Deterministic tracing

If enabled:

- emit a scheduler trace as an ordered list of events
- event ordering derives from scheduler decisions, not wall-clock
- trace is canonical JSON and hashable

Example event:

```json
{ "t": 1000, "event": "DISPATCH", "node_id": "nid:F3" }
```

`trace_hash = sha256(canonical_trace_json)`

---

## 9. Minimal conformance checks

1. Repro build: seal twice → identical `artifact_hash`
2. Repro run: same input + replay store → identical output + `trace_hash`
3. Rate limit: dispatch order matches policy
4. Replay miss: deterministic error code and node id
