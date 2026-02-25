# Conclave

*Human Generated*

Conclave is a deterministic, agent-native workflow language built for a world where AI systems increasingly write and operate software. At its core is a purpose-built DSL that expresses intent as graphs of plans and constraints: removing ambiguity, syntactic noise, and hidden state so both humans and agents can reason about structure (and code review) precisely. Conclave then separates Plan (structure), Seal (explicit capability version binding), and Build (artifact creation) into a pipeline that produces cryptographically identifiable, replayable workflow artifacts. The result is AI-powered software that can evolve creatively during design but remains reproducible, auditable, and stable in execution: ideal for production systems where trust, traceability, and controlled change are important.

**Intent-first, deterministic programming model for agentic systems.**

*AI Generated*

Conclave gives AI agents a substrate where they can be *powerful* and *auditable* at the same time. An agent generates a `.conclave` program. That program is lowered to a Plan IR, sealed into a content-addressed artifact, and run by a deterministic scheduler. Same inputs → identical output and execution trace. Every time.

> **Manifesto (with human written content):** [I Have Built A Small Deterministic Cathedral](blog/00-i-have-built-a-small-deterministic-cathedral.md)

---

## The problem it solves

When an AI agent runs code today, you can't answer:

- Why did it produce *that* output last Tuesday?
- Did this run use the same logic as the last one?
- Can I prove what data was actually accessed?
- Which agent or module contributed which part of the computation?

These questions get harder — not easier — as you add more agents. A pipeline with five agents making decisions is five points of hidden state, five places where nondeterminism can enter, five things you can't audit after the fact.

Conclave makes these questions answerable. The execution plan is hashed. Capability binaries are pinned by content hash. The trace is deterministic. Two runs with the same trace hash *are* identical — byte for byte. And every node in the plan carries a `subgraph_id` that traces it back to its origin: which goal, which imported module, which map expansion produced it.

---

## Multi-agent collaboration

Conclave is designed from the ground up for workflows where multiple agents contribute to a shared plan.

**Plans are structured coordination artifacts.** The Plan IR is canonical JSON — not a blob of code, not a prompt, not a shell script. It has typed nodes, explicit edges, stable content-addressed IDs, and a subgraph structure that makes each agent's contribution independently identifiable. Agents (and humans) can read, review, and reason about plan structure before anything is sealed or executed.

**Goals compose by hash.** An agent can publish a goal to the local module cache and get back a content hash. Another agent imports that goal by hash:

```conclave
import FetchAndExtract: sha256:a3f1...;
```

The lowerer expands the import as a typed subgraph in the parent Plan IR. Every node contributed by the imported goal carries the `subgraph_id` of its origin. You know, at the trace level, which agent's goal ran which node.

**Seal validates the full import graph.** When a plan with imports is sealed, the sealer walks all transitive capability uses — including those from imported subgraphs — and requires a capability binding for every one. An agent cannot silently add a new capability through an import. The manifest is the complete, auditable record of what the composed plan is allowed to do.

**Capabilities are agent-written and content-addressed.** Any agent can write a capability in any language (Python, Rust, shell) and install it into the content-addressed store. The capability gets a `sha256` identity. The manifest pins that identity. Swapping the implementation breaks the seal. This is how you get agent-contributed code that is trusted, versioned, and auditable rather than anonymous subprocess calls.

---

## The pipeline

```
One or more agents write (or generate) .conclave goals
         ↓
conclave lower  → plan_ir.json    (source_hash, ast_hash, plan_ir_hash)
         ↓         [subgraphs for map, reduce, if/else, imports — each attributed]
conclave seal   → sealed_manifest.json
         ↓         [pins capability hashes, validates determinism, checks imports]
conclave pack   → artifact.cnclv
         ↓         [self-contained: plan + manifest + optional embedded caps]
conclave run    → trace.json      (trace_hash)
                   [deterministic dispatch order; virtual clock; rate limits enforced]
```

Every step hashes its output. Every hash is a commitment. The plan IR is the shared design artifact — inspect it, review it, diff it — before sealing locks it in.

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
# The plan IR is canonical JSON — review it before sealing
```

### Seal, pack, run

```bash
# Install a capability binary (gets a sha256 content-address)
conclave install-cap ./target/release/conclave-cap-fetch
# → sha256:<HASH>  ← paste this into your manifest

# Seal: pin capabilities, validate determinism mode, check all imports
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

## Source language (v0.2)

The Conclave source language is designed to be easy for agents to generate and for humans to review. Full spec: [docs/source_language_v0.1.md](docs/source_language_v0.1.md). v0.2 plan: [docs/V0.2_PLAN.md](docs/V0.2_PLAN.md).

### Structure

