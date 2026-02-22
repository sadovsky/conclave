# Conclave Evolution Plan

This document defines the staged evolution of Conclave from design to stable v0.1 and beyond.

Conclave’s core invariant:

> Determinism is never optional.

All evolution must preserve this constraint.

---

# Phase 0 — Workspace Bootstrap

Goal: Compiling Rust workspace with defined crate boundaries.

Tasks:

- Create Rust workspace
- Create crates:
  - conclave-hash
  - conclave-ir
  - conclave-manifest
  - conclave-seal
  - conclave-runtime
  - conclave-pack
  - conclave-cli
- Add CI (fmt, clippy, test)
- Add minimal CLI scaffolding

Deliverable:
- Cleanly compiling project with CI passing

---

# Phase 1 — Deterministic Hashing Foundation (v0.0.1)

Goal: Stable canonical serialization + hashing.

Implement:

- Canonical JSON serializer
- Sorted key ordering (BTreeMap)
- Numeric normalization
- SHA256 wrapper
- Stable ID generation helpers

Tests:

- Golden serialization fixtures
- Golden hash fixtures
- Cross-platform reproducibility tests

Deliverable:
- Hash outputs are bit-stable across machines

---

# Phase 2 — Plan IR (v0.0.2)

Goal: Canonical semantic graph representation.

Implement:

- Plan IR structs
- Node and edge representation
- Constraint attachment
- Canonicalization rules
- plan_ir_hash()

CLI:

    conclave plan program.conclave

Outputs:

- plan_ir.json
- plan_ir_hash

Deliverable:
- Identical Plan IR hash across platforms

---

# Phase 3 — Manifest & Seal (v0.0.3)

Goal: Deterministic artifact commitment.

Implement:

- Manifest struct
- Canonical manifest hashing
- Capability binding by hash
- Determinism mode validation
- IO policy enforcement

CLI:

    conclave seal program.conclave

Requirements:

- No floating dependencies
- Seal twice → identical manifest bytes

Deliverable:
- Reproducible Seal stage

---

# Phase 4 — Artifact Packaging (v0.0.4)

Goal: Produce single executable artifact.

Artifact layout:

    runtime_bytes || bundle_bytes || trailer

Trailer (16 bytes):

- 8-byte bundle length (LE)
- 8-byte magic: CNCLV01\0

Bundle contains:

- Canonical Plan IR
- Canonical Manifest
- Optional embedded capabilities
- Bundle hash

Deliverable:
- Bit-for-bit reproducible executable artifacts

---

# Phase 5 — Deterministic Runtime (v0.1)

Goal: Execute Plan IR deterministically.

Runtime constraints:

- Single-threaded
- No wall-clock time
- Deterministic scheduler
- Stable ready-queue sorting
- Virtual clock
- Deterministic token bucket
- Deterministic trace emission
- IO policy enforcement

Minimum built-in capabilities:

- fetch (replay-only)
- extract_text (deterministic stub)
- summarize (deterministic stub)
- assemble_json intrinsic

Deliverable:
- Same artifact + same input → identical output + trace

---

# Phase 6 — Capability System Expansion (v0.2)

Goal: Language-agnostic capabilities.

Add:

- Subprocess-based capability invocation
- Canonical CBOR encoding
- Deterministic context injection
- Capability signature verification (ed25519)
- Content-addressed capability store
- Optional WASM capability support

Deliverable:
- Sandboxed, hash-pinned capability ecosystem

---

# Phase 7 — DSL & Lowering Improvements

Goal: Developer ergonomics.

Add:

- Real DSL parser
- want: lowering to Plan IR
- Constraint builder improvements
- Better error diagnostics
- Plan graph visualization

Deliverable:
- Usable surface language

---

# Phase 8 — Multi-Agent Propose (Optional)

Goal: Agent-aware plan exploration.

Add:

- Multiple candidate plan generation
- Cost model scoring
- Constraint verification
- Deterministic selection during Seal

Important:
- Propose never produces artifacts.
- Seal remains authoritative.

Deliverable:
- Multi-agent exploration without sacrificing determinism

---

# Phase 9 — Advanced Axes (Post v0.2)

Choose carefully:

A) WASM-first capability execution  
B) Distributed deterministic execution  
C) Formal constraint verification (SMT)  
D) Proof-carrying capabilities  
E) Self-optimizing runtime  

Only pursue one axis at a time.

---

# v0.1 Completion Criteria

Conclave v0.1 is complete when:

- Same source + same manifest → identical artifact bytes
- Same artifact + same input → identical output
- Same artifact + same input → identical execution trace
- No hidden nondeterminism remains
- Replay mode deterministic failures are enforced

---

# Long-Term Vision

Conclave aims to become:

- An agent-native programming model
- A deterministic execution protocol
- A capability-bound orchestration system
- A reproducible artifact generator

But v0.1 proves the thesis:

Collective reasoning and strict determinism can coexist.
