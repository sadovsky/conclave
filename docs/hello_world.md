# Hello World: A Complete Conclave Program

This guide walks through every step to write, seal, pack, and run a working Conclave program from scratch. Every command is exact and copy-pasteable.

**What we're building:** a program that fetches two URLs and collects their HTML. One capability (`fetch`), two inputs, one deterministic trace.

---

## Try it with Claude Code first

If you have Claude Code and access to this repo, paste this prompt:

```
You have access to the Conclave repo. Read docs/hello_world.md and docs/AGENT_GUIDE.md.
Then do the following, exactly as the guide describes:

1. Build the CLI and the conclave-cap-fetch capability binary.
2. Install the fetch capability and save its hash.
3. Write hello.conclave (the FetchPages program from hello_world.md) to the repo root.
4. Lower it with --url-count 2 and save plan_ir.json.
5. Write manifest.json using the hash from step 2.
6. Seal, pack, and run the artifact against https://example.com and https://anthropic.com in live mode.
7. Report the source_hash, ast_hash, and plan_ir_hash printed by `conclave lower`.
```

Then follow this guide yourself and compare the hashes from Step 3.

### The challenge

**The `plan_ir_hash` you get from `conclave lower` will be identical to what Claude Code reports** — regardless of who ran it, on what machine, or when.

That hash is a SHA256 over the canonical execution graph derived purely from the source text. It is not a function of the build, the machine, the time, or the capability binaries. If you both wrote the same `hello.conclave` and used `--url-count 2`, your hashes match.

This is the thesis of Conclave made concrete: the plan is a commitment. Same source → same hash. Always.

Try it. Post both hashes. See if they match.

---

## Prerequisites

You need Rust and Cargo. Then build the tools from the repo root:

```bash
cargo build --release
cargo build --release -p conclave-cap-fetch
```

This produces:
- `./target/release/conclave` — the CLI
- `./target/release/conclave-cap-fetch` — the HTTP fetch capability binary

---

## Step 1: Write the program

Create `hello.conclave`:

```conclave
version 0.1;

type Url = String where re2("^https?://");

capability fetch: fetch(Url) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;

goal FetchPages(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      emit page;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "live";
    scheduler.max_inflight <= 2;
  }
}
```

---

## Step 2: Install the fetch capability

```bash
FETCH_HASH=$(./target/release/conclave install-cap ./target/release/conclave-cap-fetch)
echo "fetch hash: $FETCH_HASH"
```

`install-cap` copies the binary into the content-addressed capability store and prints its `sha256:` hash to stdout. The hash is derived from the binary's bytes — it is stable as long as the binary doesn't change.

**Cap store location:**
- macOS: `~/Library/Caches/conclave/caps`
- Linux: `~/.cache/conclave/caps`

---

## Step 3: Lower to Plan IR

```bash
./target/release/conclave lower hello.conclave --url-count 2 --output plan_ir.json
```

`--url-count` must match the number of URLs you will pass at runtime. This produces `plan_ir.json` and prints three hashes to stderr:

```
source_hash:  sha256:<hash of hello.conclave bytes>
ast_hash:     sha256:<hash of parsed AST>
plan_ir_hash: sha256:<hash of execution graph>
```

---

## Step 4: Write the manifest

Create `manifest.json`, substituting your `$FETCH_HASH` from Step 2:

```bash
cat > manifest.json << EOF
{
  "conclave_manifest_version": "0.1",
  "program": { "name": "hello_world", "plan_ir_hash": "" },
  "target": { "triple": "aarch64-apple-darwin", "os": "macos", "arch": "aarch64" },
  "toolchain": {
    "lowerer_hash": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    "runtime_hash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    "stdlib_hash":  "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  },
  "capability_bindings": {
    "fetch(Url)->Html": {
      "capability_name": "fetch",
      "artifact_hash": "$FETCH_HASH",
      "determinism_profile": "replayable",
      "trust": "sandboxed_network_only",
      "config": {},
      "signatures": null
    }
  },
  "scheduler_policy": {
    "strategy": "bounded_parallel_map",
    "max_inflight": 2,
    "ready_queue_order": ["url_index", "node_kind", "node_id"],
    "node_kind_order": ["FETCH"],
    "tie_breaker": { "kind": "stable", "seed": 0 }
  },
  "determinism": {
    "mode": "live",
    "clock": "virtual",
    "randomness": { "allowed": true, "seed": 1337, "source": "ctr_drbg" },
    "float": "strict",
    "io_policy": {
      "network": "sandboxed",
      "filesystem": "sandboxed",
      "env": "frozen"
    }
  },
  "observability": {
    "trace_level": "deterministic",
    "emit_scheduler_trace": true,
    "emit_capability_metrics": true
  },
  "supply_chain": {
    "artifact_store": "content_addressed",
    "require_artifact_signatures": false,
    "manifest_signature": null
  }
}
EOF
```

**Important:** The key in `capability_bindings` must be the normalized signature with no spaces: `"fetch(Url)->Html"` not `"fetch(Url) -> Html"`.

**`plan_ir_hash` in the manifest can be left as `""`** — the `seal` command fills it in automatically from `plan_ir.json`.

---

## Step 5: Seal

```bash
./target/release/conclave seal \
  --plan plan_ir.json \
  --manifest manifest.json \
  --output sealed.json
```

Seal validates all constraints and pins the `plan_ir_hash` into the manifest. It prints:

```
plan_ir_hash:            sha256:<hash>
canonical_manifest_hash: sha256:<hash>
```

