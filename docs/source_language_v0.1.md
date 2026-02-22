# Conclave Source Language v0.1

This document specifies the Conclave v0.1 source language front end.

The source language lowers deterministically into Plan IR.
Determinism is a hard invariant.

This spec defines:

- Surface DSL syntax
- Core JSON format
- AST schema
- Normalization rules
- Deterministic lowering rules

---

# 1. Design Goals

The v0.1 source language must:

- Be small and regular
- Be easy for coding agents to generate
- Avoid ambiguity
- Avoid implicit behavior
- Lower deterministically to Plan IR

v0.1 intentionally avoids advanced features.

---

# 2. Surface DSL (Braces-Based)

## 2.1 Example

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

---

# 3. Grammar (Simplified)

## 3.1 Top-Level

```
module        ::= "version" number ";"
                  (type_decl | capability_decl | intrinsic_decl | goal_decl)*

type_decl     ::= "type" IDENT "=" TYPE_EXPR ("where" IDENT "(" STRING ")")? ";"
capability_decl ::= "capability" IDENT ":" SIGNATURE ";"
intrinsic_decl  ::= "intrinsic" IDENT ":" SIGNATURE ";"
goal_decl     ::= "goal" IDENT "(" param_list ")" "->" TYPE_EXPR block

block         ::= "{" goal_body "}"
goal_body     ::= want_block constraints_block?
```

## 3.2 Want Block

```
want_block    ::= "want" block
statement     ::= let_stmt | map_stmt | emit_stmt | return_stmt
let_stmt      ::= "let" IDENT "=" call_expr ";"
map_stmt      ::= "map" IDENT "as" IDENT block
emit_stmt     ::= "emit" expr ";"
return_stmt   ::= "return" expr ";"
```

## 3.3 Constraints

```
constraints_block ::= "constraints" "{" constraint_stmt* "}"
constraint_stmt   ::= expr ";"
```

---

# 4. AST Schema (Normalized Form)

The parser produces a normalized AST.

## 4.1 Module

```json
{
  "version": "0.1",
  "types": [...],
  "capabilities": [...],
  "intrinsics": [...],
  "goals": [...]
}
```

## 4.2 Goal

```json
{
  "name": "SummarizeUrls",
  "params": [{"name":"urls","type":"List<Url>"}],
  "returns": "Json",
  "want": {...},
  "constraints": [...]
}
```

## 4.3 Want Nodes

Allowed node types (v0.1):

- Let
- Map
- Emit
- Return
- Call
- Ident
- Literal

Example normalized AST for map:

```json
{
  "type":"Map",
  "list":"urls",
  "binder":"url",
  "body":[ ... ]
}
```

---

# 5. Normalization Rules

Before lowering:

1. Normalize line endings to LF.
2. Require explicit `version 0.1`.
3. Sort type declarations by name.
4. Sort capability declarations by name.
5. Normalize signature formatting: no whitespace variance.
6. Parse numeric units (e.g., `2 req/s`) into structured form.
7. Exclude debug/meta fields from hashing.

All AST objects must be serialized canonically before hashing.

---

# 6. Deterministic Lowering to Plan IR

## 6.1 Map Lowering

```
map urls as url { BODY }
```

Lowers to:

- A MAP node with attribute:
  - index_var = url_index
- BODY nodes replicated per element
- Deterministic ordering key:
  (url_index, node_kind, node_id)

## 6.2 Let Lowering

```
let x = fetch(url);
```

Lowers to:

- Capability node
- Edge from url binding
- Symbol table entry x → node output

## 6.3 Emit Lowering

```
emit summarize(text);
```

Lowers to:

- Capability node (if call)
- Collect node (append to list)
- Deterministic index ordering

## 6.4 Return Lowering

```
return assemble_json(collected);
```

Lowers to:

- Final capability/intrinsic node
- Terminal node in Plan IR

---

# 7. Constraints Lowering

Example:

```
rate_limit(fetch) <= 2 req/s;
```

Normalized AST:

```json
{
  "op":"<=",
  "left":{"fn":"rate_limit","args":[{"ident":"fetch"}]},
  "right":{"rate":2,"unit":"req/s"}
}
```

Constraints are embedded into Plan IR and referenced by node IDs.

---

# 8. Core JSON Format

Agents may bypass DSL and provide canonical JSON:

```json
{
  "version":"0.1",
  "goals":[ ... ]
}
```

Core JSON maps directly to normalized AST.

No implicit transformations allowed.

---

# 9. Determinism Guarantees

The source front end must guarantee:

- Same source bytes → identical AST
- Same AST → identical Plan IR
- No whitespace or formatting influence on semantics
- No hidden inference

---

# 10. v0.1 Scope Limits

Not supported in v0.1:

- Conditionals
- Loops other than `map`
- User-defined functions
- Mutable state
- Async semantics
- Dynamic capability resolution

These may be added in future versions with explicit version bump.

---

# 11. Versioning

Any grammar change requires:

- Version increment
- Explicit migration path
- Updated canonicalization rules
- Updated golden tests

---

# 12. Summary

Conclave v0.1 source language is:

- Minimal
- Explicit
- Deterministic
- Agent-friendly
- Graph-oriented

It is intentionally small to preserve reproducibility.
