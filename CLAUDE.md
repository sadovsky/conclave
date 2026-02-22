# CLAUDE.md
# Conclave Development Guide for Claude (and Other Agents)

This file defines how AI agents should begin implementing Conclave.

Conclave is an intent-first, deterministic programming model.
Determinism is the primary invariant.
All development decisions must preserve reproducibility.

---

## 1. Primary Objective

Implement Conclave v0.1 with the following guarantees:

- Same source + same manifest ⇒ identical binary bytes
- Same binary + same input ⇒ identical output
- Same binary + same input ⇒ identical execution trace
- No hidden nondeterminism anywhere in the system

---

## 2. Technology Stack

Language: Rust
Architecture: Workspace with isolated crates
Hashing: SHA256 with canonical serialization
Artifact Format: Append-only executable bundle
Runtime: Single-threaded deterministic interpreter

---

## 3. Required Workspace Structure

conclave/
├── Cargo.toml (workspace)
├── crates/
│   ├── conclave-hash/
│   ├── conclave-ir/
│   ├── conclave-manifest/
│   ├── conclave-seal/
│   ├── conclave-runtime/
│   ├── conclave-pack/
│   └── conclave-cli/

Do not collapse crate boundaries.

---

## 4. Development Order (Strict)

1. conclave-hash
2. conclave-ir
3. conclave-manifest
4. conclave-pack
5. conclave-runtime
6. conclave-cli

Do not begin runtime work before hashing is stable.

---

## 5. Determinism Rules

Never introduce:

- SystemTime::now()
- Instant::now()
- Non-seeded randomness
- HashMap iteration
- Floating dependency resolution
- Host-specific serialization
- Embedded timestamps
- Implicit network access

Always use:

- BTreeMap
- Canonical JSON serialization
- Explicit seed injection
- Virtual clock

---

## 6. Plan IR Requirements

Plan IR must:

- Be canonicalized before hashing
- Exclude meta/debug fields from hash
- Produce stable node IDs
- Produce stable edge IDs

plan_ir_hash = sha256(canonical_plan_ir_json)

---

## 7. Seal Requirements

Seal must:

- Pin all capabilities by hash
- Fail on floating versions
- Enforce determinism mode
- Produce identical manifest bytes when re-run
- Be the only phase that produces artifacts

---

## 8. Artifact Format

Executable structure:

runtime_bytes || bundle_bytes || trailer

Trailer (16 bytes):
- 8-byte bundle length (LE)
- 8-byte magic: CNCLV01\0

Bundle must contain:

- Canonical Plan IR
- Canonical Manifest
- Optional embedded capabilities
- Bundle hash

Packing must be bit-for-bit reproducible.

---

## 9. Runtime Constraints

v0.1 runtime must:

- Be single-threaded
- Use deterministic scheduler
- Use virtual clock
- Enforce IO policy
- Emit deterministic trace

No async runtime in v0.1.

---

## 10. Testing Requirements

All contributions must include:

- Golden hash tests
- Golden Plan IR tests
- Golden manifest tests
- Golden artifact equality tests
- Golden scheduler trace tests

CI must fail on nondeterministic drift.

---

## 11. Philosophy

Conclave is not about maximizing performance.

It is about controlled, reproducible intelligence.

Determinism is sacred.
Seal is authoritative.
Capabilities are content-addressed.
