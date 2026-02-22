# Agent Guide: Writing and Running Conclave Programs

This guide is for AI agents that want to generate and execute Conclave programs. It covers everything you need — from writing valid source syntax to running a sealed artifact — without requiring prior knowledge of Conclave internals.

---

## What Conclave does

Conclave lets you define a *goal* (what you want to compute) using a simple declarative language. Conclave then:

1. Compiles your program to a deterministic execution graph (Plan IR)
2. Seals the graph against specific capability implementations (pinned by SHA-256 hash)
3. Runs the sealed artifact deterministically — same inputs, same trace, every time

The key guarantee: **the same sealed artifact + the same inputs → identical output and execution trace**.

---

## The source language

A Conclave program is a text file with extension `.conclave`. Here is the canonical example:

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

### Syntax rules you must follow

**Version line (required, must be first):**
```conclave
version 0.1;
```

**Type declarations (optional):**
```conclave
type TypeName = BaseType;
type TypeName = BaseType where validator("pattern");
```
- Base types: `String`, `Html`, `Json`, or any identifier
- Validator: `re2("pattern")` for regex validation

**Capability declarations (external calls — touch the network):**
```conclave
capability alias: fn_name(ArgType) -> ReturnType;
```
- `alias` is the name you use in the `want` block
- `fn_name` is the function name in the signature
- Multiple capabilities: declare one per line

**Intrinsic declarations (pure built-ins — no side effects):**
```conclave
intrinsic alias: fn_name(ArgType) -> ReturnType;
```

**Goal declaration:**
```conclave
goal GoalName(param: ParamType) -> ReturnType {
  want {
    // statements
  }
  constraints {
    // constraint expressions
  }
}
```

### Want block statements

| Statement | Syntax | Meaning |
|---|---|---|
| Let | `let x = fn(arg);` | Call `fn` with `arg`, bind result to `x` |
| Map | `map list as item { ... }` | Expand body once per item in `list` |
| Emit | `emit fn(arg);` | Call `fn`, collect result into `collected` |
| Emit (ident) | `emit x;` | Collect a previously bound value into `collected` |
| Return | `return fn(collected);` | Produce the goal's output (must be last) |

**Rules:**
- `return` must be the last statement in the `want` block
- `collected` is a special identifier — it refers to the list of all `emit` results
- Inside a `map`, the binder (`url` in `map urls as url`) refers to the current element
- Binding names cannot be reused in the same scope (no shadowing)
- Calls can only use capabilities or intrinsics that are declared at the top of the file

### Constraint expressions

```conclave
constraints {
  determinism.mode == "sealed_replay";   // required
  rate_limit(fetch) <= 2 req/s;         // optional
  scheduler.max_inflight <= 2;          // optional
}
```

- `determinism.mode` must be `"sealed_replay"` or `"live"`
- `rate_limit(capability_alias) <= N req/s` limits calls per second
- `scheduler.max_inflight <= N` limits concurrent operations

---

## Step-by-step: writing a valid program

### Step 1: Decide what capabilities you need

Each capability is a separate binary that implements a JSON stdio protocol. In this guide, we use the provided `conclave-cap-fetch` binary and the Python `cap_fetch.py` example.

Available built-in intrinsics you can always use:
- `assemble_json(List<String>) -> Json` — combines a list of strings into JSON

### Step 2: Declare your types and capabilities

Declare every type, capability, and intrinsic you will use. Order doesn't matter (they get sorted during compilation), but each must be declared before use.

```conclave
version 0.1;

capability my_cap: my_fn(InputType) -> OutputType;
intrinsic assemble_json: assemble_json(List<OutputType>) -> Json;
```

### Step 3: Write your goal

A goal takes inputs and produces output via the `want` block.

**Pattern: process a list, collect results, return JSON**

```conclave
goal MyGoal(items: List<InputType>) -> Json {
  want {
    map items as item {
      let result = my_cap(item);
      emit result;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
  }
}
```

**Pattern: multi-step pipeline per item**

```conclave
goal Pipeline(items: List<Url>) -> Json {
  want {
    map items as item {
      let a = step1(item);     // a is a bound name
      let b = step2(a);        // b depends on a
      emit step3(b);           // collected gets b's result
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
    scheduler.max_inflight <= 3;
  }
}
```

### Step 4: Common mistakes to avoid

❌ **Using a name that wasn't declared:**
```conclave
// Wrong: undefined_cap not declared
let x = undefined_cap(url);
```

❌ **Reusing a name in the same scope:**
```conclave
// Wrong: 'html' is already bound
let html = fetch(url);
let html = extract_text(html);  // error: shadowed binding
```

❌ **Return before all emits:**
```conclave
// Wrong: return must be last
return assemble_json(collected);
emit summarize(text);
```

❌ **Missing version line:**
```conclave
// Wrong: version must be first
capability fetch: fetch(Url) -> Html;
version 0.1;
```

❌ **Wrong version:**
```conclave
// Wrong: only version 0.1 is supported
version 0.2;
```

