# Conclave Rust Architecture Plan (v0.1)

This document defines the recommended Rust workspace structure and implementation strategy for Conclave v0.1.

The goals for v0.1 are:

- Deterministic Plan IR
- Deterministic sealing
- Append-only artifact format
- Deterministic single-threaded runtime
- Strict canonical hashing
- Reproducible binaries

---

## 1. Workspace Layout

Conclave should be implemented as a Rust workspace from the beginning.

```
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
```

Each crate has a narrowly scoped responsibility to preserve determinism and separation of concerns.

---

## 2. Crate Responsibilities

### 2.1 conclave-hash

Purpose: deterministic canonicalization + hashing foundation.

Responsibilities:

- Canonical JSON serializer (sorted keys, stable formatting)
- Numeric normalization rules
- SHA256 wrapper
- Stable ID generation helpers
- Hash utilities for Plan IR and Manifest

Requirements:

- No randomness
- No system time
- No platform-dependent behavior
- Prefer `BTreeMap` over `HashMap`

This crate is the cryptographic root of determinism.

---

### 2.2 conclave-ir

Purpose: Plan IR representation and hashing.

Responsibilities:

- Plan IR Rust structs
- Canonicalization rules
- Stable ID computation
- `plan_ir_hash()` implementation
- Validation of schema constraints

Dependencies:

- serde
- conclave-hash

Must not depend on runtime or sealing logic.

---

### 2.3 conclave-manifest

Purpose: Build Manifest representation and validation.

Responsibilities:

- Manifest Rust structs
- Canonical manifest hashing
- Determinism validation rules
- Capability binding schema
- IO policy enforcement validation (seal-time)

Dependencies:

- conclave-hash
- conclave-ir

No runtime execution logic.

---

### 2.4 conclave-seal

Purpose: Seal phase implementation.

Responsibilities:

- Resolve capability bindings
- Validate determinism mode constraints
- Construct canonical manifest
- Verify Plan IR + manifest consistency
- Emit canonical `manifest.json`

Seal must be deterministic given identical inputs.

---

### 2.5 conclave-runtime

Purpose: Deterministic interpreter for Plan IR.

Responsibilities:

- Deterministic scheduler
- Node lifecycle state machine
- Capability dispatcher
- Policy enforcement (IO, replay, sandboxing)
- Virtual clock implementation
- Deterministic trace emitter

v0.1 constraints:

- Single-threaded only
- No async runtime
- No wall-clock access
- No implicit concurrency
- All time derived from virtual clock

Determinism > performance.

---

### 2.6 conclave-pack

Purpose: Artifact bundling and trailer logic.

Responsibilities:

- Construct bundle structure
- Canonical bundle hashing
- Append-only artifact layout
- Write 16-byte trailer (`bundle_len` + `CNCLV01\0` magic)
- Verify bundle during unpack

Packing must be bit-for-bit reproducible.

---

### 2.7 conclave-cli

Purpose: Thin command interface.

Commands:

```
conclave plan
conclave seal
conclave pack
conclave run
conclave inspect
```

CLI must delegate logic to underlying crates and avoid embedding core logic directly.

---

## 3. Deterministic Scheduler (v0.1 Design)

The scheduler must be fully deterministic.

Constraints:

- Single-threaded
- Stable ready queue sorting
- Deterministic tie-breaking
- Virtual time only

Pseudo-code:

```rust
loop {
    let ready = collect_ready_nodes(&state);
    let mut sorted = sort_ready_nodes(ready, policy);

    for node in sorted {
        if can_dispatch(node, policy, state) {
            dispatch(node);
        }
    }

    if no_running_nodes() {
        break;
    }

    advance_virtual_time();
}
```

No `Instant::now()`.
No wall clock.
No thread races.

---

## 4. Hashing Implementation Rules

Do NOT rely on:

- `HashMap` iteration order
- `serde_json::to_string()` without canonical control
- Floating-point default formatting

Instead:

- Use `BTreeMap`
- Write explicit canonical serializer
- Normalize numeric values
- Ensure stable UTF-8 encoding
- Remove debug-only fields before hashing

---

## 5. Testing Strategy

### 5.1 Golden Hash Tests

Given fixed Plan IR JSON:
- `plan_ir_hash` must match expected value.

### 5.2 Seal Reproducibility

Sealing the same program twice:
- Identical manifest bytes
- Identical canonical manifest hash

### 5.3 Pack Reproducibility

Packing twice with same runtime:
- Identical artifact bytes
- Identical artifact hash

### 5.4 Scheduler Trace Conformance

Given fixed replay data:
- Emitted trace matches golden trace exactly.

---

## 6. Recommended Dependencies (Minimal)

- serde
- serde_json
- sha2
- clap
- thiserror
- base64
- ed25519-dalek (future signature support)

Avoid heavy frameworks or async runtimes in v0.1.

---

## 7. Implementation Order

Strict order to prevent architectural drift:

1. conclave-hash
2. conclave-ir
3. conclave-manifest
4. conclave-pack
5. conclave-runtime
6. conclave-cli

Do not start with runtime before hashing + canonicalization are stable.

---

## 8. v0.1 Success Criteria

Conclave v0.1 is successful if:

- Same source + same manifest ⇒ identical binary bytes
- Same input + replay store ⇒ identical output + trace
- Deterministic scheduler ordering matches conformance spec
- No hidden nondeterminism in runtime or packing

Determinism is the defining property of v0.1.
