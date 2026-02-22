# An Agent Wrote a Capability: Watching the Feedback Loop Close

*February 2026*

One of the design goals for Conclave has always been that it should be *agent-friendly*. Not just usable by humans who read documentation, but generatable by AI agents that read a guide and run commands. This post documents the first time we actually tested that: we gave an agent the AGENT_GUIDE, told it to write a new capability from scratch, and let it run the full pipeline.

Here's exactly what happened.

---

## The task

We asked the agent to:

1. Write a new `word_count` capability — a Python script that receives HTML on stdin, strips tags, and counts whitespace-delimited tokens
2. Write a Conclave program that uses `fetch` + `word_count` to count words on two web pages
3. Run the full pipeline: `lower → seal → pack → run`
4. Fix any errors it encountered

No hand-holding. No corrections from us. Just the guide and the CLI.

---

## What the agent wrote

### The capability: `cap_word_count.py`

The agent wrote a 50-line Python script implementing the Conclave capability ABI. The protocol is simple: read one JSON line from stdin, parse the HTML from `inputs.html`, count words, return base64-encoded output to stdout.

```python
#!/usr/bin/env python3
import base64, json, re, sys

def strip_html_tags(html):
    return re.sub(r'<[^>]*>', ' ', html)

def count_words(html):
    tokens = [t for t in strip_html_tags(html).split() if t]
    return len(tokens)

def main():
    req = json.loads(sys.stdin.readline().strip())
    html = req["inputs"]["html"]
    n = count_words(html)
    result = f"{n} words"
    data_b64 = base64.b64encode(result.encode()).decode()
    print(json.dumps({"output": {"type": "String", "data_b64": data_b64}, "duration_ms": 1}))

main()
```

The agent got the full protocol right on the first try: JSON on stdin, base64-encoded output, single line to stdout, exit 0.

### The program: `count_words.conclave`

```conclave
version 0.1;

type Url = String where re2("^https?://");

capability fetch: fetch(Url) -> Html;
capability word_count: word_count(Html) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;

goal CountWords(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      let count = word_count(page);
      emit count;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
    scheduler.max_inflight <= 2;
  }
}
```

Valid syntax, correct structure, two-step pipeline per URL. The agent produced this without any corrections.

---

## The commands, in order

### Install both capabilities

```
$ ./target/release/conclave install-cap ./target/release/conclave-cap-fetch
sha256:ee1f247bd06632de389269e81a05ccd0361c60e2729eb7f49deaefbb39ae1216

$ ./target/release/conclave install-cap /tmp/agent_word_count/cap_word_count.py
sha256:4f2d88d0d5040a6266a37cdd0d9fe9017f73f41c1e0367a7d12de453b38be026
```

Two content-addressed hashes. The Python script is now a first-class artifact in the capability store.

### Lower to Plan IR

```
$ ./target/release/conclave lower count_words.conclave \
    --url-count 2 --output plan_ir.json

source_hash:  sha256:24aa52bb09d85c3f6a1a84e71584f7ccc10c14f57b29a0bc6a7a64c64ac235e4
ast_hash:     sha256:e027919d224f7d90c69b4c9b6f8dadc4ff8f940d2eae385a1bee853470fbfa14
plan_ir_hash: sha256:88f40befe596f584f92f9f6413795dc13980f5debb35b41a4d5abdf3144623a0
```

Three hashes. Source bytes, parsed AST, and expanded execution graph — each a distinct commitment.

### Seal (with one fix)

The agent's first seal attempt failed:

```
$ ./target/release/conclave seal \
    --plan plan_ir.json --manifest manifest.json --output sealed.json

error: seal failed: network capability 'fetch(Url)->Html' must be configured
       as replay-only in sealed_replay mode
```

The manifest's `fetch` binding was missing `"fetch_mode": "replay"` in its `config` block. The agent added it and also corrected the `word_count` trust level from `"sandboxed_network_only"` to `"sandboxed"` (word_count doesn't need network access). Second attempt:

