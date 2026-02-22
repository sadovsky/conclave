# Testing Guide

This document explains how to run Conclave's tests and what they verify.

---

## Run all tests

```bash
cargo test --workspace
```

Expected output: **133 tests, 0 failures**.

---

## Crate-by-crate breakdown

### `conclave-hash` (6 tests)
Canonical JSON serialization, SHA-256 hashing, stable ID computation.

```bash
cargo test -p conclave-hash
```

### `conclave-ir` (16 tests)
Plan IR types, validation, edge/node/goal ID computation, canonicalization.

```bash
cargo test -p conclave-ir
```

### `conclave-lang` (42 tests)
The source language compiler. Tests are organized by pipeline phase:

```bash
cargo test -p conclave-lang

# Run a specific test file:
cargo test -p conclave-lang --test lexer
cargo test -p conclave-lang --test parse
cargo test -p conclave-lang --test normalize
cargo test -p conclave-lang --test lower
cargo test -p conclave-lang --test hashing
```

Key test categories:

| Test file | What it checks |
|---|---|
| `tests/lexer.rs` | Token stream output for known inputs |
| `tests/parse.rs` | AST structure for the canonical fixture |
| `tests/normalize.rs` | Sort order, signature canonicalization, duplicate detection |
| `tests/lower.rs` | Node/edge counts, entry/exit nodes, constraint structure |
| `tests/hashing.rs` | Hash stability across runs, whitespace invariance |

### `conclave-manifest` (7 tests)
Manifest types, seal rule enforcement.

```bash
cargo test -p conclave-manifest
```

### `conclave-seal` (5 tests)
Sealing pipeline, Ed25519 verification.

```bash
cargo test -p conclave-seal
```

### `conclave-store` (4 tests)
Filesystem and embedded capability stores.

```bash
cargo test -p conclave-store
```

### `conclave-runtime` (9 tests)
Deterministic scheduler, virtual clock, rate limiter, conformance trace.

```bash
cargo test -p conclave-runtime
```

The conformance test verifies the golden scheduler trace:
- F1 and F2 dispatch at t=0ms
- F3 dispatches at t=1000ms (rate limiter: 2 req/s window)
- All nodes complete in the expected order

### `conclave-pack` (4 tests)
Artifact packing and unpacking roundtrip.

```bash
cargo test -p conclave-pack
```

### `conclave-cap-fetch` (5 tests)
The HTTP fetch capability binary. Requires a local test HTTP server (uses `tiny_http`).

```bash
cargo test -p conclave-cap-fetch
```

---

## Golden fixture tests

The source language tests use a golden fixture:

```
crates/conclave-lang/tests/fixtures/summarize_urls/source.conclave
```

This is the canonical `SummarizeUrls` program. The hashing tests verify that:
- The same source always produces the same `source_hash`
- The same source always produces the same `ast_hash`
- The same source always produces the same `plan_ir_hash`
- Whitespace variants (extra blank lines, more spaces) produce the same `ast_hash`

If any of these change, it means the compiler's semantics changed. That requires a version bump per spec §11.

---

## End-to-end CLI test

After building the release binary:

```bash
cargo build --release

# Lower the canonical fixture
./target/release/conclave lower \
  crates/conclave-lang/tests/fixtures/summarize_urls/source.conclave \
  --url-count 3 \
  --output /tmp/test_plan_ir.json

# Expected output on stderr:
# source_hash:  sha256:ff215371...
# ast_hash:     sha256:d90da17d...
# plan_ir_hash: sha256:0186237c...

# Verify Plan IR validates
./target/release/conclave plan /tmp/test_plan_ir.json

# Or use the .conclave file directly with plan
./target/release/conclave plan \
  crates/conclave-lang/tests/fixtures/summarize_urls/source.conclave \
  --url-count 3
```

---

## Hash stability

Hash stability is a critical property. If you change the source language compiler, normalization rules, or lowering rules, the following hashes from the canonical fixture **will change**:

```
source_hash:  sha256:ff215371551c003df18f9557cca095aeab7fcd2254a7cd67aef07e2d55f41fc5
ast_hash:     sha256:d90da17d75b118030531ab269cc02267f271743709a15da58878d027fd61e271
plan_ir_hash: sha256:0186237ca145375211788ad3671683f81190e1dc59fc9c54e16071275521827f
```

If they change, update these golden values in the documentation and bump the language version.

---

## What each test category proves

| Category | Invariant proven |
|---|---|
| Lexer golden streams | Tokenization is deterministic and correct |
| Parse AST structure | Parser produces expected tree |
| Normalize sort order | Same program in different order → same AST |
| Normalize whitespace | Extra spaces → same AST |
| Lower node count | `url_count=N` → `3N+1` nodes for 3-step pipeline |
| Lower hash stability | `lower(src, n)` twice → same hashes |
| Conformance trace | Scheduler matches golden F1→F2→E1→S1…A sequence |
| Cap-fetch binary | JSON stdio protocol works end-to-end |
