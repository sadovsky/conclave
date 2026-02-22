# Conclave Plan IR (v0.1)

This document specifies the **Plan IR**: Conclave’s canonical, serialization-stable representation of a program after parsing + normalization and before proposal/optimization.

Plan IR must support:

- Deterministic hashing (`plan_ir_hash`)
- Graph execution semantics (nodes + edges)
- Constraint attachment at goal / subgraph / node level
- Stable node identifiers independent of formatting

---

## 1. Core model

### Entities

- **PlanIR**: the whole graph for one compilation unit  
- **Node**: an operation (capability call, intrinsic, control, aggregate)  
- **Edge**: a value dependency (output port → input port)  
- **Constraint**: declarative rule attached to scope  
- **TypeShape**: structural types with optional predicates  

---

## 2. Schema

> The schema is presented in JSON-ish notation for clarity. Implementations may use structs/classes, but must serialize canonically as described in §6.

```json
{
  "conclave_ir_version": "0.1",
  "module": {
    "name": "string",
    "source_fingerprint": "sha256:..." 
  },

  "types": {
    "TypeName": {
      "kind": "primitive|struct|list|map|union|alias",
      "of": "...",
      "fields": { "k": "Type" },
      "variants": ["Type"],
      "predicates": [
        { "lang": "re2", "expr": "^https?://" },
        { "lang": "dsl", "expr": "value > 0" }
      ]
    }
  },

  "goals": [
    {
      "goal_id": "gid:...",
      "name": "SummarizeUrls",
      "params": [{ "name": "urls", "type": "List<Url>" }],
      "returns": [{ "name": "out", "type": "Json" }],

      "constraints": [ { "$ref": "#/constraints/c1" } ],
      "accept": [ { "$ref": "#/constraints/a1" } ],

      "entry_nodes": ["nid:..."],
      "exit_nodes": ["nid:..."]
    }
  ],

  "nodes": [
    {
      "node_id": "nid:...",
      "kind": "capability_call|intrinsic|control|aggregate",
      "op": {
        "name": "fetch",
        "signature": "fetch(Url)->Html"
      },

      "inputs": [
        { "port": "in.url", "type": "Url", "source": { "edge_id": "eid:..." } }
      ],
      "outputs": [
        { "port": "out.html", "type": "Html" }
      ],

      "attrs": {
        "determinism_profile": "replayable|fixed|nondet",
        "cost_hints": { "latency": "variable", "cpu": "low" }
      },

      "constraints": [ { "$ref": "#/constraints/nc7" } ],
      "meta": {
        "origin": "lowered_from.want",
        "span": { "start": 123, "end": 187 }
      }
    }
  ],

  "edges": [
    {
      "edge_id": "eid:...",
      "from": { "node_id": "nid:...", "port": "out.html" },
      "to":   { "node_id": "nid:...", "port": "in.html" }
    }
  ],

  "constraints": {
    "c1": {
      "constraint_id": "cid:...",
      "scope": "goal|node|subgraph",
      "expr": {
        "lang": "conclave-constraint-0.1",
        "ast": {
          "op": "<=",
          "left": { "fn": "rate_limit", "args": ["fetch"] },
          "right": { "lit": "2 req/s" }
        }
      }
    }
  },

  "subgraphs": [
    {
      "subgraph_id": "sid:...",
      "kind": "map|reduce|pipeline|branch",
      "nodes": ["nid:...", "..."],
      "constraints": [ { "$ref": "#/constraints/c1" } ]
    }
  ],

  "exports": {
    "entry_goal": "gid:..."
  }
}
```

---

## 3. Node kinds (v0.1)

Minimum viable set:

- `capability_call`: invokes a capability by name+signature  
- `intrinsic`: deterministic built-ins (e.g., `assemble_json`, `validate_json`)  
- `control`: structural control (e.g., `map`, `join`, `gate`, `rate_limiter`)  
- `aggregate`: collection ops (collect, merge, group)  

> Control constructs should lower to nodes to keep the runtime uniform.

---

## 4. Type normalization rules

- Use **structural typing** for builtins; allow aliases for ergonomics.
- Normalize type strings (no whitespace differences, stable generic formatting).
  - Example: `List<Url>` not `List< Url >`.

---

## 5. Constraint language requirements (v0.1)

Keep it small and AST-based (not raw strings):

- Comparisons: `<, <=, ==, >=, >`
- Boolean ops: `and, or, not`
- Functions: `rate_limit(cap)`, `max_tokens(cap)`, `determinism`, `fetch_mode`, `output_format`, `total_time`
- Literals: numbers, durations (`ms`, `s`), rates (`req/s`), strings

Represent constraints as an AST to avoid normalization ambiguity.

---

## 6. Canonicalization + hashing

### 6.1 Canonical JSON encoding

To compute `plan_ir_hash`, Conclave MUST:

1. Remove all `meta` fields and any debug-only fingerprints/spans.
2. Sort:
   - object keys lexicographically
   - `types` by key
   - `nodes` by `node_id`
   - `edges` by `edge_id`
   - `constraints` by `constraint_id`
3. Ensure numeric normalization (no `1` vs `1.0` ambiguity).
4. Serialize as UTF-8 JSON with:
   - no insignificant whitespace
   - stable escaping rules

### 6.2 Stable IDs

Define `node_id`, `edge_id`, `goal_id`, `constraint_id` as:

`sha256("conclave:v0.1" || entity_kind || canonical_entity_body_without_ids)`

Notes:

- IDs are computed **after** canonicalizing the entity body that excludes the ID itself.
- Edges: canonical body uses from/to node IDs + ports.
- Goals: uses name, param/return types, entry/exit node IDs, attached constraint IDs.

### 6.3 Hash outputs

- `plan_ir_hash = sha256(canonical_full_plan_ir_json)`
- Use one encoding consistently; recommended: `sha256:<hex>`

---

## 7. Compatibility + evolution

- Any breaking schema change increments `conclave_ir_version`.
- A v0.1 runtime MUST reject Plan IR versions it does not understand.
