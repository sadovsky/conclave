# From Intent to Artifact: How the Conclave Compiler Works

*A walkthrough of the full pipeline — from a `.conclave` source file to a deterministic, auditable execution artifact.*

*AI Generated*

---

You've seen what Conclave does conceptually. This post is about how it actually works — from the moment you write (or an agent generates) a `.conclave` file, to the moment a run produces a cryptographically-fingerprinted trace.

---

## The program

Here's the canonical Conclave program — the one we use in all our tests and examples:

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

This is the complete program. There's no main function. No imports. No error handling boilerplate. It says: "For each URL, fetch it, extract the text, summarize it. Collect the summaries. Return them as JSON."

That's it. The rest — concurrency, rate limiting, determinism, replay — is handled by the runtime based on the constraints block.

---

## Step 1: Lexing

The first thing the compiler does is tokenize the source. The lexer is hand-written — no parser generators, no regex dependencies. It produces a flat list of tokens, each tagged with its source line.

```
[Version, Number(0), Dot, Number(1), Semicolon,
 Type, Ident("Url"), Equals, Ident("String"), Where, Ident("re2"), LParen, StringLit("^https?://"), RParen, Semicolon,
 Capability, Ident("fetch"), Colon, Ident("fetch"), LParen, Ident("Url"), RParen, Arrow, Ident("Html"), Semicolon,
 ...]
```

A few things worth noting:

- `->` is a single token (not `-` then `>`).
- `req/s` is lexed as a unit token — the lexer recognizes it immediately after a number.
- `//` line comments are skipped completely.
- No Unicode beyond ASCII. No escape sequences in string literals. v0.1 is deliberately minimal.

---

## Step 2: Parsing

The parser is a recursive descent parser — no backtracking, every ambiguity is a parse error. It produces an AST.

The grammar has a strict shape. `version` must come first. Type, capability, and intrinsic declarations come next, in any order. Goal declarations come last. Inside a `want` block, statements are processed in order: `let`, `map`, `emit`, and then exactly one `return` (which must be last).

The AST for the `want` block looks like this:

```json
{
  "stmts": [
    {
      "kind": "Map",
      "list": "urls",
      "binder": "url",
      "body": [
        { "kind": "Let", "name": "html", "expr": { "kind": "Call", "name": "fetch", "args": [{ "kind": "Ident", "name": "url" }] } },
        { "kind": "Let", "name": "text", "expr": { "kind": "Call", "name": "extract_text", "args": [{ "kind": "Ident", "name": "html" }] } },
        { "kind": "Emit", "expr": { "kind": "Call", "name": "summarize", "args": [{ "kind": "Ident", "name": "text" }] } }
      ]
    },
    { "kind": "Return", "expr": { "kind": "Call", "name": "assemble_json", "args": [{ "kind": "Ident", "name": "collected" }] } }
  ]
}
```

---

## Step 3: Normalization

Before anything is hashed, the AST is normalized:

1. **Sort declarations** — types, capabilities, and intrinsics are sorted alphabetically by name. This means two programs that declare the same things in different orders produce the same AST.

2. **Normalize signatures** — all whitespace is stripped from capability signatures. `"fetch( Url ) -> Html"` becomes `"fetch(Url)->Html"`. This makes the canonical form independent of the author's formatting preferences.

3. **Validate** — version must be `0.1`. No duplicate declarations.

The result is a normalized AST. If you give the compiler the same program formatted differently — extra blank lines, different indentation, caps and intrinsics in a different order — you get the same normalized AST.

**The AST hash** is computed from this normalized AST using canonical JSON serialization. Same semantics → same hash.

---

## Step 4: Lowering

Lowering converts the normalized AST into a Plan IR — a directed graph of computation nodes.

The key design decision in v0.1: `map` constructs are expanded at compile time. When you run `conclave lower` with `--url-count 3`, the map body gets expanded three times, once for each URL index.

For our example with `url_count=3`, this produces:

**10 nodes:**
- `fetch` × 3 (url_index 0, 1, 2)
- `extract_text` × 3 (url_index 0, 1, 2)
- `summarize` × 3 (url_index 0, 1, 2)
- `assemble_json` × 1 (the terminal aggregate node)

**9 edges:**
- `fetch[i].output → extract_text[i].in_0` × 3
- `extract_text[i].output → summarize[i].in_0` × 3
- `summarize[i].output → assemble_json.in_i` × 3

Every node ID is content-addressed:

```
node_id = sha256("conclave:v0.1" + "node" + "{goal}.{binder}.{op}.{url_index}")
```

This means the same program always produces the same node IDs — on every machine, across every version of the compiler that implements v0.1.

The **Plan IR hash** is computed from a canonical serialization of the graph, with all `meta` fields stripped. Changing one character in the source changes the Plan IR hash.

---

## Step 5: Sealing

Sealing is the commitment phase. You provide a manifest that pins every capability to its SHA-256 hash:

```json
{
  "capability_bindings": {
    "fetch(Url)->Html": {
      "artifact_hash": "sha256:3e82e6a9..."
    }
  },
  "determinism": {
    "mode": "sealed_replay",
    "io_policy": {
      "network": "replay_only",
      "filesystem": "sandboxed",
      "env": "frozen"
    }
  }
}
```

The sealed manifest is itself hashed. If someone swaps in a different version of the fetch capability — even one that produces the same output — the hash changes and the seal is broken.

---

## Step 6: Packing and running

The artifact bundles the Plan IR and sealed manifest into a single file. The runtime extracts them, dispatches nodes according to the Plan IR graph and scheduler policy, and produces a trace.

The trace records every event: which node was dispatched at what virtual time, how long it took, what it produced. The trace itself is hashed. Two runs that produce the same `trace_hash` were byte-for-byte identical.

---

## The full hash chain

```
source bytes      → source_hash  (sha256 of source)
                  ↓
normalized AST    → ast_hash     (sha256 of canonical AST JSON)
                  ↓
Plan IR           → plan_ir_hash (sha256 of canonical Plan IR JSON)
                  ↓
sealed manifest   → manifest_hash
                  ↓
artifact bundle   → bundle_hash
                  ↓
execution trace   → trace_hash
```

This is the chain. Every step is a commitment. If anything changes — the source, a capability binary, the scheduler policy — the chain breaks at that step, and you know exactly where.

---

## What `conclave lower` prints

```bash
$ conclave lower summarize.conclave --url-count 3 --output plan_ir.json
source_hash:  sha256:ff215371...
ast_hash:     sha256:d90da17d...
plan_ir_hash: sha256:0186237c...
```

These three hashes together tell you:
- `source_hash`: the exact bytes of the source file
- `ast_hash`: the semantics (whitespace-invariant)
- `plan_ir_hash`: the compiled graph (includes the source fingerprint)

The `ast_hash` is the one you care about for "did the logic change?". The `source_hash` is for archival and audit. The `plan_ir_hash` is what the rest of the pipeline operates on.

---

## Why this matters

Most compilation pipelines produce deterministic outputs — give the same compiler the same source and you get the same binary. Conclave does that too, but it goes further:

The *execution* is deterministic. Not just the artifact, but the run. The scheduler's decisions — which nodes to dispatch, in what order, at what virtual time — are deterministic functions of the Plan IR and the inputs.

This means you can ask: "Did run A and run B do the same thing?" and get a yes/no answer without inspecting every log line. You compare trace hashes. If they're equal, the runs were identical. If they differ, you can diff the traces to see exactly where they diverged.

That's the capability Conclave is trying to build: not just reproducible builds, but reproducible *execution*.

---

*Conclave is open source. The source language spec, compiler implementation, and all tests are in the repository.*
