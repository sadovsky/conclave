//! Conclave fetch capability binary.
//!
//! Implements the Conclave capability subprocess ABI (JSON stdio):
//!
//! **stdin** (one newline-terminated JSON line):
//! ```json
//! { "capability": "fetch(Url)->Html",
//!   "inputs": { "url": "https://example.com" },
//!   "context": { "seed": 0, "virtual_time": 0, "determinism_profile": "replayable" } }
//! ```
//!
//! **stdout** on success:
//! ```json
//! { "output": { "type": "Html", "data_b64": "<base64>" }, "duration_ms": 42 }
//! ```
//!
//! **stdout** on error (exit non-zero):
//! ```json
//! { "error": "ERR_FETCH_FAILED", "details": { "url": "...", "cause": "..." } }
//! ```

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{BufRead, Read, Write};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Request {
    capability: String,
    inputs: BTreeMap<String, serde_json::Value>,
    #[allow(dead_code)]
    context: Context,
}

#[derive(Deserialize)]
struct Context {
    #[allow(dead_code)]
    seed: u64,
    #[allow(dead_code)]
    virtual_time: u64,
    #[allow(dead_code)]
    determinism_profile: String,
}

#[derive(Serialize)]
struct SuccessResponse {
    output: Output,
    duration_ms: u64,
}

#[derive(Serialize)]
struct Output {
    #[serde(rename = "type")]
    type_name: String,
    data_b64: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    details: BTreeMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // Read exactly one newline-terminated JSON request from stdin.
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() || line.trim().is_empty() {
        emit_error(&mut out, "ERR_INVALID_REQUEST", BTreeMap::new());
        std::process::exit(1);
    }

    let req: Request = match serde_json::from_str(line.trim()) {
        Ok(r) => r,
        Err(e) => {
            let mut d = BTreeMap::new();
            d.insert("parse_error".into(), serde_json::json!(e.to_string()));
            emit_error(&mut out, "ERR_INVALID_REQUEST", d);
            std::process::exit(1);
        }
    };

    let url = match req.inputs.get("url").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => {
            let mut d = BTreeMap::new();
            d.insert("capability".into(), serde_json::json!(req.capability));
            emit_error(&mut out, "ERR_MISSING_URL", d);
            std::process::exit(1);
        }
    };

    let t0 = Instant::now();
    match ureq::get(&url).call() {
        Ok(response) => {
            let mut body: Vec<u8> = Vec::new();
            if let Err(e) = response.into_reader().read_to_end(&mut body) {
                let mut d = BTreeMap::new();
                d.insert("url".into(), serde_json::json!(url));
                d.insert("cause".into(), serde_json::json!(e.to_string()));
                emit_error(&mut out, "ERR_FETCH_FAILED", d);
                std::process::exit(1);
            }
            let duration_ms = t0.elapsed().as_millis() as u64;
            let data_b64 = base64::engine::general_purpose::STANDARD.encode(&body);
            let resp = SuccessResponse {
                output: Output { type_name: "Html".into(), data_b64 },
                duration_ms,
            };
            let json = serde_json::to_string(&resp).unwrap();
            writeln!(out, "{json}").unwrap();
            out.flush().unwrap();
        }
        Err(e) => {
            let mut d = BTreeMap::new();
            d.insert("url".into(), serde_json::json!(url));
            d.insert("cause".into(), serde_json::json!(e.to_string()));
            emit_error(&mut out, "ERR_FETCH_FAILED", d);
            std::process::exit(1);
        }
    }
}

fn emit_error(out: &mut impl Write, code: &str, details: BTreeMap<String, serde_json::Value>) {
    let resp = ErrorResponse { error: code.into(), details };
    let json = serde_json::to_string(&resp).unwrap();
    let _ = writeln!(out, "{json}");
    let _ = out.flush();
}
