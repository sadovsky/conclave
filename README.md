# Conclave

*AI Generated*

Conclave is a deterministic, agent-native workflow language built for a world where AI systems increasingly write and operate software. At its core is a purpose-built DSL that expresses intent as canonical graphs of plans and constraints—removing ambiguity, syntactic noise, and hidden state so both humans and agents can reason about structure precisely. Conclave then separates Plan (canonical structure), Seal (explicit capability version binding), and Build (artifact creation) into a disciplined pipeline that produces cryptographically identifiable, replayable workflow artifacts. The result is AI-powered automation that can evolve creatively during design but remains reproducible, auditable, and stable in execution—ideal for production systems where trust, traceability, and controlled change truly matter.

**Intent-first, deterministic programming model for agentic systems.**

Conclave gives AI agents a substrate where they can be *powerful* and *auditable* at the same time. An agent generates a `.conclave` program. That program is lowered to a Plan IR, sealed into a content-addressed artifact, and run by a deterministic scheduler. Same inputs → identical output and execution trace. Every time.

> **Manifesto (with human written content):** [I Have Built A Small Deterministic Cathedral](blog/00-i-have-built-a-small-deterministic-cathedral.md)

---

## The problem it solves

When an AI agent runs code today, you can't answer:

- Why did it produce *that* output last Tuesday?
- Did this run use the same logic as the last one?
- Can I prove what data was actually accessed?

Conclave makes these questions answerable. The execution plan is hashed. Capability binaries are pinned by content hash. The trace is deterministic. Two runs with the same trace hash *are* identical — byte for byte.

---

## The pipeline

```
Agent writes (or generates) a .conclave program
         ↓
conclave lower → plan_ir.json    (source_hash, ast_hash, plan_ir_hash)
         ↓
conclave seal  → sealed_manifest.json
         ↓
conclave pack  → artifact.cnclv
         ↓
conclave run   → trace.json      (trace_hash)
```

Every step hashes its output. Every hash is a commitment.

---

## Quick start

> **Full walkthrough:** [docs/hello_world.md](docs/hello_world.md) — every command, copy-pasteable, start to finish.

### Build

```bash
cargo build --release
# Binary: ./target/release/conclave
```

### Write a program

`summarize.conclave`:

```conclave
version 0.1;

type Url = String where re2("^https?://");

capability fetch: fetch(Url) -> Html;
capability extract_text: extract_text(Html) -> String;
capability summarize: summarize(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;

goal SummarizeUrls(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let html = fetch(url);
      let text = extract_text(html);
      emit summarize(text);
    }
    return assemble_json(collected);
  }

  constraints {
    determinism.mode == "sealed_replay";
    rate_limit(fetch) <= 2 req/s;
    scheduler.max_inflight <= 2;
  }
}
```

### Lower to Plan IR

```bash
conclave lower summarize.conclave --url-count 3 --output plan_ir.json
# stderr: source_hash, ast_hash, plan_ir_hash
```

### Seal, pack, run

```bash
# Install a capability binary (gets a sha256 content-address)
conclave install-cap ./target/release/conclave-cap-fetch
# → sha256:<HASH>  ← paste this into your manifest

# Seal: pin capabilities, validate determinism mode
conclave seal --plan plan_ir.json --manifest manifest.json --output sealed.json

# Pack: bundle plan + manifest into a self-contained artifact
conclave pack \
  --runtime ./target/release/conclave \
  --plan plan_ir.json \
  --manifest sealed.json \
  --output artifact.cnclv

# Run
conclave run artifact.cnclv \
  --urls "https://example.com,https://anthropic.com,https://example.org" \
  --trace-out trace.json \
  --mode live
```

---

## Source language

The Conclave v0.1 source language is designed to be easy for agents to generate. Full spec: [docs/source_language_v0.1.md](docs/source_language_v0.1.md).

### Structure

```conclave
version 0.1;

// Optional type refinements
type Url = String where re2("^https?://");

// External calls (touch the network, run as subprocesses)
capability alias: fn_name(ArgType) -> ReturnType;

// Pure built-ins (no side effects)
intrinsic alias: fn_name(ArgType) -> ReturnType;

// A goal: takes inputs, produces output
goal GoalName(param: ParamType) -> ReturnType {
  want {
    // computation graph expressed as data flow
  }
  constraints {
    // scheduler and determinism policies
  }
}
```

