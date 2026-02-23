use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub conclave_manifest_version: String, // "0.1"
    pub program: Program,
    pub target: Target,
    pub toolchain: Toolchain,
    pub capability_bindings: BTreeMap<String, CapabilityBinding>,
    /// Module import bindings: import name → plan_ir_hash of the imported sub-goal.
    /// Populated by the sealer when the Plan IR declares `imports`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub module_bindings: BTreeMap<String, String>,
    pub scheduler_policy: SchedulerPolicy,
    pub determinism: Determinism,
    pub observability: Observability,
    pub supply_chain: SupplyChain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub name: String,
    pub plan_ir_hash: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Target {
    pub triple: String,
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Toolchain {
    pub lowerer_hash: String,
    pub runtime_hash: String,
    pub stdlib_hash: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapabilityBinding {
    pub capability_name: String,
    /// Must be a pinned "sha256:..." hash; no floating references allowed.
    pub artifact_hash: String,
    pub determinism_profile: String, // "replayable" | "fixed" | "nondet"
    pub trust: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<CapabilitySignatures>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapabilitySignatures {
    pub required: bool,
    #[serde(default)]
    pub accepted_keys: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchedulerPolicy {
    pub strategy: String,
    pub max_inflight: u32,
    pub ready_queue_order: Vec<String>,
    pub node_kind_order: Vec<String>,
    pub tie_breaker: TieBreaker,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TieBreaker {
    pub kind: String,
    pub seed: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Determinism {
    pub mode: String,  // "sealed_replay"
    pub clock: String, // "virtual"
    pub randomness: RandomnessPolicy,
    pub float: String, // "strict"
    pub io_policy: IoPolicy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RandomnessPolicy {
    pub allowed: bool,
    pub seed: u64,
    pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IoPolicy {
    pub network: NetworkPolicy,
    pub filesystem: FilesystemPolicy,
    pub env: EnvPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Deny,
    ReplayOnly,
    Live,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemPolicy {
    Deny,
    Sandboxed,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvPolicy {
    Frozen,
    Host,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Observability {
    pub trace_level: String,
    pub emit_scheduler_trace: bool,
    pub emit_capability_metrics: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupplyChain {
    pub artifact_store: String,
    pub require_artifact_signatures: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_signature: Option<ManifestSignature>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestSignature {
    pub algo: String,
    pub public_key_id: String,
    /// Excluded from canonical_manifest_hash computation (a signature can't sign itself).
    pub signature: String,
}