```
$ ./target/release/conclave seal \
    --plan plan_ir.json --manifest manifest.json --output sealed.json

plan_ir_hash:            sha256:88f40befe596f584f92f9f6413795dc13980f5debb35b41a4d5abdf3144623a0
canonical_manifest_hash: sha256:48c3bad335a2cf3a0269ed5d8e2637ee5671f82245e240d1024554ad0faec0f6
```

### Pack

```
$ ./target/release/conclave pack \
    --runtime ./target/release/conclave \
    --plan plan_ir.json \
    --manifest sealed.json \
    --output artifact.cnclv

artifact_hash: sha256:16716d7ee446c9a21f4630f90f96113b81f43a3f61e3e1fbb34f2f2da9a1b618
bundle_hash:   sha256:4aa7c178c60c80ac90d904af7711e9ec47749c16b130732efebb5e40c354ec63
artifact written to: artifact.cnclv
```

2.7 MB self-contained artifact. Includes the Conclave runtime binary, the sealed Plan IR, and the sealed manifest.

### Run (with two fixes)

**First attempt:** wrong capability store path on macOS.

```
$ ./target/release/conclave run artifact.cnclv \
    --cap-store ~/.cache/conclave/caps \
    --urls "https://example.com,https://anthropic.com" \
    --trace-out trace.json --mode live

error: runtime error: ERR_CAPABILITY_MISSING
```

The agent remembered that macOS uses `~/Library/Caches/` not `~/.cache/`. Fixed.

**Second attempt:** a real runtime gap.

```
$ ./target/release/conclave run artifact.cnclv \
    --cap-store ~/Library/Caches/conclave/caps \
    --urls "https://example.com,https://anthropic.com" \
    --trace-out trace.json --mode live

error: runtime error: ERR_MISSING_HTML
```

This one was more interesting. The `word_count` capability was being called, but `inputs.html` was empty — the fetched HTML from the upstream `fetch` node was never passed through as an input.

---

## The runtime gap the agent found

The v0.1 runtime scheduler knew how to inject URL strings into capability calls (for the first node in each chain), but it didn't know how to thread output from one capability into the input of the next. For a single-step pipeline like `fetch(url) → emit page`, that was fine. For a two-step pipeline like `fetch(url) → word_count(page)`, the scheduler was calling `word_count` with an empty inputs map.

The agent diagnosed this and patched `cap_dispatcher.rs` and `scheduler.rs`. The fix:

**In `scheduler.rs`:** before dispatching each capability node, build an `edge_source_map` that tracks which upstream node produced each input port's value. For each input port on the current node, look up the upstream node's completed output and inject it into `extra_inputs` using the lowercased type name as the key (`Html` → `"html"`).

**In `cap_dispatcher.rs`:** accept an `extra_inputs: BTreeMap<String, Value>` parameter and merge it with URL-injected inputs before building the subprocess request.

```rust
// scheduler.rs — resolve upstream outputs as extra inputs
let extra_inputs = if matches!(ir_node.kind, NodeKind::CapabilityCall) {
    let mut extras: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for input_port in &ir_node.inputs {
        if let Some(source) = &input_port.source {
            if let Some(upstream_node_id) = edge_source_map.get(&source.edge_id) {
                if let Some(upstream_data) = nodes.get(upstream_node_id) {
                    if let Some(upstream_output) = &upstream_data.output {
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
```

This was a real gap in the runtime that the agent's test exposed. It only mattered once you had a multi-step pipeline where a downstream capability consumed the output of an upstream one — something the single-step `fetch` program never exercised.

**Third attempt — success:**

```
$ ./target/release/conclave run artifact.cnclv \
    --cap-store ~/Library/Caches/conclave/caps \
    --urls "https://example.com,https://anthropic.com" \
    --trace-out trace.json --mode live

trace_hash: sha256:68e0a74759d2f042d7793c6311dc9f43c76ca23f8c0a1e77d52bce64b1f2128d
completed nodes: 5
```

---

## The execution trace

