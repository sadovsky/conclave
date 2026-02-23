# Multi-Agent Conclave: What Already Works, and What Comes Next

*AI Generated* — *February 2026*

Here is something that already happened without any fanfare.

Two different agents wrote two different capabilities. One wrote `conclave-cap-fetch` — a Rust binary that makes HTTP requests. The other wrote `cap_word_count` — a Python script that strips HTML tags and counts tokens. They were written independently, in different languages, at different times, for different purposes.

Then a third agent composed them into a single Conclave program, sealed it, and ran it.

It worked.

Nobody orchestrated the composition. Nobody verified that the languages would be compatible. Nobody checked if the authors had talked to each other. The seal phase just asked one question per capability: **does the hash match?**

It did.

That's multi-agent Conclave v0.1. It's quiet. It's unspectacular. And it's already real.

---

## Why the capability layer is the natural boundary

Most multi-agent systems coordinate by passing messages. Agent A tells Agent B what to do. Agent B tells Agent C. The coordination graph is usually more complex than the work graph. Things go wrong at the seams.

Conclave takes a different approach. Capabilities aren't messages. They're artifacts.

A capability is defined entirely by three things:

1. **Its signature** — what type it takes, what type it returns (`fetch(Url)->Html`)
2. **Its content hash** — `sha256(binary_bytes)`, derived from the actual bytes of the executable
3. **Its determinism profile** — replayable, sandboxed, or otherwise

That's it. The runtime doesn't care who wrote it. The seal phase doesn't care what language it's in. The Plan IR doesn't care when it was built. All of that information collapses into a single hash at seal time.

```
capability fetch: fetch(Url) -> Html;        // declared by the goal author
artifact_hash: sha256:ee1f247bd06632de...    // provided by the capability author
```

Those two things come from different people. Seal is where they meet.

---

## The fetch + word_count collaboration

In the v0.1 session that produced this capability pair, the concrete story was:

**Agent A** (the infrastructure agent) built `conclave-cap-fetch` in Rust — a real HTTP capability with TLS support, content-addressed, installed into the capability store:

```
$ ./target/release/conclave install-cap ./target/release/conclave-cap-fetch
sha256:ee1f247bd06632de389269e81a05ccd0361c60e2729eb7f49deaefbb39ae1216
```

**Agent B** (the task-specific agent) wrote `cap_word_count.py` in Python, from scratch, given only the AGENT_GUIDE. It implemented the same JSON stdio protocol, installed into the same store:

```
$ ./target/release/conclave install-cap examples/cap_word_count.py
sha256:4f2d88d0d5040a6266a37cdd0d9fe9017f73f41c1e0367a7d12de453b38be026
```

**Agent B** then wrote the Conclave program that composed them:

```conclave
capability fetch: fetch(Url) -> Html;
capability word_count: word_count(Html) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;

goal CountWords(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      let count = word_count(page);
      emit count;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
    scheduler.max_inflight <= 2;
  }
}
```

The manifest named both hashes. Seal validated both. Pack bundled everything. Run produced a deterministic trace.

Nobody negotiated. The capability contract — a function signature and a hash — was the entire protocol.

---

## What makes this work at all

The capability ABI is deliberately minimal. Any executable that reads one JSON line from stdin and writes one JSON line to stdout is a valid Conclave capability. The JSON structure is fixed:

**stdin:**
```json
{
  "capability": "word_count(Html)->String",
  "inputs": { "html": "..." },
  "context": { "seed": 1337, "virtual_time": 0, "determinism_profile": "replayable" }
}
```

**stdout:**
```json
{ "output": { "type": "String", "data_b64": "..." }, "duration_ms": 1 }
```

That's the whole protocol. A Python script, a Rust binary, a compiled Go program, a shell script that calls out to a model — they all look identical to the runtime. The only question Seal asks is whether the hash you declared in the manifest matches the hash of the binary sitting in the store.

This is a composition boundary that doesn't require coordination. You don't need to agree on a framework, a language, a serialization format, or a deployment model. You need to agree on the signature — and the DSL makes that explicit up front.

---

## What v0.1 can't do (yet)

The multi-agent story above is real, but narrow. Both capabilities were written before the goal that used them existed. The goal author knew the signatures they needed and wrote the program around them. That's one form of composition — bottom-up, capability-first.

What v0.1 doesn't support:

**Goal composition.** There's no module system. You can't import another agent's goal as a step in your goal. Every program is one file, one goal, one lowering. If Agent A has a working `FetchAndExtract` goal and Agent B wants to build `SummarizePages` on top of it, they have to copy the logic or inline it manually.

