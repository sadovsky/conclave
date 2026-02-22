# Conclave

**Conclave is an intent-first, deterministic programming model designed for agent-generated software.**

It separates exploration from commitment and produces cryptographically reproducible artifacts from canonical semantic graphs.

---

## The Problem

Modern software increasingly involves:

- AI systems generating code
- Automated optimization
- Dynamic synthesis
- Complex dependency graphs
- Supply-chain security risks

Traditional compilation assumes:

    source → compile → link → binary

This model was built for human authors writing static files.

It does not explicitly model:

- Multi-agent proposal
- Deterministic commitment boundaries
- Capability-level cryptographic binding
- Execution-time nondeterminism control

Conclave experiments with a different abstraction.

---

## Core Model

Conclave introduces a structured pipeline:

    source → Plan IR → Seal → runtime + bundle → reproducible artifact

### 1. Plan IR

Source lowers into a canonical semantic graph:

- Graph-based representation
- Constraint-attached nodes
- Canonically serialized
- Cryptographically hashed

    plan_ir_hash = sha256(canonical_plan_ir_json)

Formatting changes do not affect identity.

### 2. Seal

Seal is the deterministic commitment phase.

It:

- Pins all capabilities by cryptographic hash
- Produces a canonical Build Manifest
- Enforces determinism mode
- Produces a reproducible artifact

Only Seal produces artifacts.

### 3. Deterministic Runtime

The runtime:

- Executes Plan IR as a graph
- Uses deterministic scheduling
- Uses a virtual clock
- Enforces IO policy
- Emits deterministic execution traces

Given the same Plan IR, manifest, and input, you get the same output, the same trace, and the same artifact hash — every time, on every machine.

---

## Capabilities

Conclave replaces version-based dependencies with hash-bound capabilities:

    fetch(Url)->Html → sha256:aa12...

There are:

- No floating versions
- No implicit upgrades
- No mutable linking

Capabilities are content-addressed and optionally signed.

---

## Artifact Format

Conclave produces a single executable:

    runtime_binary || bundle || trailer

The bundle contains:

- Canonical Plan IR
- Canonical Build Manifest
- Optional embedded capability artifacts
- Bundle hash

The trailer is exactly 16 bytes:

- Bundle length (u64 little-endian)
- Magic `CNCLV01\0`

Artifacts are self-contained and reproducible bit-for-bit.

---

## Design Principles

- Determinism first
- Canonical hashing everywhere
- Content-addressed capability binding
- Strict separation between proposal and commitment
- No hidden nondeterminism
- No floating dependencies

---

## Implementation

Conclave is implemented in Rust as a multi-crate workspace:

    conclave/
    ├── crates/
    │   ├── conclave-hash       # canonical JSON, SHA256, stable IDs
    │   ├── conclave-ir         # Plan IR structs, canonicalization, hashing
    │   ├── conclave-manifest   # manifest structs, seal rules
    │   ├── conclave-seal       # seal phase (commitment gate)
    │   ├── conclave-runtime    # deterministic scheduler, virtual clock, trace
    │   ├── conclave-pack       # artifact bundling and trailer
    │   └── conclave-cli        # conclave plan|seal|pack|run|inspect
    ├── docs/                   # specifications
    ├── examples/               # example Plan IR graphs
    └── tests/conformance/      # golden scheduler trace spec

The runtime is single-threaded, virtual-time driven, and policy-enforced. Performance is secondary to reproducibility.

---

## CLI

```
conclave plan    <plan_ir.json>                         # canonicalize + hash Plan IR
conclave seal    --plan <plan_ir.json> \
                 --manifest <manifest.json>             # pin capabilities, emit manifest
conclave pack    --runtime <binary> \
                 --manifest <manifest.json> \
                 --plan <plan_ir.json> -o <artifact>    # produce executable artifact
conclave run     <artifact> [--replay store.json]       # execute deterministically
conclave inspect <artifact>                             # print hashes, bindings, policies
```

---

## Quick Start

```bash
cargo build -p conclave-cli

# Canonicalize a Plan IR and compute its hash
./target/debug/conclave plan examples/plan_ir.json

# Run all tests including the conformance golden trace
cargo test --workspace
```

---

## Current Status

**v0.1 — Complete.**

All v0.1 completion criteria are met (59 tests, 0 failures):

- Same source + same manifest → identical artifact bytes ✓
- Same artifact + same input → identical output ✓
- Same artifact + same input → identical execution trace ✓
- No hidden nondeterminism ✓
- Replay mode deterministic failures enforced ✓

The conformance golden scheduler trace (`F1→F2→E1→S1→E2→S2→F3→E3→S3→A`, rate window enforced at virtual t=1000ms) passes exactly.

**In progress: Phase 6 — Capability System Expansion.**

Real subprocess-based capability invocation, content-addressed capability store, and ed25519 signature verification.

See [docs/EVOLUTION_PLAN.md](docs/EVOLUTION_PLAN.md) for the full roadmap.

---

## Why Conclave?

As AI systems increasingly participate in building software, we need:

- Explicit commitment boundaries
- Cryptographically verifiable artifacts
- Deterministic execution
- Capability-based binding
- Controlled nondeterminism

Conclave explores that design space.

---

## Non-Goals (v0.1)

- Multi-threaded runtime
- JIT compilation
- Dynamic linking
- Floating dependency resolution
- Performance-first design
- Distributed execution

These may come later, but not before determinism is proven.

---

## License

TBD

---

## Contributing

Read [docs/EVOLUTION_PLAN.md](docs/EVOLUTION_PLAN.md) and [CLAUDE.md](CLAUDE.md).

All contributions must preserve determinism.
