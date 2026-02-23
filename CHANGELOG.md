# Changelog

## [Unreleased — v0.2]

Full spec: [docs/V0.2_PLAN.md](docs/V0.2_PLAN.md)

### Planned

#### Expanded DSL
- `if/else` conditional branching in `want` blocks
- `reduce`/`fold` over lists (sequential, deterministic left-fold)
- `pure` inline compute blocks (intrinsics only, no subprocess)
- Multiple goals per `.conclave` file; `--goal NAME` selector in `conclave lower`

#### Goal Modules
- `import Name: sha256:<hash>;` top-level declaration
- Lowerer resolves and expands imported goals as typed subgraphs in the parent Plan IR
- `subgraph_id` field on Plan IR nodes for attribution and auditing
- `module_bindings` in manifest — transitive capability requirements from imports enforced at seal time
- `conclave module` CLI subcommands: `publish`, `install`, `list`, `inspect`
- Module cache: `~/Library/Caches/conclave/modules/` (macOS), `~/.cache/conclave/modules/` (Linux)

---

## [0.1.0] — 2026-02-22

Initial release of Conclave v0.1.

### What v0.1 delivers

Conclave v0.1 proves the thesis: **same source + same manifest → identical artifact bytes; same artifact + same input → identical output and execution trace.**

---

### Core guarantees

- **Deterministic hashing** — Canonical JSON serialization with sorted keys and integer normalization. SHA256 over canonical bytes. Stable node and edge IDs.
- **Deterministic Plan IR** — Typed node/edge graph, canonicalized before hashing. `plan_ir_hash` is stable across machines.
- **Deterministic Seal** — Pins all capability artifact hashes, toolchain hashes, and determinism mode. Seal twice → identical manifest bytes.
- **Deterministic Pack** — Single executable artifact: `runtime_bytes || bundle_bytes || trailer`. Bit-for-bit reproducible.
- **Deterministic Runtime** — Single-threaded scheduler with virtual clock, deterministic ready-queue ordering, rate limiting via token bucket, IO policy enforcement, and deterministic trace emission.

---

### Components

| Crate | Description |
|---|---|
| `conclave-hash` | Canonical JSON serializer, SHA256, stable ID generation |
| `conclave-ir` | Plan IR structs, canonicalization, hashing |
| `conclave-manifest` | Manifest structs, canonical hashing, seal validation rules |
| `conclave-seal` | Seal phase: commitment gate, ed25519 signature verification |
| `conclave-store` | `CapabilityStore` trait, `FilesystemStore`, `EmbeddedStore`, `ChainedStore` |
| `conclave-runtime` | Deterministic scheduler, `CapabilityDispatcher`, virtual clock, trace emitter |
| `conclave-pack` | Bundle + artifact layout |
| `conclave-lang` | DSL: lexer → parser → normalize → lower |
| `conclave-cli` | `lower`, `plan`, `seal`, `pack`, `run`, `inspect`, `install-cap` |
| `conclave-cap-fetch` | Real HTTP fetch capability binary (Rust, content-addressed) |

---

### CLI commands

```
conclave lower    <source.conclave>          Lower DSL to Plan IR JSON
conclave plan     <plan.json|source.conclave> Canonicalize and hash a Plan IR
conclave seal     --plan --manifest --output  Pin capabilities, validate, emit manifest
conclave pack     --runtime --plan --manifest --output  Pack into runnable artifact
conclave run      <artifact>                  Execute a sealed artifact
conclave inspect  <artifact>                  Print hashes, bindings, and policies
conclave install-cap <binary>                 Install a capability into the content-addressed store
```

---

### Capability ABI

Capabilities are content-addressed executables (any language) invoked as subprocesses. Protocol: one JSON line on stdin, one JSON line on stdout, exit 0 on success.

```json
// stdin
{"capability": "fetch(Url)->Html", "inputs": {"url": "https://example.com"}, "context": {"seed": 0, "virtual_time": 0, "determinism_profile": "replayable"}}

// stdout
{"output": {"type": "Html", "data_b64": "<base64>"}, "duration_ms": 42}
```

Included capabilities:
- `cap_fetch.py` — Python HTTP fetch (replay-safe)
- `cap_word_count.py` — Python HTML word counter
- `conclave-cap-fetch` — Rust HTTP fetch binary

---

### v0.1 completion criteria (all met)

- [x] Same source + same manifest → identical artifact bytes
- [x] Same artifact + same input → identical output
- [x] Same artifact + same input → identical execution trace
- [x] No hidden nondeterminism (no `HashMap`, no `SystemTime`, no unseeded randomness)
- [x] `sealed_replay` mode enforces deterministic failures on replay miss
- [x] Multi-step capability pipelines: upstream outputs thread to downstream inputs

---

### Known limitations (v0.2 scope)

- Runtime is single-threaded; no parallel execution within an artifact
- No WASM capability support yet
- No distributed execution
- DSL `rate_limit(fetch) <= N req/s` constraint is parsed but the rate is hardcoded to 2 req/s in the scheduler; per-capability rate configuration is not yet wired
