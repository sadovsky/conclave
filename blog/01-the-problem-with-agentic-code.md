# What Happens When Your AI Agent Lies — And You Can't Tell

*A case for deterministic execution in the age of agentic coding.*

*AI Generated*

---

We are in the middle of a shift that most people haven't fully named yet.

AI isn't just writing code anymore. It's running it. Planning it. Deciding what to fetch, what to summarize, what to pass to the next step. The phrase "agentic coding" sounds technical, but the idea is simple: instead of asking an AI for a suggestion you then execute yourself, you hand it a goal and it figures out the steps, runs them, and gives you the result.

This is genuinely powerful. It's also genuinely risky — for a reason that isn't talked about enough.

## The Problem Nobody Talks About

When an AI agent runs your code, it typically does something like this:

1. Fetch some URLs
2. Summarize what it found
3. Assemble the results into a report

Simple enough. Now ask yourself: **if you ran that same agent again tomorrow, would you get the same result?**

Almost certainly not. The pages it fetched may have changed. The model may return different summaries. The order things ran in might differ. The timing is different. A hundred small things shift, and the output changes.

That's fine for some use cases. But when you're making decisions based on agent output — when you're trusting it, auditing it, comparing runs, or trying to reproduce a finding — non-determinism is a serious problem.

You can't tell if the output changed because the world changed, or because the agent behaved differently. You can't prove what happened. You can't replay it. You can't verify it.

This isn't a theoretical concern. It becomes acute the moment you try to:

- Debug why an agent produced a bad result last Tuesday
- Audit a financial or legal decision an agent made
- Run a compliance check on what data was actually accessed
- Compare two agent runs and know the difference is meaningful

Right now, most agentic systems have no answer to any of these questions.

## The Deeper Issue

The problem isn't just that AI outputs vary. The problem is that **the infrastructure underneath agents was never designed for auditability**.

Agents call external APIs. They read from URLs. They use models that are updated without notice. The capabilities they depend on — "fetch this page", "summarize this text" — are treated as black boxes with no stable identity.

When something goes wrong, you have logs. Maybe. If someone thought to add them. In whatever format whoever wrote the code preferred.

That's not a foundation you can build trust on.

## What Conclave Does

Conclave starts from a different premise.

> Determinism is not optional. It is the foundation everything else is built on.

The core idea is straightforward: before an agent runs, you *seal* its execution plan. Sealing means:

- Every capability it will use (fetch, summarize, etc.) is identified by the **SHA-256 hash of its binary** — not by name or version string, but by its exact content
- Every external call that touches the network is configured as **replayable** — the first live run records the results; every subsequent run replays them deterministically
- The execution plan itself is hashed and committed — you can't silently change what the agent does
- The scheduler runs in a **virtual clock**, so timing is controlled and reproducible

The result: given the same sealed artifact and the same inputs, Conclave produces **identical output, every time**. Not approximately identical. Byte-for-byte identical. The execution trace has a hash. You can compare it across machines, across time, across runs.

## What a Conclave Program Looks Like

A Conclave program is a directed graph of *nodes*. Each node is either:

- A **capability call** — something that touches the outside world (fetching a URL, calling an API)
- An **intrinsic** — a pure computation (parsing, transforming)
- An **aggregate** — combining results from multiple branches

The scheduler runs these nodes deterministically: respecting data dependencies, enforcing rate limits, and ordering ties by stable rules (url index, then node kind, then node ID). No randomness. No wall clock. No surprise ordering.

Here's the execution trace from running a 3-URL summarization plan:

```
t=     0ms  DISPATCH   F1    ← fetch url[0]
t=     0ms  DISPATCH   F2    ← fetch url[1], both start immediately (max_inflight=2)
t=   214ms  COMPLETE   F1
t=   214ms  DISPATCH   E1    ← extract text from F1's result
t=   229ms  COMPLETE   E1
t=   229ms  DISPATCH   S1    ← summarize E1's result
t=   314ms  COMPLETE   S1
t=   456ms  COMPLETE   F2
t=   456ms  DISPATCH   E2
...
t=  1000ms  DISPATCH   F3    ← rate limiter: only 2 req/s, F3 waits for next window
t=  1500ms  COMPLETE   A     ← final assembly
```

That `t=1000ms` pause for F3 isn't accidental. The scheduler enforces a 2-requests-per-second rate limit. Every time. On every machine. The trace is a precise, auditable record of exactly what happened and when — in virtual time.

## The Seal Is the Contract

When you seal a Conclave program, you produce a manifest. The manifest records:

- The hash of the execution plan
- The hash of every capability binary that will be used
- The determinism mode (live vs. sealed replay)
- The scheduler policy
- The IO policy (what's allowed to touch the network)

That manifest is itself hashed. If anything changes — if someone swaps in a different version of the fetch capability, if the plan changes, if the toolchain changes — the hash changes, and the seal is broken.

This makes cheating hard and auditing easy. The artifact is a commitment. Run it twice and you get the same trace hash. Give it to someone else and they can verify they're running exactly what you ran.

## Who This Is For

Conclave isn't trying to replace your AI tools. It's trying to give them a substrate they can be trusted on.

The easy answer is "high-stakes domains: financial, medical, legal." That's true — but it's too narrow. The core problem Conclave addresses is more general:

> **How do you safely move from probabilistic reasoning to deterministic execution?**

That problem shows up anywhere AI participates in building or orchestrating systems. The real dividing line isn't industry. It's one question:

*Do you care whether the same input tomorrow produces the exact same artifact and execution trace?*

If yes, Conclave is relevant. If no, it's probably overkill.

**Where it fits:**

- **AI-generated internal tools** — Even low-stakes tools are painful to debug without reproducibility. When an agent generated the workflow, you can't reason about it without a record of what it actually ran.

- **Data pipelines and research** — Reproducible experiments are hard enough without AI in the loop. With AI, they become nearly impossible without explicit determinism infrastructure.

- **CI/CD workflows generated by agents** — Agents rewriting build scripts can introduce subtle nondeterministic drift. Content-addressed capability binding catches this.

- **Multi-tool AI workflows** — Agents calling tools in parallel can produce race conditions or unstable ordering. The deterministic scheduler eliminates both.

- **Autonomous infrastructure agents** — Any agent that can take actions on real systems needs a clear boundary between "planning" and "committing." The seal is that boundary.

- **Supply-chain conscious development** — Content-addressed capability binding is useful even without AI. Knowing exactly which binary ran, identified by hash, is a supply-chain property independent of who wrote the orchestration.

**What it is not for:**

If you're writing CRUD services manually, Conclave doesn't buy you much. It's not aimed at static web apps, traditional backends, one-off prototypes, or systems where nondeterminism genuinely doesn't matter. The overhead of sealing and replay is real — it only makes sense when auditability has value.

The more precise framing:

> Conclave is for systems where AI is allowed to think, but not allowed to silently mutate reality.

That's broader than any single industry. It includes AI DevOps, research automation, orchestration layers, reproducible ML pipelines, and any workflow where the difference between two runs needs to be *explainable* — not just observed.

The thesis is simple: collective intelligence and strict determinism can coexist. Agents can be powerful *and* auditable. You don't have to choose.

---

*Conclave is an open-source project. The code, tests, and conformance spec are available on GitHub.*