### Want block

| Statement | Meaning |
|---|---|
| `let x = fn(arg);` | Call a capability or intrinsic, bind result to `x` |
| `map list as item { ... }` | Expand body once per item in `list` |
| `emit fn(arg);` | Call and collect result into `collected` |
| `return fn(collected);` | Produce the goal's output (must be last) |

### Constraints

```conclave
constraints {
  determinism.mode == "sealed_replay";  // "live" also valid
  rate_limit(fetch) <= 2 req/s;        // max 2 fetch calls/second
  scheduler.max_inflight <= 2;         // max 2 concurrent ops
}
```

---

## Capability protocol (any language works)

Capabilities are separate executables that implement a simple JSON stdio protocol:

**stdin** (one JSON line):
```json
{
  "capability": "fetch(Url)->Html",
  "inputs": { "url": "https://example.com" },
  "context": { "seed": 1337, "virtual_time": 0, "determinism_profile": "replayable" }
}
```

**stdout** (one JSON line):
```json
{ "output": { "type": "Html", "data_b64": "<base64>" }, "duration_ms": 42 }
```

Exit 0 = success. Non-zero = error. Stderr goes to debug trace only.

```bash
# Python example capability included
conclave install-cap examples/cap_fetch.py

# Rust capability
conclave install-cap ./target/release/conclave-cap-fetch
```

---

## Workspace crates

| Crate | Role |
|---|---|
| `conclave-hash` | SHA-256, canonical JSON, stable IDs |
| `conclave-ir` | Plan IR types, validation, canonicalization |
| `conclave-lang` | Source language: lexer, parser, normalizer, lowerer |
| `conclave-manifest` | Manifest types, seal validation rules |
| `conclave-seal` | Sealing pipeline, Ed25519 verification |
| `conclave-store` | Content-addressed capability store |
| `conclave-runtime` | Deterministic scheduler, virtual clock, dispatcher |
| `conclave-pack` | Artifact packing/unpacking |
| `conclave-cli` | `conclave` CLI binary |
| `conclave-cap-fetch` | HTTP fetch capability (Rust, with native TLS) |

---

## CLI reference

```
conclave lower   <source.conclave> [--url-count N] [-o plan_ir.json]
conclave plan    <input>           [--url-count N] [-o plan_ir.json]
conclave seal    --plan <plan_ir.json> --manifest <manifest.json> [-o sealed.json]
conclave pack    --runtime <binary> --plan <plan_ir.json> --manifest <sealed.json> -o artifact.cnclv
conclave run     <artifact.cnclv> [--urls URL,...] [--trace-out trace.json] [--mode live|sealed_replay]
conclave inspect <artifact.cnclv>
conclave install-cap <capability_binary> [--store <dir>]
```

The `plan` command accepts both `.json` (Plan IR) and `.conclave` (source) files.

---

## Running tests

```bash
cargo test --workspace
# 134 tests, 0 failures
```

---

## Key invariants

- **No `HashMap` anywhere** — all maps are `BTreeMap` for deterministic iteration order
- **No wall clock** — the scheduler uses a virtual clock
- **No non-seeded randomness** — seeds are explicit inputs
- **Content-addressed capabilities** — capabilities are identified by `sha256(binary_bytes)`
- **Sealed plans are commitments** — changing anything breaks the hash chain
- **Lowering is pure** — `lower(source, url_count) → PlanIr` with no side effects

---

## Project status

| Phase | What | Status |
|---|---|---|
| 0–5 | Core IR, manifest, seal, pack, runtime, CLI | ✓ Complete |
| 6 | Capability store, subprocess dispatch, Ed25519 | ✓ Complete |
| 7 | Source language (`conclave-lang`) | ✓ Complete |
| 8 | Chained capability inputs, agent-written capabilities | ✓ Complete |
| 9 | Expanded DSL: `if/else`, `reduce`, `pure`, multi-goal | Planned (v0.2) |
| 10 | Goal modules: `import`, subgraph expansion, module registry | Planned (v0.2) |

See `docs/` for specifications and `blog/` for narrative explanations. The v0.2 plan is at [docs/V0.2_PLAN.md](docs/V0.2_PLAN.md).

---

## License

MIT