The trace shows the scheduler interleaving two independent fetch-then-count chains:

```json
[
  {"event":"DISPATCH","node":"sha256:d169...","t":0},
  {"event":"DISPATCH","node":"sha256:01a3...","t":0},
  {"event":"COMPLETE","node":"sha256:d169...","t":196},
  {"event":"DISPATCH","node":"sha256:141f...","t":196},
  {"event":"COMPLETE","node":"sha256:141f...","t":197},
  {"event":"COMPLETE","node":"sha256:01a3...","t":471},
  {"event":"DISPATCH","node":"sha256:20f2...","t":471},
  {"event":"COMPLETE","node":"sha256:20f2...","t":472},
  {"event":"DISPATCH","node":"sha256:be05...","t":472},
  {"event":"COMPLETE","node":"sha256:be05...","t":476}
]
```

The pattern is exactly what you'd expect: both `fetch` nodes dispatch immediately (t=0). The first completes at t=196ms; its dependent `word_count` fires immediately (t=196) and completes at t=197ms (Python, 1ms of computation). The second fetch finishes at t=471ms; its `word_count` fires at t=471, completes at t=472. The final `assemble_json` aggregate fires at t=472 and completes at t=476.

10 events, 5 nodes, 1 trace hash. Run it again with the same URLs in `sealed_replay` mode and you get the same hash.

---

## What the agent got right and wrong

**Right on the first try:**
- Valid `.conclave` syntax — version line, capability/intrinsic declarations, map block, emit, return
- Python capability ABI — stdin JSON, base64 output, exit 0
- CLI flags — `--plan`, `--manifest`, `--runtime`, `--url-count` (the guide was accurate)
- Manifest structure — both bindings, scheduler policy, determinism block

**Required fixes:**
1. **Seal config** — `"fetch_mode": "replay"` missing from fetch's config block. The guide mentioned `io_policy` values but not this field. Fixable with one read of the error message.
2. **Cap store path** — macOS stores caches in `~/Library/Caches/` not `~/.cache/`. Platform-specific; easy to diagnose.
3. **Runtime gap** — upstream outputs not threaded to downstream capabilities. This was a real bug in the runtime, not an agent error. The agent diagnosed it correctly and wrote the fix.

---

## What this means

A Python script and a 20-line Conclave program, written by an agent from a guide, exposed a real gap in the runtime that hadn't been exercised before. The agent then fixed it.

That's the feedback loop working. The program is declarative enough that the agent could write it without knowing the runtime internals. The runtime is deterministic enough that the error was reproducible and diagnosable. The hash chain made it clear exactly where the failure occurred.

The fix is now in the codebase. The runtime correctly threads upstream capability outputs to downstream inputs. Multi-step capability pipelines work.

---

## Files created by the agent

| File | Description |
|---|---|
| `examples/cap_word_count.py` | Python word_count capability |
| `/tmp/agent_word_count/count_words.conclave` | Conclave source program |
| `crates/conclave-runtime/src/cap_dispatcher.rs` | Patched: accepts `extra_inputs` |
| `crates/conclave-runtime/src/scheduler.rs` | Patched: resolves upstream outputs |

All 134 workspace tests pass after the patch.

---

## Reproducing this

```bash
# Install the Python capability
./target/release/conclave install-cap examples/cap_word_count.py
# → sha256:<WORD_COUNT_HASH>

# Write count_words.conclave (see above), then:
./target/release/conclave lower count_words.conclave --url-count 2 --output plan_ir.json

# Create manifest.json with both capability hashes, then:
./target/release/conclave seal --plan plan_ir.json --manifest manifest.json --output sealed.json
./target/release/conclave pack \
  --runtime ./target/release/conclave \
  --plan plan_ir.json --manifest sealed.json --output artifact.cnclv

./target/release/conclave run artifact.cnclv \
  --urls "https://example.com,https://anthropic.com" \
  --trace-out trace.json --mode live
```

The `word_count` capability is now in `examples/`. You can use it in any Conclave program that wants to count words in fetched HTML.
