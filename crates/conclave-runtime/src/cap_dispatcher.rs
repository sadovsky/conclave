use crate::dispatch::{ReplayStore, Value};
use crate::error::RuntimeError;
use base64::Engine;
use conclave_manifest::CapabilityBinding;
use conclave_store::CapabilityStore;
use std::collections::BTreeMap;
use std::io::Write as IoWrite;

/// Context injected into every capability subprocess invocation.
#[derive(serde::Serialize)]
struct CapabilityContext {
    seed: u64,
    virtual_time: u64,
    determinism_profile: String,
}

/// Request sent to a capability subprocess on stdin (newline-terminated JSON).
#[derive(serde::Serialize)]
struct CapabilityRequest {
    capability: String,
    inputs: BTreeMap<String, serde_json::Value>,
    context: CapabilityContext,
}

/// Successful response from a capability subprocess.
#[derive(serde::Deserialize)]
struct CapabilityResponse {
    output: CapabilityOutput,
    duration_ms: u64,
}

#[derive(serde::Deserialize)]
struct CapabilityOutput {
    #[serde(rename = "type")]
    type_name: String,
    data_b64: String,
}

/// Error response from a capability subprocess.
#[derive(serde::Deserialize)]
struct CapabilityErrorResponse {
    error: String,
    #[serde(default)]
    details: BTreeMap<String, serde_json::Value>,
}

/// Drives capability dispatch: either from the replay store or via subprocess invocation,
/// based on the manifest's determinism mode and per-capability profile.
pub struct CapabilityDispatcher<'a> {
    pub replay_store: &'a dyn ReplayStore,
    pub cap_store: Option<&'a dyn CapabilityStore>,
    pub bindings: &'a BTreeMap<String, CapabilityBinding>,
    /// "sealed_replay" — only replay; fail on miss.
    /// "live" — try replay first (if replayable), then subprocess.
    pub determinism_mode: String,
    pub seed: u64,
}

impl CapabilityDispatcher<'_> {
    pub fn dispatch(
        &self,
        node_id: &str,
        cap_signature: &str,
        virtual_time: u64,
    ) -> Result<(Value, u64), RuntimeError> {
        let binding = self.bindings.get(cap_signature);

        let profile = binding
            .map(|b| b.determinism_profile.as_str())
            .unwrap_or("replayable");

        // In sealed_replay mode, nondet capabilities are forbidden.
        if self.determinism_mode == "sealed_replay" && profile == "nondet" {
            return Err(RuntimeError::new("ERR_IO_POLICY_VIOLATION")
                .with_node(node_id)
                .with_capability(cap_signature)
                .with_detail("reason", serde_json::Value::String(
                    "nondet capability forbidden in sealed_replay mode".into(),
                )));
        }

        // Always try replay store first (both sealed_replay and live modes).
        if let Some(entry) = self.replay_store.get(cap_signature, node_id) {
            return Ok((entry.output, entry.duration_ms));
        }

        // sealed_replay: replay miss = deterministic error.
        if self.determinism_mode == "sealed_replay" {
            return Err(RuntimeError::new("ERR_REPLAY_MISS")
                .with_node(node_id)
                .with_capability(cap_signature)
                .with_detail("capability", serde_json::Value::String(cap_signature.into())));
        }

        // live mode: invoke subprocess.
        let artifact_bytes = self
            .cap_store
            .and_then(|s| binding.map(|b| s.get(&b.artifact_hash)).flatten())
            .ok_or_else(|| {
                RuntimeError::new("ERR_CAPABILITY_MISSING")
                    .with_node(node_id)
                    .with_capability(cap_signature)
            })?;

        spawn_capability(
            node_id,
            cap_signature,
            &artifact_bytes,
            profile,
            self.seed,
            virtual_time,
        )
    }
}

