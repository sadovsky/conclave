# Conclave DSL Implementation Plan — v0.2

This document is the implementation plan for the Conclave source language
front end, as specified in `docs/source_language_v0.1.md`.

The DSL lowers deterministically into Plan IR. It is the primary interface
between agent-generated programs and the Conclave execution engine.

---

## New Crate: `conclave-lang`

**Location:** `crates/conclave-lang/`

```
conclave-lang/
  src/
    lib.rs         — pub use, LowerOutput type
    lexer.rs       — tokenizer (hand-written, no regex dependency)
    ast.rs         — AST node types (all BTreeMap, no HashMap)
    parser.rs      — recursive descent parser
    normalize.rs   — normalization pass (sort decls, canonicalize sigs)
    lower.rs       — AST → Plan IR lowering
    error.rs       — LangError enum
  tests/
    lexer.rs       — golden token stream tests
    parse.rs       — golden AST tests (source → expected JSON)
    normalize.rs   — whitespace variant tests (same AST for different formatting)
    lower.rs       — golden Plan IR tests (source → expected plan_ir.json)
    hashing.rs     — source hash and AST hash stability tests
```

**Dependencies:**
- `serde`, `serde_json` (workspace)
- `thiserror` (workspace)
- `conclave-hash` (path)
- `conclave-ir` (path)

No parser generator. Hand-written recursive descent only. This keeps the
dependency surface minimal and the error messages controlled.

---

## Phase 1 — Lexer (`src/lexer.rs`)

### Tokens

```rust
pub enum Token {
    // Keywords
    Version, Type, Capability, Intrinsic, Goal,
    Want, Map, As, Let, Emit, Return, Constraints,
    Where,

    // Punctuation
    LBrace, RBrace, LParen, RParen, LAngle, RAngle,
    Semicolon, Comma, Colon, Equals, Arrow, // ->
    LtEq,  // <=

    // Literals
    Ident(String),
    StringLit(String),
    Number(u64),
    Float(f64),

    // Units
    ReqPerSec,  // "req/s"
}
```

### Lexer Rules
- Skip whitespace and `//`-line comments
- Keywords are exact identifier matches
- `->` is a single token
- `req/s` is lexed as a unit token (appears after a number)
- All string literals are double-quoted, no escape sequences in v0.1
- No Unicode beyond ASCII in v0.1 identifiers

### Tests (golden token streams)
```
"goal Foo()" → [Goal, Ident("Foo"), LParen, RParen]
"2 req/s"    → [Number(2), ReqPerSec]
"-> Html"    → [Arrow, Ident("Html")]
```

---

## Phase 2 — AST Types (`src/ast.rs`)

All collection fields use `BTreeMap` or `Vec` with stable ordering.
No `HashMap` anywhere.

```rust
pub struct Module {
    pub version: String,
    pub types: Vec<TypeDecl>,        // sorted by name in normalized form
    pub capabilities: Vec<CapDecl>,  // sorted by name in normalized form
    pub intrinsics: Vec<IntrinsicDecl>,
    pub goals: Vec<GoalDecl>,
}

pub struct TypeDecl {
    pub name: String,
    pub base: String,
    pub constraint: Option<TypeConstraint>,
}

pub struct TypeConstraint {
    pub validator: String,   // e.g. "re2"
    pub pattern: String,
}

pub struct CapDecl {
    pub alias: String,
    pub signature: String,  // normalized: "fetch(Url)->Html"
}

pub struct GoalDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub returns: String,
    pub want: WantBlock,
    pub constraints: Vec<ConstraintExpr>,
}

pub struct WantBlock {
    pub stmts: Vec<Stmt>,
}

pub enum Stmt {
    Let { name: String, expr: Expr },
    Map { list: String, binder: String, body: Vec<Stmt> },
    Emit { expr: Expr },
    Return { expr: Expr },
}

pub enum Expr {
    Ident(String),
    StringLit(String),
    Call { name: String, args: Vec<Expr> },
}

pub enum ConstraintExpr {
    Comparison {
        left: Expr,
        op: CmpOp,
        right: ConstraintValue,
    }
}

pub enum ConstraintValue {
    Number(u64),
    Rate { value: u64, unit: String },  // "req/s"
    StringLit(String),
}

pub enum CmpOp { Eq, LtEq }
```

---

## Phase 3 — Parser (`src/parser.rs`)