If seal fails, check:
- Every capability declared in the `.conclave` file has a binding in `capability_bindings`
- The binding key exactly matches the normalized signature (no spaces, `->` not ` -> `)
- Every `artifact_hash` is a valid `sha256:<64 hex chars>` string

---

## Step 6: Pack

```bash
./target/release/conclave pack \
  --runtime ./target/release/conclave \
  --plan plan_ir.json \
  --manifest sealed.json \
  --output hello.cnclv
```

This produces `hello.cnclv`: a self-contained artifact containing the runtime binary, the Plan IR, and the sealed manifest. It prints:

```
artifact_hash: sha256:<hash>
bundle_hash:   sha256:<hash>
artifact written to: hello.cnclv
```

---

## Step 7: Run

```bash
./target/release/conclave run hello.cnclv \
  --urls "https://example.com,https://anthropic.com" \
  --trace-out trace.json \
  --mode live
```

`--urls` is comma-separated. The count must match `--url-count` from Step 3.

Expected output (stderr):

```
trace_hash: sha256:<hash>
completed nodes: 5
```

The trace is written to `trace.json`. Each event records which node was dispatched or completed and at what virtual clock time.

---

## Step 8: Inspect the artifact

```bash
./target/release/conclave inspect hello.cnclv
```

Prints the bundle hash, plan IR hash, manifest hash, capability bindings, and scheduler policy — everything that was committed at seal time.

---

## The complete script

```bash
#!/bin/bash
set -e

# Build
cargo build --release 2>/dev/null
cargo build --release -p conclave-cap-fetch 2>/dev/null

# Install capability
FETCH_HASH=$(./target/release/conclave install-cap ./target/release/conclave-cap-fetch)
echo "fetch hash: $FETCH_HASH"

# Write program
cat > hello.conclave << 'CONCLAVE'
version 0.1;

type Url = String where re2("^https?://");

capability fetch: fetch(Url) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;

goal FetchPages(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      emit page;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "live";
    scheduler.max_inflight <= 2;
  }
}
CONCLAVE

# Lower
./target/release/conclave lower hello.conclave --url-count 2 --output plan_ir.json

# Write manifest
cat > manifest.json << EOF
{
  "conclave_manifest_version": "0.1",
  "program": { "name": "hello_world", "plan_ir_hash": "" },
  "target": { "triple": "aarch64-apple-darwin", "os": "macos", "arch": "aarch64" },
  "toolchain": {
    "lowerer_hash": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    "runtime_hash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    "stdlib_hash":  "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  },
  "capability_bindings": {
    "fetch(Url)->Html": {
      "capability_name": "fetch",
      "artifact_hash": "$FETCH_HASH",
      "determinism_profile": "replayable",
      "trust": "sandboxed_network_only",
      "config": {},
      "signatures": null
    }
  },
  "scheduler_policy": {
    "strategy": "bounded_parallel_map",
    "max_inflight": 2,
    "ready_queue_order": ["url_index", "node_kind", "node_id"],
    "node_kind_order": ["FETCH"],
    "tie_breaker": { "kind": "stable", "seed": 0 }
  },
  "determinism": {
    "mode": "live",
    "clock": "virtual",
    "randomness": { "allowed": true, "seed": 1337, "source": "ctr_drbg" },
    "float": "strict",
    "io_policy": {
      "network": "sandboxed",
      "filesystem": "sandboxed",
      "env": "frozen"
    }
  },
  "observability": {
    "trace_level": "deterministic",
    "emit_scheduler_trace": true,
    "emit_capability_metrics": true
  },
  "supply_chain": {
    "artifact_store": "content_addressed",
    "require_artifact_signatures": false,
    "manifest_signature": null
  }
}
EOF

# Seal
./target/release/conclave seal \
  --plan plan_ir.json \
  --manifest manifest.json \
  --output sealed.json

# Pack
./target/release/conclave pack \
  --runtime ./target/release/conclave \
  --plan plan_ir.json \
  --manifest sealed.json \
  --output hello.cnclv

# Run
./target/release/conclave run hello.cnclv \
  --urls "https://example.com,https://anthropic.com" \
  --trace-out trace.json \
  --mode live

echo "Done. Trace written to trace.json."
```

---

## What you get

After a successful run:

| File | Contents |
|---|---|
| `hello.conclave` | Source program |
| `plan_ir.json` | Canonical execution graph |
| `manifest.json` | Manifest template |
| `sealed.json` | Sealed manifest (plan_ir_hash filled in) |
| `hello.cnclv` | Self-contained runnable artifact |
| `trace.json` | Deterministic scheduler trace |

The `trace_hash` printed to stderr is a SHA256 over the trace events. Run the artifact again with the same URLs in the same order and the trace hash will match — that's the determinism guarantee.

---

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `seal failed: missing capability binding` | A capability in the `.conclave` file has no entry in `capability_bindings` | Add the binding; key must be the normalized signature |
| `seal failed: network capability must be configured as replay-only` | `determinism.mode == "sealed_replay"` but fetch binding has no `"fetch_mode": "replay"` | Add `"config": {"fetch_mode": "replay"}` to the binding, or use `"mode": "live"` |
| `runtime error: ERR_CAPABILITY_MISSING` | Capability binary not found in the cap store | Re-run `install-cap`; check you're not running from a different user |
| `runtime error: ERR_REPLAY_MISS` | Running in `sealed_replay` mode without a replay store | Pass `--replay <store.json>` or run with `--mode live` |
| `lowering failed` | Syntax error in the `.conclave` file | Check `version 0.1;` is the first line; all capabilities are declared before use |
