# Conclave Build Manifest Specification (v0.1)

The **Build Manifest** is the sealed, auditable contract that makes a build reproducible.

A sealed artifact MUST be a pure function of:

- `plan_ir_hash`
- manifest fields (including capability hashes)
- target + toolchain hashes
- deterministic lowering settings

If any required field is missing or unpinned, **Seal MUST fail**.

---

## 1. Manifest schema (v0.1)

```json
{
  "conclave_manifest_version": "0.1",

  "program": {
    "name": "string",
    "plan_ir_hash": "sha256:..."
  },

  "target": {
    "triple": "x86_64-unknown-linux-gnu",
    "os": "linux",
    "arch": "x86_64"
  },

  "toolchain": {
    "lowerer_hash": "sha256:...",
    "runtime_hash": "sha256:...",
    "stdlib_hash": "sha256:..."
  },

  "capability_bindings": {
    "fetch(Url)->Html": {
      "capability_name": "fetch",
      "artifact_hash": "sha256:...",
      "determinism_profile": "replayable",
      "trust": "sandboxed_network_only",
      "config": {
        "fetch_mode": "replay",
        "replay_store_hash": "sha256:..."
      },
      "signatures": {
        "required": true,
        "accepted_keys": ["kid:..."]
      }
    },

    "extract_text(Html)->String": {
      "capability_name": "extract_text",
      "artifact_hash": "sha256:...",
      "determinism_profile": "fixed",
      "trust": "deterministic"
    },

    "summarize(String)->Summary": {
      "capability_name": "summarize",
      "artifact_hash": "sha256:...",
      "determinism_profile": "fixed",
      "trust": "deterministic_when_seeded",
      "config": {
        "max_tokens": 256,
        "decoding": "greedy",
        "seed": 1337
      }
    }
  },

  "scheduler_policy": {
    "strategy": "bounded_parallel_map",
    "max_inflight": 2,
    "ready_queue_order": ["url_index", "node_kind", "node_id"],
    "node_kind_order": ["FETCH", "EXTRACT", "SUMMARIZE", "ASSEMBLE"],
    "tie_breaker": { "kind": "stable", "seed": 0 }
  },

  "determinism": {
    "mode": "sealed_replay",
    "clock": "virtual",
    "randomness": { "allowed": true, "seed": 1337, "source": "ctr_drbg" },
    "float": "strict",
    "io_policy": {
      "network": "replay_only",
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
    "require_artifact_signatures": true,
    "manifest_signature": {
      "algo": "ed25519",
      "public_key_id": "kid:...",
      "signature": "base64:..."
    }
  }
}
```

---

## 2. Seal rules

### 2.1 MUST

- `program.plan_ir_hash` present and valid.
- Every capability referenced by Plan IR MUST have a binding with an `artifact_hash`.
- `toolchain.lowerer_hash` and `toolchain.runtime_hash` MUST be pinned.
- If `determinism.mode == sealed_replay`, any capability with network trust MUST be configured to replay-only.
- Deterministic tracing MUST NOT depend on wall-clock time.

### 2.2 SHOULD

- Manifest includes a signature.
- Capability artifacts are signed and verified.

### 2.3 MUST NOT

- No “latest”, no semver ranges, no mutable references.
- No host environment leakage in strict modes.

---

## 3. Binary identity

Define:

- `artifact_hash = sha256(binary_bytes)`

Optionally:

- `artifact_id = sha256(plan_ir_hash || canonical_manifest_hash || toolchain_hashes)`

---

## 4. Canonical manifest hashing

To compute `canonical_manifest_hash`:

- Sort keys lexicographically
- Normalize numerics and arrays
- UTF-8 JSON, no insignificant whitespace
- Exclude `manifest_signature.signature` (signature does not sign itself)