Recursive descent. No backtracking. Every ambiguity is a parse error.

**Entry point:**
```rust
pub fn parse(source: &str) -> Result<Module, LangError>
```

**Key rules:**
- `version` statement must be first
- `type`, `capability`, `intrinsic` declarations can appear in any order
  (normalization sorts them)
- `goal` declarations come last
- `map` must be followed by a list ident, `as`, a binder ident, and a block
- `let` assigns the result of a call to a name
- `emit` collects a call result into an implicit `collected` list
- `return` is the final statement in a `want` block

**Error type:**
```rust
pub enum LangError {
    UnexpectedToken { expected: String, got: Token, line: usize },
    UnexpectedEof { expected: String },
    DuplicateDeclaration(String),
    UnknownCapability(String),
    VersionMismatch { expected: String, got: String },
}
```

---

## Phase 4 — Normalization (`src/normalize.rs`)

Applied immediately after parsing, before any hashing.

**Rules (from spec §5):**
1. Normalize line endings to LF (done before lexing)
2. Require explicit `version 0.1`
3. Sort `type` declarations by `name` (ascending)
4. Sort `capability` declarations by `name` (ascending)
5. Sort `intrinsic` declarations by `name` (ascending)
6. Normalize signature formatting: strip all whitespace, then insert one
   space after commas in arg lists. Canonical form: `"fetch(Url)->Html"`
7. Parse `N req/s` into `ConstraintValue::Rate { value: N, unit: "req/s" }`
8. Strip any meta/debug fields from AST before hashing

**Signature canonicalization:**
```
"fetch( Url ) -> Html"  →  "fetch(Url)->Html"
"fetch(Url)->Html"      →  "fetch(Url)->Html"
```

**AST hash:**
```rust
pub fn ast_hash(module: &Module) -> String {
    // serialize normalized AST as canonical JSON, sha256
    let json = to_canonical_json(&serde_json::to_value(module).unwrap());
    sha256_str(&json)
}
```

---

## Phase 5 — Lowering (`src/lower.rs`)

Lowers a normalized `Module` to a `PlanIr`. Deterministic — same AST
always produces the same Plan IR.

**Entry point:**
```rust
pub struct LowerOutput {
    pub plan_ir: PlanIr,
    pub source_hash: String,  // sha256 of source bytes
    pub ast_hash: String,     // sha256 of canonical AST JSON
}

pub fn lower(source: &str) -> Result<LowerOutput, LangError>
```

### Lowering Rules

**`map urls as url { BODY }`** (§6.1)

The lowerer expands the map body once per url_index. Since v0.1 maps are
over runtime-provided lists, the lowering produces template nodes tagged
with `url_index` attributes:

```
For each statement S in BODY:
  → create node(S) with attrs.url_index = current_map_depth
  → edge: map_source_node.out → node(S).in (for first stmt)
  → edges between BODY nodes as data dependencies
```

Node IDs are stable: `stable_id("node", "{goal}:{binder}:{op}:{url_index}")`.

**`let x = fetch(url);`** (§6.2)

```
→ CapabilityCall node { op: "fetch", signature: "fetch(Url)->Html" }
→ attrs.url_index = current_map_depth
→ symbol_table[x] = node.output_port
```

**`emit summarize(text);`** (§6.3)

```
→ CapabilityCall or Intrinsic node for the call
→ edge: symbol_table[text].port → node.in
→ collect_node: append node.out to collected_list
```

**`return assemble_json(collected);`** (§6.4)

```
→ Aggregate node { op: "assemble_json" }
→ one input edge per collected item (deterministic ordering: url_index)
→ this node is the terminal node (goal entry point)
```

**Constraints lowering** (§7)

```
rate_limit(fetch) <= 2 req/s
→ ConstraintExpr attached to PlanIr.constraints["rate_limit:fetch"]
  = { "op": "<=", "left": {"fn":"rate_limit","args":["fetch"]},
      "right": {"rate":2,"unit":"req/s"} }

scheduler.max_inflight <= 2
→ SchedulerPolicy.max_inflight = 2

determinism.mode == "sealed_replay"
→ validated at seal time against manifest
```

### Symbol Table

Tracks `name → (node_id, port)` within a scope. Each `map` body creates
a new scope. Shadowing is a LangError in v0.1.

### Node ID Generation

All node IDs use `conclave_hash::stable_id`:

```rust
stable_id("node", &format!("{goal}.{binder}.{op}.{url_depth}"))
```

This guarantees that:
- The same source produces the same node IDs
- Different programs never collide (goal name is part of the key)
- Node IDs are stable across machines

---

## Phase 6 — CLI Integration

**New subcommand:** `conclave lower`

```
conclave lower <source.conclave> [--output plan_ir.json]
```

Outputs:
- `plan_ir.json` — the lowered Plan IR
- Stderr: `source_hash`, `ast_hash`, `plan_ir_hash`

**Updated `conclave plan`:** accept both `.conclave` (lower first) and
`.json` (canonicalize directly). Detect by file extension.

**Updated `Cargo.toml`:** add `conclave-lang` member and path dep in CLI.

---

## Testing Strategy

### Golden Tests (source → expected output)

Three golden fixture files per test case:
```
tests/fixtures/summarize_urls/
  source.conclave
  expected_ast.json
  expected_plan_ir.json
```

Test asserts:
1. `parse(source)` == `expected_ast`
2. `lower(source).plan_ir` == `expected_plan_ir`
3. `lower(source).plan_ir_hash` is stable (run twice, compare)
4. `ast_hash` is stable

### Normalization Tests

For each fixture, also test two whitespace-variant versions of the source
and assert they produce identical AST and Plan IR.

### Constraint Lowering Tests

Tests that `rate_limit(fetch) <= 2 req/s` maps to the correct constraint
JSON, and that invalid constraints produce LangError.

### Hash Stability (cross-machine)

Golden hash fixtures committed in-repo. CI fails if hashes drift.

### Error Case Tests

Each LangError variant has at least one test case triggering it.

---

## Implementation Order

1. `ast.rs` — define all AST types, add serde derives, write serialization test
2. `lexer.rs` — tokenizer, golden token tests
3. `parser.rs` — recursive descent, error cases
4. `normalize.rs` — normalization pass, whitespace variant tests
5. `lower.rs` — lowering rules, golden Plan IR tests
6. CLI `lower` subcommand
7. Update `conclave plan` to accept `.conclave` files

Each step must have passing tests before the next step begins.

---

## Key Invariants (Non-Negotiable)

- No `HashMap` anywhere in `conclave-lang`. All maps are `BTreeMap`.
- No `SystemTime`, `Instant`, or wall-clock in the lowerer.
- Lowering must be a pure function: `lower(source) → PlanIr` with no side effects.
- Node ID generation must be deterministic and collision-resistant.
- Signature normalization must be applied before any hashing.
- All golden test fixtures are committed in-repo and verified in CI.

---

## Open Questions (Decide Before Implementation)

1. **Map over runtime list:** The DSL `map urls as url { ... }` maps over a
   list provided at runtime. The lowerer doesn't know the list length. Options:
   - Lower to a single template subgraph tagged as a `map` in Plan IR
     (requires runtime to expand it per element at execution time)
   - Require list length as a lowering input (simpler but less flexible)
   - v0.1 decision: **require list length as lowering input** — simplest
     to implement, matches current Plan IR structure

2. **`collected` implicit binding:** The `emit` statement appends to an
   implicit `collected` list. This list is passed to `assemble_json`.
   Decision: the lowerer tracks collected nodes in a `Vec<(url_index, NodeId)>`
   and generates input edges in `url_index` order.

3. **Multi-goal modules:** The spec allows multiple goals. v0.1 will lower
   only the first goal (or the `--goal` flag if multiple exist). Defer
   multi-goal support to v0.2.

---

## Estimated Crate Size

| File | Estimated LOC |
|---|---|
| ast.rs | ~100 |
| lexer.rs | ~200 |
| parser.rs | ~350 |
| normalize.rs | ~150 |
| lower.rs | ~300 |
| error.rs | ~50 |
| tests/ | ~400 |
| **Total** | **~1550** |

Comparable in scope to `conclave-manifest` + `conclave-seal` combined.

---

## Relationship to Existing Crates

```
conclave-lang
  └─ depends on: conclave-hash, conclave-ir
  └─ consumed by: conclave-cli (lower subcommand)
  └─ does NOT depend on: conclave-manifest, conclave-runtime, conclave-seal
```

The lang crate knows nothing about sealing or execution. It only produces
Plan IR. The existing pipeline (seal → pack → run) is unchanged.