/// Spawn a capability binary as a subprocess, send request on stdin, read response from stdout.
fn spawn_capability(
    node_id: &str,
    cap_signature: &str,
    artifact_bytes: &[u8],
    determinism_profile: &str,
    seed: u64,
    virtual_time: u64,
) -> Result<(Value, u64), RuntimeError> {
    // Write artifact bytes to a temporary file and make it executable.
    let tmp = tempfile_for_artifact(artifact_bytes, node_id, cap_signature)?;

    // Build the request JSON.
    let request = CapabilityRequest {
        capability: cap_signature.to_string(),
        inputs: BTreeMap::new(), // v0.2: populate from node inputs
        context: CapabilityContext {
            seed,
            virtual_time,
            determinism_profile: determinism_profile.to_string(),
        },
    };
    let request_json =
        serde_json::to_string(&request).expect("CapabilityRequest is always serializable");

    // Spawn subprocess.
    let mut child = std::process::Command::new(&tmp)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            RuntimeError::new("ERR_CAPABILITY_SPAWN_FAILED")
                .with_node(node_id)
                .with_capability(cap_signature)
                .with_detail("error", serde_json::Value::String(e.to_string()))
        })?;

    // Write request to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request_json.as_bytes());
        let _ = stdin.write_all(b"\n");
    }

    // Wait for exit and capture output.
    let output = child.wait_with_output().map_err(|e| {
        RuntimeError::new("ERR_CAPABILITY_FAILED")
            .with_node(node_id)
            .with_capability(cap_signature)
            .with_detail("error", serde_json::Value::String(e.to_string()))
    })?;

    // Clean up temp file (best effort).
    let _ = std::fs::remove_file(&tmp);

    if !output.status.success() {
        // Try to parse error response from stdout.
        let code = if let Ok(err_resp) =
            serde_json::from_slice::<CapabilityErrorResponse>(&output.stdout)
        {
            err_resp.error
        } else {
            "ERR_CAPABILITY_FAILED".to_string()
        };
        return Err(RuntimeError::new(&code)
            .with_node(node_id)
            .with_capability(cap_signature)
            .with_detail(
                "exit_code",
                serde_json::Value::Number(
                    output.status.code().unwrap_or(-1).into(),
                ),
            ));
    }

    // Parse successful response.
    let resp: CapabilityResponse = serde_json::from_slice(&output.stdout).map_err(|e| {
        RuntimeError::new("ERR_CAPABILITY_BAD_RESPONSE")
            .with_node(node_id)
            .with_capability(cap_signature)
            .with_detail("parse_error", serde_json::Value::String(e.to_string()))
    })?;

    let data = base64::engine::general_purpose::STANDARD
        .decode(&resp.output.data_b64)
        .map_err(|e| {
            RuntimeError::new("ERR_CAPABILITY_BAD_RESPONSE")
                .with_node(node_id)
                .with_capability(cap_signature)
                .with_detail("base64_error", serde_json::Value::String(e.to_string()))
        })?;

    Ok((Value { type_name: resp.output.type_name, data }, resp.duration_ms))
}

/// Write artifact bytes to a temp file and make it executable. Returns the path.
fn tempfile_for_artifact(
    bytes: &[u8],
    node_id: &str,
    cap_signature: &str,
) -> Result<std::path::PathBuf, RuntimeError> {
    use std::os::unix::fs::PermissionsExt;

    let path = std::env::temp_dir()
        .join(format!("conclave_cap_{}", conclave_hash::sha256_bytes(bytes)));

    std::fs::write(&path, bytes).map_err(|e| {
        RuntimeError::new("ERR_CAPABILITY_SPAWN_FAILED")
            .with_node(node_id)
            .with_capability(cap_signature)
            .with_detail("io_error", serde_json::Value::String(e.to_string()))
    })?;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| {
        RuntimeError::new("ERR_CAPABILITY_SPAWN_FAILED")
            .with_node(node_id)
            .with_capability(cap_signature)
            .with_detail("chmod_error", serde_json::Value::String(e.to_string()))
    })?;

    Ok(path)
}
