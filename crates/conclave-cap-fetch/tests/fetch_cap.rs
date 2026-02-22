//! Integration tests for the conclave-cap-fetch binary.
//!
//! These tests spawn the actual binary and exercise the JSON stdio ABI.
//! A `tiny_http` server runs locally so no real network access is required.

use base64::Engine as _;
use std::io::Write;
use std::process::{Command, Stdio};

const CAP_BIN: &str = env!("CARGO_BIN_EXE_conclave-cap-fetch");

fn make_request(url: &str) -> String {
    serde_json::json!({
        "capability": "fetch(Url)->Html",
        "inputs": { "url": url },
        "context": { "seed": 0, "virtual_time": 0, "determinism_profile": "replayable" }
    })
    .to_string()
}

fn spawn_cap(request: &str) -> std::process::Output {
    let mut child = Command::new(CAP_BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn conclave-cap-fetch");
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{request}").unwrap();
    drop(stdin);
    child.wait_with_output().unwrap()
}

// ---------------------------------------------------------------------------
// Success path: local HTTP server returns a fixed body
// ---------------------------------------------------------------------------

#[test]
fn fetch_cap_returns_html_body() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/");

    // Serve one request on a background thread.
    let handle = std::thread::spawn(move || {
        let req = server.recv().unwrap();
        let body = "<html><body>hello from conclave</body></html>";
        let response = tiny_http::Response::from_string(body)
            .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
        req.respond(response).unwrap();
    });

    let output = spawn_cap(&make_request(&url));
    handle.join().unwrap();

    assert!(
        output.status.success(),
        "binary exited non-zero: {:?}",
        output.status
    );

    let resp: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");

    let data_b64 = resp["output"]["data_b64"]
        .as_str()
        .expect("output.data_b64 must be present");
    let body = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .unwrap();
    assert_eq!(
        body, b"<html><body>hello from conclave</body></html>",
        "decoded body must match server response"
    );

    assert_eq!(resp["output"]["type"], "Html");

    let duration_ms = resp["duration_ms"]
        .as_u64()
        .expect("duration_ms must be present");
    assert!(duration_ms < 5_000, "unreasonably slow: {duration_ms}ms");
}

// ---------------------------------------------------------------------------
// Determinism: two runs against the same server return identical base64
// ---------------------------------------------------------------------------

#[test]
fn fetch_cap_is_byte_stable_for_same_content() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/");

    // Serve two identical requests.
    let handle = std::thread::spawn(move || {
        for _ in 0..2 {
            let req = server.recv().unwrap();
            req.respond(tiny_http::Response::from_string("<html>stable</html>"))
                .unwrap();
        }
    });

    let o1 = spawn_cap(&make_request(&url));
    let o2 = spawn_cap(&make_request(&url));
    handle.join().unwrap();

    let r1: serde_json::Value = serde_json::from_slice(&o1.stdout).unwrap();
    let r2: serde_json::Value = serde_json::from_slice(&o2.stdout).unwrap();

    assert_eq!(
        r1["output"]["data_b64"], r2["output"]["data_b64"],
        "identical server content must produce identical base64 output"
    );
}

// ---------------------------------------------------------------------------
// Error: missing url field in inputs
// ---------------------------------------------------------------------------

#[test]
fn fetch_cap_errors_on_missing_url() {
    let request = serde_json::json!({
        "capability": "fetch(Url)->Html",
        "inputs": {},
        "context": { "seed": 0, "virtual_time": 0, "determinism_profile": "replayable" }
    })
    .to_string();

    let output = spawn_cap(&request);

    assert!(
        !output.status.success(),
        "binary should exit non-zero for missing url"
    );

    let resp: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should contain JSON error");
    assert_eq!(resp["error"], "ERR_MISSING_URL");
}

// ---------------------------------------------------------------------------
// Error: malformed JSON request
// ---------------------------------------------------------------------------

#[test]
fn fetch_cap_errors_on_invalid_json() {
    let output = spawn_cap("not json at all");

    assert!(
        !output.status.success(),
        "binary should exit non-zero for invalid JSON"
    );

    let resp: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should contain JSON error");
    assert_eq!(resp["error"], "ERR_INVALID_REQUEST");
}

// ---------------------------------------------------------------------------
// Error: unreachable host → ERR_FETCH_FAILED
// ---------------------------------------------------------------------------

#[test]
fn fetch_cap_errors_on_unreachable_host() {
    // Port 1 is reliably unreachable on loopback.
    let output = spawn_cap(&make_request("http://127.0.0.1:1/"));

    assert!(
        !output.status.success(),
        "binary should exit non-zero for unreachable host"
    );

    let resp: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should contain JSON error");
    assert_eq!(resp["error"], "ERR_FETCH_FAILED");
    assert!(
        resp["details"]["url"].as_str().is_some(),
        "details.url should be present"
    );
}
