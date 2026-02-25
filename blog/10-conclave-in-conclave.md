# Conclave in Conclave

*February 2026*

Let me be upfront: this is not a self-hosting compiler.

Conclave is not writing Rust. The Conclave runtime is not lowering itself.
The DSL does not parse the DSL.

What happened today is smaller than that — and in some ways more interesting.

---

## What Actually Happened

I wrote a Conclave goal called `Package`. Its job: take a list of `.conclave`
source files and lower each one to Plan IR.

```conclave
version 0.1;

capability conclave_lower: conclave_lower(String) -> Json;
intrinsic assemble_json: assemble_json(List<Json>) -> Json;

goal Package(source_files: List<String>) -> Json {
  want {
    map source_files as file {
      emit conclave_lower(file);
    }
    return assemble_json(collected);
  }

  constraints {
    determinism.mode == "sealed_replay";
    rate_limit(conclave_lower) <= 4 req/s;
    scheduler.max_inflight <= 2;
  }
}
```

The `conclave_lower` capability is a Python script. It receives a file path
on stdin (as JSON), shells out to `conclave lower <path>`, and returns the
Plan IR JSON. It is content-addressed, sealed into the manifest, and bound
to the program's canonical hash.

Then I ran the whole pipeline:

```
conclave lower examples/package.conclave --url-count 3 > package_plan.json
conclave seal --plan package_plan.json --manifest package_manifest_template.json \
              --out package_manifest.json
conclave pack --runtime target/debug/conclave \
              --plan package_plan.json \
              --manifest package_manifest.json \
              -o package.bin
conclave run package.bin --mode live \
  --urls "examples/package.conclave,\
          crates/conclave-lang/tests/fixtures/summarize_urls/source.conclave,\
          examples/package.conclave"
```

And it produced this trace:

```
DISPATCH sha256:bc3b3f... t=0       # lower package.conclave
DISPATCH sha256:1c9c82... t=0       # lower summarize_urls/source.conclave
COMPLETE sha256:bc3b3f... t=4
COMPLETE sha256:1c9c82... t=5
DISPATCH sha256:8d4da7... t=1000    # lower package.conclave (rate limited)
COMPLETE sha256:8d4da7... t=1004
DISPATCH sha256:68bd39... t=1004    # assemble_json
COMPLETE sha256:68bd39... t=1008
```

The third call lands at t=1000. Not because of any sleep. Because the scheduler
enforces `rate_limit(conclave_lower) <= 4 req/s` — two tokens per window — and
holds the third dispatch until the window resets.

Conclave scheduled the lowering of Conclave source files.
The scheduler enforced policies about its own tooling.
The trace is deterministic.

---

## Why This Is a Meaningful Milestone

When a language can orchestrate a program that processes programs in that
language, something real has happened.

The Conclave runtime does not know that `conclave lower` is its own compiler.
It sees a capability binary: content-addressed, sealed, deterministic profile
`replayable`. It dispatches it like any other subprocess. The rate limiter does
not care what the subprocess does.

But we care. Because the artifact — `package.bin` — is sealed. The capability
hash pins the exact Python script that invokes `conclave lower`. The plan IR
hash pins the exact topology: three map nodes, one aggregate, three edges. The
manifest hash pins the scheduler policy, the rate limit, the determinism mode.

If you rebuild the binary with a different version of the lowerer, you get a
different capability hash. The old `package.bin` refuses to run it. The seal
is the record.

---

## What Makes This Different from "Just Running a Subprocess"

Any shell script can call another program.

What Conclave adds:

**Scheduling.** The map runs with bounded parallelism. The rate limit is
enforced by the virtual clock. `max_inflight = 2` means two simultaneous
`conclave lower` invocations at most. This is a policy, not a hope.

**Content addressing.** The capability that lowered your files today is pinned.
Someone can verify which version of the lowerer ran, what it produced, and
whether the trace matches. You cannot swap the binary without breaking the seal.

**Deterministic dispatch order.** Run the same artifact twice on the same
inputs: the same nodes dispatch in the same order. The trace hash may differ
(real subprocess time varies), but the event sequence does not.

**Composition.** The output of `Package` — a JSON array of Plan IRs — can
become the input to another Conclave goal. Build a pipeline where Conclave
lowers, seals, validates, and packs programs, all within the same deterministic
execution model.

---

## What This Is Not

It is not self-hosting. `conclave lower` is not written in Conclave. The
runtime is Rust. The scheduler is Rust. The capability binary is Python
wrapping Rust.

The recursion is one level deep: a Conclave plan that calls the Conclave
lowerer as a capability. We did not write a Conclave interpreter in Conclave.

That is fine. v0.1 set out to prove that the same source plus the same manifest
produces the same trace. v0.2 added conditionals, folds, and imported goals.
Today's milestone is narrower: that the infrastructure is general enough that
the tooling can be a first-class capability inside a plan.

---

## What Comes Next

The honest next step is module publishing. Today, `conclave module publish` can
register a lowered Plan IR in a local content-addressed cache. A second goal
can import it by hash. But that second goal does not yet know how to call the
imported goal's result as a subgraph.

When that works, a Conclave program will be able to import and invoke another
Conclave program's Plan IR — not by filename, but by content hash. That is
closer to the self-referential sense people have in mind when they say "language
compiles itself."

Today was a step toward it. The dispatch order is correct. The seal is valid.
The rate limiter counted correctly.

That is enough for today.
