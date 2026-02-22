#!/usr/bin/env python3
"""
Conclave fetch capability — Python implementation.

Implements the Conclave capability subprocess ABI (JSON stdio).

stdin  → one newline-terminated JSON request:
  {"capability": "fetch(Url)->Html",
   "inputs": {"url": "https://example.com"},
   "context": {"seed": 0, "virtual_time": 0, "determinism_profile": "replayable"}}

stdout → success:
  {"output": {"type": "Html", "data_b64": "<base64>"}, "duration_ms": 42}

stdout → error (exit 1):
  {"error": "ERR_FETCH_FAILED", "details": {"url": "...", "cause": "..."}}
"""
import base64
import json
import sys
import time
import urllib.error
import urllib.request


def emit_error(code, details):
    print(json.dumps({"error": code, "details": details}), flush=True)


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

    url = req.get("inputs", {}).get("url")
    if not url:
        emit_error("ERR_MISSING_URL", {"capability": req.get("capability", "")})
        sys.exit(1)

    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(url, timeout=30) as resp:
            body = resp.read()
    except urllib.error.URLError as e:
        emit_error("ERR_FETCH_FAILED", {"url": url, "cause": str(e)})
        sys.exit(1)
    except Exception as e:
        emit_error("ERR_FETCH_FAILED", {"url": url, "cause": str(e)})
        sys.exit(1)

    elapsed_ms = int((time.perf_counter() - t0) * 1000)
    data_b64 = base64.b64encode(body).decode("ascii")
    print(json.dumps({
        "output": {"type": "Html", "data_b64": data_b64},
        "duration_ms": elapsed_ms,
    }), flush=True)


if __name__ == "__main__":
    main()