```conclave
version 0.1;

// Optional type refinements
type Url = String where re2("^https?://");

// Import another goal by content hash (expands as a typed subgraph)
import FetchAndExtract: sha256:a3f1...;

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

Multiple goals per file are supported. Each lowers independently.

### Want block (v0.2)

| Statement | Meaning |
|---|---|
| `let x = fn(arg);` | Call a capability or intrinsic, bind result to `x` |
| `map list as item { ... }` | Expand body once per item — creates a map subgraph |
| `reduce list as item into acc { ... }` | Sequential left-fold over list — creates a reduce subgraph |
| `if cond_fn(x) { ... } else { ... }` | Conditional branch — creates true/false subgraphs; non-taken branch is skipped at runtime |
| `pure { expr }` | Inline intrinsic compute — no capability call, no subprocess |
| `emit fn(arg);` | Call and collect result into `collected` |
| `return fn(collected);` | Produce the goal's output (must be last) |

### Subgraphs and attribution

Every structural construct — `map`, `reduce`, `if/else`, imported goals — lowers to a named subgraph in the Plan IR. Each node in the plan carries the `subgraph_id` of its origin. This means:

- The trace tells you which map iteration produced which output
- Conditional branches are labeled; you can see which path was taken
- Imported goals are fully attributed: every node points back to the module hash that contributed it
- `conclave inspect` shows the full subgraph structure before you commit to a run

### Constraints

```conclave
constraints {
  determinism.mode == "sealed_replay";  // "live" also valid
  rate_limit(fetch) <= 2 req/s;        // max 2 fetch calls/second
  scheduler.max_inflight <= 2;         // max 2 concurrent ops
}
```

Constraints are part of the canonical plan hash. Changing a rate limit or inflight cap produces a new plan IR hash and invalidates the existing seal.

---

## Capability protocol (any language works)

Capabilities are separate executables that implement a simple JSON stdio protocol. Any agent can write one.

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
# Python capability (agent-written, any logic)
conclave install-cap examples/cap_fetch.py

# Rust capability (native TLS, high performance)
conclave install-cap ./target/release/conclave-cap-fetch

# The "Conclave in Conclave" capability: lower .conclave files as a capability
conclave install-cap examples/cap_conclave_lower.py
```

Installed capabilities are identified by `sha256(binary_bytes)`. The manifest binds capability signatures to hashes. Swapping the implementation breaks the seal — intentionally.

---

## Goal modules

```bash
# Publish a goal to the local module cache; get back its content hash
conclave module publish fetch_and_extract.conclave
# → sha256:a3f1...

# Import it by hash in another goal file
# import FetchAndExtract: sha256:a3f1...;

# List cached modules
conclave module list

# Install a module from a hash (fetch from remote, future)
conclave module install sha256:a3f1...
```

The module cache lives at `~/Library/Caches/conclave/modules/` (macOS) or `~/.cache/conclave/modules/` (Linux). Each entry is canonical Plan IR JSON, identified by its content hash.

---

## Workspace crates

| Crate | Role |
|---|---|
| `conclave-hash` | SHA-256, canonical JSON, stable IDs |
| `conclave-ir` | Plan IR types, subgraphs, validation, canonicalization |
| `conclave-lang` | Source language: lexer, parser, normalizer, lowerer, module cache |
| `conclave-manifest` | Manifest types, seal validation rules, module bindings |
| `conclave-seal` | Sealing pipeline, Ed25519 verification |
| `conclave-store` | Content-addressed capability store |
| `conclave-runtime` | Deterministic scheduler, virtual clock, conditional dispatch, dispatcher |
| `conclave-pack` | Artifact packing/unpacking |
| `conclave-cli` | `conclave` CLI binary |
| `conclave-cap-fetch` | HTTP fetch capability (Rust, with native TLS) |

---

## CLI reference

```
conclave lower   <source.conclave> [--url-count N] [--goal NAME] [-o plan_ir.json]
conclave plan    <input>           [--url-count N] [-o plan_ir.json]
conclave seal    --plan <plan_ir.json> --manifest <manifest.json> [-o sealed.json]
conclave pack    --runtime <binary> --plan <plan_ir.json> --manifest <sealed.json> -o artifact.cnclv
conclave run     <artifact.cnclv> [--urls URL,...] [--trace-out trace.json] [--mode live|sealed_replay]
conclave inspect <artifact.cnclv>
conclave install-cap <capability_binary> [--store <dir>]
conclave module  publish|list|install
```

The `plan` command accepts both `.json` (Plan IR) and `.conclave` (source) files. `lower --goal NAME` selects a specific goal from a multi-goal file.

---

## Running tests

```bash
cargo test --workspace
# 150+ tests, 0 failures
```

---

## Key invariants

- **No `HashMap` anywhere** — all maps are `BTreeMap` for deterministic iteration order
- **No wall clock** — the scheduler uses a virtual clock
- **No non-seeded randomness** — seeds are explicit inputs
- **Content-addressed capabilities** — capabilities are identified by `sha256(binary_bytes)`
- **Sealed plans are commitments** — changing anything (constraint, import, capability) breaks the hash chain
- **Lowering is pure** — `lower(source, url_count) → PlanIr` with no side effects
- **Subgraphs are attributed** — every node traces back to its origin goal or import
- **Imports are transitive** — seal validates capability bindings across the full import graph

---

## Project status

| Phase | What | Status |
|---|---|---|
| 0–5 | Core IR, manifest, seal, pack, runtime, CLI | ✓ Complete |
| 6 | Capability store, subprocess dispatch, Ed25519 | ✓ Complete |
| 7 | Source language (`conclave-lang`): lexer, parser, lowerer | ✓ Complete |
| 8 | Chained capability inputs, agent-written capabilities | ✓ Complete |
| 9 | Expanded DSL: `if/else`, `reduce`, `pure`, multi-goal | ✓ Complete (v0.2) |
| 10 | Goal modules: `import`, subgraph inlining, module registry | ✓ Complete (v0.2) |
| 11 | Runtime: conditional branch skipping, reduce dispatch, arity checking | ✓ Complete (v0.2) |
| 12 | Conclave in Conclave: lower `.conclave` files as a sealed capability | ✓ Complete (v0.2) |

See `docs/` for specifications and `blog/` for narrative explanations.

---

## License

MIT
