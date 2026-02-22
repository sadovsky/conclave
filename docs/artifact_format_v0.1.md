# Conclave Artifact Format (v0.1)

This document specifies how Conclave v0.1 produces a **single runnable executable** using **Option B**:

> A generic Conclave runtime (VM/interpreter) with a sealed program bundle appended to it deterministically.

The resulting file is a normal OS executable (ELF/Mach-O/PE) and can be invoked directly.

---

## 1. Terms

- **Runtime**: the generic Conclave VM executable (`conclave-runtime`), compiled once per target/toolchain.
- **Bundle**: the sealed program payload containing Build Manifest + Plan IR (+ optional embedded capability blobs).
- **Artifact**: the final runnable program executable: `runtime_bytes || bundle_bytes || trailer`.

---

## 2. High-level layout

```
+-------------------------------+
| runtime_bytes                 |  (native executable)
+-------------------------------+
| bundle_bytes                  |  (canonical payload)
+-------------------------------+
| trailer                       |  (fixed-size footer)
+-------------------------------+
```

Conclave v0.1 uses an **append-only** container so it works cross-platform without needing platform-specific section editing.

---

## 3. Trailer format

The trailer is always **exactly 16 bytes**:

- `bundle_len_u64_le` (8 bytes, unsigned little-endian)
- `magic` (8 bytes ASCII) = `CNCLV01\0`  (7 chars + NUL)

```
offset from EOF:
-16 .. -9   bundle_len_u64_le
 -8 .. -1   magic bytes: 43 4E 43 4C 56 30 31 00
```

### Validation rules
At runtime start:

1. Read last 8 bytes → must match magic.
2. Read preceding 8 bytes → `bundle_len`.
3. Compute `bundle_start = file_size - 16 - bundle_len`.
4. If `bundle_start < 0`, fail with `ERR_ARTIFACT_TRUNCATED`.
5. Read `bundle_bytes` and parse.

---

## 4. Bundle encoding

### 4.1 Canonical encoding requirement

To preserve reproducibility, `bundle_bytes` MUST be encoded canonically.

v0.1 recommendation:
- **CBOR (canonical)** or
- **Canonical JSON** (sorted keys, normalized numbers, no insignificant whitespace)

This spec uses **Canonical JSON** in examples.

### 4.2 Bundle schema (v0.1)

```json
{
  "bundle_version": "0.1",

  "manifest": { ... },          // per docs/manifest_spec.md (canonical form)
  "plan_ir": { ... },           // per docs/plan_ir.md (canonical form)

  "embedded_artifacts": {
    "sha256:...": {
      "kind": "capability",
      "name": "fetch",
      "signature": "fetch(Url)->Html",
      "bytes_encoding": "raw",
      "bytes": "<opaque bytes base64 for JSON transport only>"
    }
  },

  "bundle_hashes": {
    "canonical_manifest_hash": "sha256:...",
    "plan_ir_hash": "sha256:...",
    "bundle_hash": "sha256:..."
  }
}
```

Notes:
- `embedded_artifacts` is OPTIONAL. If omitted, runtime loads capability artifacts from a content-addressed store by `artifact_hash` specified in the manifest.
- If JSON is used, `bytes` must be transported as base64. When hashing, hashing is done over the decoded raw bytes, not base64 text.

---

## 5. Hashing rules

### 5.1 plan_ir_hash and canonical_manifest_hash

- `plan_ir_hash` MUST equal `manifest.program.plan_ir_hash`.
- `canonical_manifest_hash` computed per docs/manifest_spec.md.

### 5.2 bundle_hash

`bundle_hash = sha256(canonical_bundle_json_without_bundle_hashes.bundle_hash)`

More precisely:

1. Remove `bundle_hashes.bundle_hash` field.
2. Canonicalize bundle JSON.
3. Hash the UTF-8 bytes.
4. Insert the resulting hash as `bundle_hashes.bundle_hash`.

This makes the bundle self-identifying without recursive ambiguity.

---

## 6. Execution startup sequence (normative)

A v0.1 runtime MUST:

1. Validate trailer magic and locate bundle.
2. Parse bundle (canonical JSON/CBOR).
3. Verify:
   - `bundle.bundle_version` supported
   - `bundle.bundle_hashes.plan_ir_hash == manifest.program.plan_ir_hash`
   - recompute `canonical_manifest_hash` and match
   - recompute `plan_ir_hash` from canonical Plan IR and match
4. Load capability artifacts:
   - if `embedded_artifacts` contains `artifact_hash`, prefer embedded bytes
   - else fetch from content-addressed cache/store by hash
5. Enforce manifest IO policy prior to execution.
6. Execute deterministic scheduler and Plan IR.
7. Optionally emit deterministic trace per manifest.

Any verification failure MUST be a deterministic error with a stable error code.

---

## 7. Determinism constraints for packing

The `conclave pack` step MUST be deterministic given:

- runtime binary bytes
- canonical manifest
- canonical Plan IR
- embedded artifacts (raw bytes)
- packer implementation version

To ensure bit-for-bit reproducibility:

- Packer MUST NOT embed timestamps, hostnames, random IDs.
- Packer MUST write trailer and bundle exactly as specified.
- If compression is used in v0.2+, it MUST be deterministic and pinned (algorithm+level).

---

## 8. CLI flow (recommended)

### 8.1 Build flow overview

```
conclave plan   program.conclave   -> plan_ir.json
conclave seal   program.conclave   -> manifest.json
conclave pack   --runtime conclave-runtime --manifest manifest.json --plan plan_ir.json \
               [--embed-capabilities]      -> artifact (executable)
conclave run    ./artifact input.json      -> output
conclave inspect ./artifact                -> prints hashes, bindings, policies
```

### 8.2 Command responsibilities

#### `conclave plan`
- Parse + normalize source into canonical Plan IR
- Output canonical `plan_ir.json`
- Print `plan_ir_hash`

#### `conclave seal`
- Resolve capability bindings (hashes pinned)
- Validate determinism mode constraints
- Output canonical `manifest.json`
- Print `canonical_manifest_hash`

#### `conclave pack`
- Read runtime bytes
- Read canonical `plan_ir.json` and `manifest.json`
- Optionally embed capability blobs referenced by manifest
- Write `artifact = runtime || bundle || trailer`
- Print `artifact_hash` and `bundle_hash`

---

## 9. Error codes (v0.1 suggested)

- `ERR_ARTIFACT_TRUNCATED`
- `ERR_ARTIFACT_BAD_MAGIC`
- `ERR_BUNDLE_PARSE_FAILED`
- `ERR_BUNDLE_HASH_MISMATCH`
- `ERR_MANIFEST_HASH_MISMATCH`
- `ERR_PLAN_HASH_MISMATCH`
- `ERR_CAPABILITY_MISSING`
- `ERR_CAPABILITY_SIGNATURE_INVALID`

---

## 10. Future evolution

v0.2+ may introduce:
- platform-native sections instead of append-only blobs
- deterministic compression
- multiple bundles (multi-goal)
- incremental updates
- notarization / transparency logs

All changes MUST version the bundle and maintain strict validation.