✓ **Correct structure:**
```conclave
version 0.1;
// declarations...
goal MyGoal(...) -> ... {
  want {
    map items as item {
      let x = cap(item);
      emit x;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
  }
}
```

---

## Step-by-step: running a program

### Prerequisites

Build the CLI:
```bash
cargo build --release
# Binary: ./target/release/conclave
```

Install a capability:
```bash
# Rust fetch binary
cargo build --release -p conclave-cap-fetch
conclave install-cap ./target/release/conclave-cap-fetch
# → installed: sha256:<HASH>
# Note: save this hash — you need it in the manifest

# Or: Python fetch capability
conclave install-cap examples/cap_fetch.py
# → installed: sha256:<HASH>
```

### Step 1: Write your `.conclave` file

Save it as e.g. `my_program.conclave`.

### Step 2: Lower to Plan IR

```bash
conclave lower my_program.conclave --url-count 3 --output plan_ir.json
```

- `--url-count` is the number of URLs (or items) to process. This must match the number of items you'll pass at runtime.
- This produces `plan_ir.json` and prints `source_hash`, `ast_hash`, `plan_ir_hash` to stderr.

### Step 3: Create a manifest

Create `manifest.json`:

```json
{
  "conclave_manifest_version": "0.1",
  "program": { "name": "my_program", "plan_ir_hash": "" },
  "target": { "triple": "aarch64-apple-darwin", "os": "macos", "arch": "aarch64" },
  "toolchain": {
    "lowerer_hash":  "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    "runtime_hash":  "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    "stdlib_hash":   "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  },
  "capability_bindings": {
    "fetch(Url)->Html": {
      "capability_name": "fetch",
      "artifact_hash": "sha256:<HASH_FROM_INSTALL_CAP>",
      "determinism_profile": "replayable",
      "trust": "sandboxed_network_only",
      "config": {},
      "signatures": null
    }
  },
  "scheduler_policy": {
    "strategy": "bounded_parallel_map",
    "max_inflight": 2,
    "ready_queue_order": ["url_index", "node_kind", "node_id"],
    "node_kind_order": ["FETCH"],
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
    "require_artifact_signatures": false,
    "manifest_signature": null
  }
}
```

**Critical:** The key in `capability_bindings` must exactly match the **normalized signature** of the capability. Normalization strips all whitespace. So:
- `fetch(Url) -> Html` → key is `"fetch(Url)->Html"`
- `extract_text(Html) -> String` → key is `"extract_text(Html)->String"`
- `summarize(String) -> String` → key is `"summarize(String)->String"`

If your program uses multiple capabilities, add a binding for each one.

**`io_policy` values** (must use these exact strings):
- `"network"`: `"replay_only"` | `"sandboxed"` | `"open"`
- `"filesystem"`: `"sandboxed"` | `"read_only"` | `"open"`
- `"env"`: `"frozen"` | `"read_only"` | `"open"`

**`node_kind_order`** should list the node kind names used in your program. For a single `fetch` capability: `["FETCH"]`. For the full summarize pipeline: `["FETCH", "EXTRACT_TEXT", "SUMMARIZE", "ASSEMBLE_JSON"]`. These are uppercase versions of the capability function names.

### Step 4: Seal

```bash
conclave seal --plan plan_ir.json --manifest manifest.json --output sealed.json
```

Note: `--plan` and `--manifest` are named flags, not positional arguments.

### Step 5: Pack

```bash
conclave pack \
  --runtime ./target/release/conclave \
  --plan plan_ir.json \
  --manifest sealed.json \
  --output artifact.cnclv
```

Note: `--runtime` is required and points to the conclave CLI binary itself.

### Step 6: Run (live mode — hits the network)

```bash
conclave run artifact.cnclv \
  --cap-store ~/.cache/conclave/caps \
  --urls "https://example.com,https://anthropic.com,https://example.org" \
  --trace-out trace.json \
  --mode live
```

- `--urls` is a comma-separated list of URLs. The count must match `--url-count` from the lower step.
- `--cap-store` is the directory where capability binaries are stored (default: `~/.cache/conclave/caps`)
- `--mode live` fetches real URLs. `--mode sealed_replay` replays cached results.

The run prints the execution trace to `trace.json` and prints `trace_hash` to stderr.

---

## Checking your output

```bash
# Inspect the artifact
conclave inspect artifact.cnclv

# Run again and compare trace hashes
conclave run artifact.cnclv \
  --urls "https://example.com,..." \
  --trace-out trace2.json \
  --mode sealed_replay

# If trace_hash matches trace.json → identical run
```

---

## Full example from scratch

