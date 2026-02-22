# I Have Built A Small Deterministic Cathedral

*And It Is Probably Unnecessary. But Also Possibly The Future.*

Okay.

Imagine you're explaining this to a normal person.

You:
"I'm building a deterministic orchestration DSL for AI workflows with a sealing phase that cryptographically binds capability artifacts."

Them:
"…so like, an app?"

No.

Not an app.

Not a framework.

Not a container tool.

Not a new JavaScript runtime (please, no).

Not a startup pitch.

It's a thing called Conclave.

And if you have no idea what that means, that's fine.

I barely do either.

But here's the gist:

AI agents are starting to write real systems.

And real systems are not supposed to improvise.

---

## AI Agents Are Incredible — And Also Absolute Menaces

LLMs are astonishing.

They can:

- Refactor entire pipelines.
- Rewrite build systems.
- Optimize loops.
- Replace half your codebase with something "cleaner."

They are brilliant.

They are also the roommate who reorganizes the kitchen at 2am because it "felt inefficient."

You wake up.

Nothing is where it was.

You ask what changed.

They say:

"I made it better."

Did you?

Did you though?

Now scale that to:

- Loan approvals.
- Insurance claims.
- Research results.
- Infrastructure automation.
- Compliance systems.

That's not quirky anymore. You want them off the lease.

---

## Conclave Is My Attempt At Putting A Lock On The Kitchen

Conclave is built around three words:

**Plan.
Seal.
Build.**

That's it.

It sounds dramatic because I have chosen dramatic words.

But the idea is simple.

---

## Plan Is the Blueprint Table

The DSL isn't meant to be a "real" programming language.

It's a structured declaration of intent.

And it's written for agents, not humans.

You (aka one of your agents) write something like:

```conclave
version 0.1;

type Url = String where re2("^https?://");

capability fetch: fetch(Url) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;

goal FetchPages(urls: List<Url>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      emit page;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "live";
    scheduler.max_inflight <= 2;
  }
}
```

Notice: explicit type declarations. Regex-constrained refinements. Strongly typed signatures with `->`. Curly braces everywhere. Every capability named and declared before use. Every constraint written out literally.

A human would find this tedious. An agent finds it unambiguous. There's no "just figure it out from context." There's no convention over configuration. Every decision is stated. Every boundary is named.

That's by design. The goal is a format that a language model can generate reliably, that a linter can validate mechanically, and that a runtime can enforce without interpretation. Not ergonomic. Not expressive. *Exact.*

This is not "run this, then that."

It's more like:

- Here's the shape of the workflow.
- Here's how data flows.
- Here are the components.
- Here are the constraints.

Plan takes that and strips away:

- Formatting.
- Style.
- Human quirks.

It keeps only structure.

Two identical structures? Same hash.

Different structure? Different hash.

Plan is the structural engineer.

No vibes allowed.

And when the plan is finalized — when the graph is canonical and unambiguous — that's the first wisp of smoke. Not white yet. But visible. Something is taking shape.

---

## Seal Is the Freeze Frame

Seal is the moment you stop tweaking.

Seal is the multiplayer lock-in screen where it counts down to zero and despite having no healer or tank, the game still starts.

Seal is the freeze frame before the credits roll.

Seal says:

> "This exact fetch.
> This exact summarize.
> These exact versions.
> Nothing floats."

No silent upgrades.
No surprise dependency drift.
No "latest."

Seal writes the decision down in cryptographic ink.

And yes — this is where the dramatic music plays.

This is the Kiss From A Rose moment giving this step its iconic name.

Because once sealed, it's locked.

Not in a cage.

In commitment.

Seal isn't about control.

It's about devotion.

It's about saying:

> "We are choosing this version.
> We are standing by it."

In a world that auto-updates everything, that kind of commitment feels almost romantic.

---

## Build Is the White Smoke

Build takes:

- The structure.
- The sealed manifest.
- The runtime.

And packs them into a single artifact.

One thing.

Self-contained.
Identifiable.
Reproducible.

And when that artifact is produced — when the hash is finalized and the executable exists — that's white smoke.

Decision made.

No ambiguity.

The world outside the chamber doesn't see the debates, the tweaks, the refactors.

It sees the result.

You can point to it and say:

> "This.
> This exact thing ran."

Run it today.
Run it next year.
Run it somewhere else.

It doesn't ask the internet who it feels like being today.

It already knows.

---

## Real Scenarios Where This Matters

Imagine:

- An AI pipeline approving insurance claims.
- A research workflow producing published results.
- A government system recommending benefits.
- A multi-agent automation system managing infrastructure.
- An AI-powered moderation system making content decisions.

Now imagine someone asks:

> "What exactly ran?"

Without structure, the answer is fuzzy. AI workflows today are chaos wearing a YAML file.

With Conclave:

- Plan hash.
- Manifest hash.
- Artifact hash.
- Replay store.
- Deterministic trace.

Not vibes.

Evidence.

Without Conclave:

You have:

- Parallelism.
- External APIs.
- LLM calls.
- Retry logic.
- Hidden rate limits.
- Floating models.

Run it twice.

Different output.

Ask why.

Silence.

Conclave says:

"If it's sealed, it behaves."

Same input.
Same artifact.
Same output.
Same trace.

It should not be impressive.

And yet it is.

---

## The Pattern Across All These Scenarios

Conclave helps when:

- AI is part of production workflows.
- External tools are involved.
- Parallelism exists.
- Determinism matters.
- Reproducibility matters.
- Auditability matters.
- Drift is expensive.

If none of those are true?

You probably don't need it.

If the cost of "I'm not sure what changed" is low?

Don't build a cathedral.

But if AI systems are making decisions that matter?

You start wanting walls.

Conclave is for the uncomfortable future where:

- AI agents design systems.
- AI agents modify pipelines.
- AI agents upgrade tools.
- AI agents collaborate with each other.
- AI systems make decisions that matter.

That future is coming.

And it is not being built with languages designed for humans, likely before you were born.

---

## Here's The Bigger, Slightly Unhinged Thought

All programming languages today are built for humans.

They assume:

- Humans read the code.
- Humans merge diffs.
- Humans manage versions.
- Humans remember intent.

But in the near future:

- LLMs will write most of the code.
- Agents will refactor entire systems.
- Multi-agent swarms will propose patches.
- Humans will mostly review and approve.

We are about to let probabilistic reasoning systems modify deterministic infrastructure at scale.

That should make you slightly nervous.

It makes me nervous.

Conclave is my small attempt at drawing a boundary:

Inside: creative chaos.

Outside: deterministic artifact.

---

## Conclave Probably Isn't The Language

Let's be real.

Conclave probably isn't The One True Agent Language.

It's experimental.
It's opinionated.
It's a little dramatic.

It may never escape niche workflow land.

But maybe that's fine.

Maybe the real point is this:

We need languages and systems that are built for agents as primary authors.

Not humans.

Languages that:

- Are canonical.
- Are structurally stable.
- Separate proposal from commitment.
- Make drift impossible by default.
- Treat execution as an artifact, not a vibe.

Maybe Conclave is a prototype.

Maybe it's a thought experiment with good branding.

Maybe someone else builds the real version.

---

## The Honest Ending

Will Conclave matter?

Probably not.

Will it change the industry?

Almost certainly not.

Will it make my life richer financially?

No.

But sometimes the payoff is simply:

Understanding the world a little better than you did before.

And in a world where everything auto-updates, auto-refactors, auto-optimizes, auto-improves…

Standing still might be the most radical feature of all.