**Multi-agent plan proposal.** Today one agent writes the entire `.conclave` source. There's no protocol for two agents to collaboratively draft the intent graph — one proposing the shape of the computation, another filling in the constraint policy.

**Capability discovery.** The goal author has to know the hash. There's no index, no search, no registry. If Agent C wants to use a capability that Agent A built two months ago, it needs to know the hash out of band.

**Distributed execution.** Everything runs on one machine. There's no protocol for splitting a Plan IR across multiple machines and reassembling the results into a coherent trace.

These aren't design oversights. They're scope. v0.1 proves that the determinism invariants hold. Everything else is built on top of that foundation.

---

## What comes next

The interesting multi-agent patterns aren't far off.

### Capability registries

The content-addressed store is already the right abstraction. A capability registry is just a content-addressed store with a lookup index: given a signature like `fetch(Url)->Html`, return the known hashes and their metadata. Agents could query it. Seal could consult it automatically for unresolved bindings.

The tricky part isn't the storage. It's trust. Who vouches for a hash? This leads to the next pattern.

### Signed capabilities

Right now, `artifact_hash` is just bytes. There's no signature over those bytes from a known principal. In a multi-agent world, you'd want to know: did this capability come from a known team? Was it reviewed? Has it been used successfully in N prior sealed programs?

This is proof-carrying capabilities: the manifest entry includes not just the hash but an attestation chain. Agent A signs their capability. Agent B's manifest includes the signature. Seal can verify the chain without trusting any particular agent implicitly.

The groundwork is there — the manifest schema has a `signatures` field on each binding. It's `null` in v0.1. In v0.2, it isn't.

### Goal modules

A goal module is a named, versioned, content-addressed unit that can be imported like a capability. Instead of `fetch(Url)->Html` as a leaf operation, you'd have `SummarizeUrl(Url)->String` as a reusable sub-goal — written by one agent, signed, and importable by another.

The lowerer would resolve imports at lower time, not at runtime. The imported goal expands into the parent Plan IR as a subgraph. The resulting plan_ir_hash commits to the full expanded graph including all transitive imports.

This is where multi-agent becomes multi-team. Agent teams can own sub-goals. Composition is just IR expansion. The hash chain remains intact.

### Multi-agent proposal protocol

Today's workflow: one agent writes a `.conclave` program and runs it. A future workflow: one agent drafts the intent graph, a second agent reviews the constraints and capability choices, a third agent runs the seal and signs the manifest.

This doesn't require changes to the runtime. Seal already validates everything. The proposal protocol is just a handoff convention: who has authority to advance the program from `lower` to `seal`, and what signatures are required to do so?

The manifest's `supply_chain` block is already designed for this. `require_artifact_signatures: false` in v0.1. `require_artifact_signatures: true` with a threshold policy in v0.2 means no single agent can seal unilaterally.

### Distributed deterministic execution

The single-threaded scheduler is a feature in v0.1 — it makes determinism trivially provable. But single-threaded also means single-machine, and single-machine means the scale ceiling is low.

A distributed executor would need to partition the Plan IR across nodes, execute each partition deterministically, and aggregate the results into a single trace. The hard part isn't the execution — it's the proof of correctness. How does the orchestrating agent know that a remote executor didn't cheat?

One path: each executor emits a signed trace fragment. The assembler verifies all signatures before constructing the composite trace. The composite trace hash is then a function of all fragments — if any executor deviated, the hash changes.

This is further out. But the architecture isn't hostile to it. The Plan IR is a graph. Graphs partition. The trace is deterministic. Deterministic traces compose.

---

## The pattern beneath all of this

Every multi-agent future I've described above has the same structure:

1. Agents work independently at well-defined boundaries.
2. Seal is where independent work becomes a shared commitment.
3. The hash chain makes defection visible.

That's the architecture. Multi-agent Conclave isn't about agents talking to each other in real time. It's about agents producing artifacts that can be composed without trust, verified without coordination, and replayed without the authors present.

The capability layer works today because it has that structure. You don't have to trust the fetch capability author. You trust the hash. You trust Seal. You trust the chain.

As the boundaries move up the stack — from capabilities to goals to programs to registries — the same structure should hold. The hard work isn't writing more code. It's maintaining the property that composition can be verified without coordination.

We have a small version of that working.

Everything else is building upward on the same foundation.

---

*The `conclave-cap-fetch` and `cap_word_count` capabilities used as examples in this post are both in the v0.1 repository. The full end-to-end session that produced them is documented in [blog/05-agent-writes-a-capability.md](05-agent-writes-a-capability.md).*