```bash
# 1. Write program
cat > fetch_pages.conclave << 'EOF'
version 0.1;

type Url = String where re2("^https?://");

capability fetch: fetch(Url) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;

goal FetchAndCollect(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      emit page;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
    rate_limit(fetch) <= 2 req/s;
    scheduler.max_inflight <= 2;
  }
}
EOF

# 2. Build tools
cargo build --release 2>/dev/null
cargo build --release -p conclave-cap-fetch 2>/dev/null

# 3. Install capability (save the hash)
HASH=$(./target/release/conclave install-cap ./target/release/conclave-cap-fetch 2>&1 | grep -o 'sha256:[a-f0-9]*')
echo "Capability hash: $HASH"

# 4. Lower (url-count must match how many URLs you'll pass at runtime)
./target/release/conclave lower fetch_pages.conclave --url-count 2 --output plan_ir.json

# 5. Create manifest (substitute $HASH)
cat > manifest.json << EOF
{
  "conclave_manifest_version": "0.1",
  "program": { "name": "fetch_pages", "plan_ir_hash": "" },
  "target": { "triple": "aarch64-apple-darwin", "os": "macos", "arch": "aarch64" },
  "toolchain": {
    "lowerer_hash":  "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    "runtime_hash":  "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    "stdlib_hash":   "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  },
  "capability_bindings": {
    "fetch(Url)->Html": {
      "capability_name": "fetch",
      "artifact_hash": "$HASH",
      "determinism_profile": "replayable",
      "trust": "sandboxed_network_only",
      "config": {},
      "signatures": null
    }
  },
  "scheduler_policy": {
    "strategy": "bounded_parallel_map",
    "max_inflight": 2,
    "ready_queue_order": ["url_index", "node_kind", "node_id"],
    "node_kind_order": ["FETCH"],
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
    "require_artifact_signatures": false,
    "manifest_signature": null
  }
}
EOF

# 6. Seal
./target/release/conclave seal \
  --plan plan_ir.json \
  --manifest manifest.json \
  --output sealed.json

# 7. Pack
./target/release/conclave pack \
  --runtime ./target/release/conclave \
  --plan plan_ir.json \
  --manifest sealed.json \
  --output artifact.cnclv

# 8. Run
./target/release/conclave run artifact.cnclv \
  --urls "https://example.com,https://anthropic.com" \
  --trace-out trace.json \
  --mode live

echo "Done! Trace written to trace.json"
```

---

## Type reference

The following types are recognized in v0.1:

| Type | Meaning |
|---|---|
| `String` | UTF-8 string |
| `Html` | HTML content (string) |
| `Json` | JSON value |
| `List<T>` | List of T |
| `Url` | URL (define with `type Url = String where re2("^https?://")`) |

You can define custom type aliases with optional regex validators.

---

## Troubleshooting

**"capability 'X' has no pinned artifact_hash"**
→ The capability binding key in your manifest doesn't match the normalized signature. Check that there's no whitespace: `"fetch(Url)->Html"` not `"fetch(Url) -> Html"`.

**"version mismatch"**
→ Change `version 0.2` (or whatever you have) to `version 0.1;`

**"unknown capability: X"**
→ You used `X` in the want block but didn't declare `capability X: ...;` at the top.

**"shadowed binding: X"**
→ You used the same name twice in the same scope. Choose a different name.

**TLS error on macOS**
→ Use the Rust capability built with `features = ["native-certs"]` (already configured in `conclave-cap-fetch`).

**"unexpected token at line N"**
→ Check line N in your source file. Common causes: missing `;`, wrong order of statements, `return` not being last.

**seal: "error: the following required arguments were not provided"**
→ Use named flags: `conclave seal --plan plan_ir.json --manifest manifest.json --output sealed.json`

**pack: "error: the following required arguments were not provided"**
→ Use named flags and include `--runtime`: `conclave pack --runtime ./target/release/conclave --plan plan_ir.json --manifest sealed.json --output artifact.cnclv`

---

## Grammar summary (v0.1)

```
module        ::= "version" "0" "." "1" ";"
                  (type_decl | capability_decl | intrinsic_decl)*
                  goal_decl*

type_decl     ::= "type" IDENT "=" type_expr ("where" IDENT "(" STRING ")")? ";"
capability_decl ::= "capability" IDENT ":" signature ";"
intrinsic_decl  ::= "intrinsic" IDENT ":" signature ";"
signature     ::= IDENT "(" type_list ")" "->" type_expr
type_expr     ::= IDENT ("<" type_expr ">")?

goal_decl     ::= "goal" IDENT "(" param_list ")" "->" type_expr "{" goal_body "}"
goal_body     ::= want_block constraints_block?

want_block    ::= "want" "{" stmt* "}"
stmt          ::= let_stmt | map_stmt | emit_stmt | return_stmt
let_stmt      ::= "let" IDENT "=" call_expr ";"
map_stmt      ::= "map" IDENT "as" IDENT "{" stmt* "}"
emit_stmt     ::= "emit" expr ";"
return_stmt   ::= "return" expr ";"

constraints_block ::= "constraints" "{" constraint_stmt* "}"
constraint_stmt   ::= (path "==" STRING | path "<=" NUMBER | fn_call "<=" NUMBER "req/s") ";"
```
