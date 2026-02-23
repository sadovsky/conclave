# Preamble

*Now with 100% more mammalian autonomy!*

So this is a purely human authored preamble to this cyborg manifesto. A friend told me when he sees anything AI authored, it makes him wonder if the “author” even read it, and I get that. In a growing age of AI content, without a human voice it’s often impossible to know if we’re just chasing echo’d LLM patterns or actually approaching problems with thought. I like how the AI co-authored piece lays out the problem. It has my logic flow. It has my edits. But yes, it does reek of AI authorship because yes, it was vibed for the most part. I could go on about how this hyperreality, the simulation of intelligence, is some postmodern structural inevitably, but that’s a whole other topic (and one, that thanks to dating an English major who introduced me to Baudrillard throughout college, is one that I’m always equipped to talk about for a very long time.). But I digress. So before we get to the AI aided, “simulated” manifesto, let’s talk 1:1 like humans for a bit (assuming you’re actually a human reading this).

---

For the past couple weeks I’ve been wondering how AI would write its own language. The idea that machines would just speak in 1’s and 0’s machine code seems popular in the common mindset, but from an information theory point of view, seems pretty wasteful. In fact, machines I’d assume would want to speak in even higher levels of abstraction. That said, there’s no need for variance or syntactic sugar. They should be able to express ideas in terms of plans and constraints and the “hive mind” should generally be able to agree on what this means.

---

So I started off trying to build that. What would a programming language for AI Agents look like… But I quickly realized it would be pointless to reinvent all known software in yet another language. AI Agents today are impressive because they can glue insane amounts of context together without a second thought. And that’s really their power, using so much trained libraries to generate code as needed. Even if they are amazing transpilers, it’s not worth the tokens.

---

So in steps another problem I’ve been really interested in: multiple agents working together. In my late Bachelor studies with John Holland, and then subsequently my PhD, I became obsessed with emergence. Multiple tiny agents working together to create a whole that’s more than the sum of its parts. In biological, social and technological systems you can represent the emergence as some type of network, which is captured well as a graph. I felt like if a language was ever going to be successful for agents, it would have to have the ability for chucks of it to be debated and structure to be inter changeable. Plus graphs make great patterns and have amazing properties. They are the substrate that agents naturally would gravitate to.

---

Next came a problem that I kept seeing occurring. I often switch from Claude Code to Codex because I run out of tokens. In that swap, it’s often that huge swaths of code are rewritten without any formal equivalence being guaranteed. At work, many who are less comfortable with Agentic Engineering worry about the authenticity of what is being done. There’s no formal proof or even human guarantee that things have stayed the same. Blind trust in a machine is hard for many to have. Many tokens are wasted probably rewriting the exact same thing in different ways. That said, I love the creativity and innovation that a swarm of agents can provide.

---

I also went on a mini vision quest at this time thinking about how much linkers cause problems for software. After living through the “DLL hell” era of windows, and fighting many an early days of Redhat and Gentoo Linux library issues, I’ve kinda learned to despise dynamic linking. I’m also not a fan of giant static binaries, but I love the Docker “layer” style of software.

---

Ok, so at this point, my ADHD mind started to forget what my core purpose was, but then it all came together as I zoomed out. At the end of the day I wanted to focus my mind on a DSL, for agents, that could create verifiable graphs and subgraphs, that ultimately could be modified with precision and certainty. And cryptographically confirming this all was mostly icing on the cake; which, is ironic because that’s probably the most economically viable and useful part of this exercise, an end to end auditable system and software. And even though I work in legal AI compliance and this would be a huge benefit to the work I do there, for this side project, it’s just a cute way for Agents to express themselves in uncertain code that slowly but surely becomes more and more deterministic to create hopefully better software.

---

So from that, Conclave was born. Despite a Computer Science degree many years ago, my focus has always been more on Statistics, ML, and AI, so while I wish I could say I just whipped up a linter and got to work, I very heavily needed AI coding agent support to make my dream a reality and prove that I could get to a working language. The end result, a symbiotic dance with AI coding, actually built up backwards from the artifact generation to the DSL, is how I got to the 0.1 version you see today.

---

If anything, my scope is probably already too big. I might focus on its use for AI auditable pipelines in legal context as that’s where my day job interests intersect. Or I might just use it to nerd out on things like ASTs and Plan Generation and take that 400 level Programming Languages course I never got the chance to do at University. Or maybe this is it, I get distracted and never work on this again… But one day, when someone writes an LLM native language, if it uses even a touch of a topic I did here, I can hope, in my heart, I was some training set or tiny LLM network weight contribution to the future of software.

---

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

It doesn't query the cloud for instructions.
It doesn't scrape a live feed for context.

Run it anywhere and it stands on its own.

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
