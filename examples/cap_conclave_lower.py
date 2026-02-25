#!/usr/bin/env python3
"""
Conclave 'conclave_lower' capability.

This capability receives a path to a .conclave source file and runs
`conclave lower` on it, returning the canonical Plan IR JSON.

This is the 'Conclave in Conclave' capability: when Conclave runs this,
it is using itself to lower other Conclave programs.

Capability ABI (JSON stdio):

stdin:
  {"capability": "conclave_lower(String)->Json",
   "inputs": {"url": "/path/to/file.conclave"},
   "context": {"seed": 0, "virtual_time": 0, "determinism_profile": "replayable"}}

stdout (success):
  {"output": {"type": "Json", "data_b64": "<base64 Plan IR JSON>"}, "duration_ms": N}

stdout (error, exit 1):
  {"error": "ERR_LOWER_FAILED", "details": {"path": "...", "cause": "..."}}
"""
import base64
import json
import os
import subprocess
import sys
import time


def emit_error(code, details):
    print(json.dumps({"error": code, "details": details}), flush=True)


def find_conclave_binary():
    """Locate the conclave binary: prefer PATH, fall back to workspace target/debug."""
    import shutil
    found = shutil.which("conclave")
    if found:
        return found
    # Resolve relative to this script's location (examples/ → project root → target/debug)
    script_dir = os.path.dirname(os.path.abspath(__file__))
    workspace_root = os.path.dirname(script_dir)
    candidate = os.path.join(workspace_root, "target", "debug", "conclave")
    if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
        return candidate
    return None


def main():
    line = sys.stdin.readline().strip()
    if not line:
        emit_error("ERR_INVALID_REQUEST", {"cause": "empty stdin"})
        sys.exit(1)

    try:
        req = json.loads(line)
    except json.JSONDecodeError as e:
        emit_error("ERR_INVALID_REQUEST", {"parse_error": str(e)})
        sys.exit(1)

    # The source file path arrives as `inputs["url"]` — the dispatcher injects
    # url_inputs[i] under the key "url" for each map iteration.
    source_path = req.get("inputs", {}).get("url", "")
    if not source_path:
        emit_error("ERR_MISSING_PATH", {"capability": req.get("capability", "")})
        sys.exit(1)

    if not os.path.isfile(source_path):
        emit_error("ERR_FILE_NOT_FOUND", {"path": source_path})
        sys.exit(1)

    conclave = find_conclave_binary()
    if not conclave:
        emit_error("ERR_BINARY_NOT_FOUND", {"cause": "conclave binary not found in PATH or target/debug/"})
        sys.exit(1)

    t0 = time.perf_counter()
    try:
        result = subprocess.run(
            [conclave, "lower", source_path],
            capture_output=True,
            text=True,
            timeout=30,
        )
    except subprocess.TimeoutExpired:
        emit_error("ERR_LOWER_TIMEOUT", {"path": source_path})
        sys.exit(1)
    except Exception as e:
        emit_error("ERR_LOWER_FAILED", {"path": source_path, "cause": str(e)})
        sys.exit(1)

    elapsed_ms = int((time.perf_counter() - t0) * 1000)

    if result.returncode != 0:
        emit_error("ERR_LOWER_FAILED", {
            "path": source_path,
            "exit_code": result.returncode,
            "stderr": result.stderr[:500],
        })
        sys.exit(1)

    plan_ir_json = result.stdout.strip()
    if not plan_ir_json:
        emit_error("ERR_LOWER_EMPTY_OUTPUT", {"path": source_path, "stderr": result.stderr[:500]})
        sys.exit(1)

    data_b64 = base64.b64encode(plan_ir_json.encode("utf-8")).decode("ascii")
    print(json.dumps({
        "output": {"type": "Json", "data_b64": data_b64},
        "duration_ms": elapsed_ms,
    }), flush=True)


if __name__ == "__main__":
    main()
