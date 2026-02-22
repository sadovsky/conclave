#!/usr/bin/env python3
"""
Conclave word_count capability — Python implementation.

Implements the Conclave capability subprocess ABI (JSON stdio).

stdin  → one newline-terminated JSON request:
  {"capability": "word_count(Html)->String",
   "inputs": {"html": "<p>Hello world</p>"},
   "context": {"seed": 1337, "virtual_time": 0, "determinism_profile": "replayable"}}

stdout → success:
  {"output": {"type": "String", "data_b64": "<base64 of 'N words'>"}, "duration_ms": 1}
"""
import base64
import json
import re
import sys


def emit_error(code, details):
    print(json.dumps({"error": code, "details": details}), flush=True)


def strip_html_tags(html):
    """Replace HTML tags with spaces using a simple regex."""
    return re.sub(r'<[^>]*>', ' ', html)


def count_words(html):
    """Strip HTML tags and count whitespace-delimited tokens."""
    text = strip_html_tags(html)
    tokens = [t for t in text.split() if t]
    return len(tokens)


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

    html = req.get("inputs", {}).get("html")
    if html is None:
        emit_error("ERR_MISSING_HTML", {"capability": req.get("capability", "")})
        sys.exit(1)

    n = count_words(html)
    result = f"{n} words"
    data_b64 = base64.b64encode(result.encode("utf-8")).decode("ascii")
    print(json.dumps({
        "output": {"type": "String", "data_b64": data_b64},
        "duration_ms": 1,
    }), flush=True)


if __name__ == "__main__":
    main()
