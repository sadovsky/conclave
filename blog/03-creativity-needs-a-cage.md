# Creativity Needs a Cage: Why Agents Need Both a Language and Determinism

*On the missing piece between "an agent wrote this" and "I can trust this."*

---

There's a tension at the heart of agentic systems that nobody talks about clearly.

On one side: agents are most powerful when they can be creative. When they can look at a goal — "summarize these three pages and give me a JSON report" — and figure out the steps themselves. The ability to reason about *how* to accomplish something, not just *what* the answer is, is what makes agents genuinely useful.

On the other side: creativity is exactly what makes agents hard to trust. Every time an agent makes a decision — about what to call, in what order, with what parameters — it's introducing variability. And variability, in systems that make real decisions, is the enemy of auditability.

The question is: can you have both?

We think yes. But it requires building the right interface between them.

---

## What Agents Are Actually Good At

When you ask an agent to solve a problem, it's doing something that looks a lot like programming — but without being asked to write code explicitly.

It reasons about the goal. It decides what steps are needed. It figures out dependencies. It handles the case where step 2 needs the output of step 1 before it can run. This is planning — and agents are genuinely good at it.

The problem is that today, when an agent does this planning, the plan is invisible. It happens inside the model, and what comes out is either natural language ("here's what I'll do") or direct tool calls — neither of which is a structured artifact you can inspect, audit, or replay.

The plan exists only implicitly. And an implicit plan can't be verified.

---

## A Language Designed for Agents, Not Humans

The Conclave source language is designed around a simple observation: agents think in goals, not in procedures.

When you ask a capable agent to summarize a list of URLs, it doesn't think "first I need a loop, then I need to call an HTTP library, then I need to handle errors." It thinks: "I want the summary of each URL. I need to fetch, extract, summarize, and collect." It thinks in *wants*.

The `want` block is Conclave's surface for exactly that:

```conclave
version 0.1;

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

This isn't trying to be clever. It's trying to be *clear* — clear enough that an agent can generate it reliably, and clear enough that a human can read what the agent intended.

The key properties:

**Goals, not functions.** A `goal` takes inputs and declares what it wants to produce. There's no imperative control flow, no mutation, no side effects hidden in function calls. The shape of the program is the shape of the data flow.

**`want` is declarative.** The `want` block describes what computations are needed and how they depend on each other. The scheduler figures out how to run them. The agent doesn't need to reason about concurrency or ordering — that's handled by the runtime.

**Capabilities are explicit.** Every external call is declared at the top of the file. There's no hidden I/O. An agent generating this program has to commit, up front, to what the program will touch.

**Constraints are first-class.** Rate limits, scheduler policies, determinism mode — these aren't configuration files or annotations. They're part of the program. The agent expresses its intent about how the program should run, not just what it should do.

---

## What Creativity Produces (And Why It Needs to Be Contained)

Imagine you're using an agent to generate Conclave programs. You give it a goal. It produces a `.conclave` file. You seal it, pack it, and run it.

Now you run it again tomorrow.

Without determinism: you might get different results. Maybe the agent-generated plan was slightly different this time. Maybe a capability returned something different. Maybe the order things ran in changed. You have output, but you don't have a way to verify that it's the same output.

With Conclave: the plan is a sealed artifact. Its hash is a commitment. The capability binaries it uses are pinned by their content hashes. The execution trace has a hash. You can prove that run 2 is identical to run 1 — or explain precisely why it isn't.

The creativity happens at generation time. The agent decides what the goal is, how to decompose it, what capabilities to use, what constraints to apply. That's the interesting, creative, open-ended part.

But once the plan is sealed, it runs deterministically. The cage doesn't constrain the creativity — it captures the result of the creativity as an auditable artifact.

---

## How Lowering Works

The path from source language to execution has three steps:

**1. Parse → AST**

The source file is parsed into a normalized abstract syntax tree. "Normalized" matters: the parser sorts declarations, normalizes whitespace, and removes anything that doesn't affect semantics. The result is a canonical representation of what the agent wrote — not what it typed.

The same program, formatted differently, produces the same AST. This matters because agents are inconsistent formatters.

**2. AST → Plan IR**

The AST is lowered deterministically into the Plan IR — the directed graph of nodes and edges that the scheduler runs. Every rule is explicit:

- A `map` over a list becomes replicated nodes, one per element, with stable ordering keys
- A `let x = fetch(url)` becomes a capability node with an edge from the URL binding
- An `emit` becomes a capability node whose output is collected
- A `return` becomes the terminal node that produces the goal's output

The lowering is pure and mechanical. Same AST, same Plan IR. No choices, no inference.

**3. Hash everything**

The source file is hashed. The AST is hashed. The Plan IR is hashed. All three hashes are recorded. If you change one character in the source, every downstream hash changes.

This chain of hashes is how you verify that what ran is what the agent wrote. Not approximately. Exactly.

---

## The Loop

Here's the full loop as we envision it:

```
Agent generates program
        ↓
.conclave source file (sha256:...)
        ↓
conclave lower → plan_ir.json (sha256:...)
        ↓
conclave seal → sealed_manifest.json (sha256:...)
        ↓
conclave pack → artifact.cnclv (sha256:...)
        ↓
conclave run → trace.json (trace_hash: sha256:...)
```

At every step, the output is hashed. At every step, the previous hash is an input. The chain is unbreakable. If any step changes — if the agent rewrites the program, if someone swaps a capability binary, if the plan changes — the hashes diverge, and you know.

The agent is creative. The pipeline is accountable.

---

## What This Unlocks

Once you have this loop, some things become possible that aren't today:

**Agent-generated programs are auditable.** You can inspect what the agent wrote, trace it through lowering and sealing, and verify that the artifact that ran corresponds to the intent the agent expressed. "The agent wrote this" becomes a meaningful, verifiable claim.

**Multi-agent systems can share sealed plans.** One agent can generate a plan, seal it, and hand it to another agent — which can inspect and verify the plan before running it. Trust doesn't have to be blind.

**Version control over plans, not just code.** A plan is a content-addressed artifact. You can store it, share it, reference it by hash, and know that two references to the same hash are the same plan — always.

**Debugging is possible.** When something goes wrong, you have the source, the AST, the Plan IR, the sealed manifest, and the execution trace. You can walk backwards from the wrong output to the wrong assumption to the wrong constraint and find where the agent's creativity produced a bad plan.

---

## The Cage Is the Point

We called this post "creativity needs a cage." That framing might sound restrictive. It isn't.

The cage isn't a limitation on what agents can think or generate. It's a structure that makes the output of their thinking trustworthy. A sealed plan is a cage in the same way a signed contract is a cage: it doesn't prevent you from making agreements, it makes the agreements you make mean something.

Agents are going to write more and more of the programs that matter. The question isn't whether to give them that power. It's whether we build the infrastructure to make that power accountable.

Conclave is that infrastructure.

---

*The Conclave source language spec is in the repository. Implementation is planned for v0.2.*
