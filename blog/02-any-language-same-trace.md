# Same Result, Different Language: What Conclave's Multi-Language Capability Test Actually Shows

*We ran the same agent plan twice — once with a Rust fetch binary, once with a Python script. Here's what we learned.*

---

Last week we ran a test that sounds simple but proves something we think is important.

We took a Conclave execution plan — a 3-URL summarization job — and ran it twice: once using a Rust binary as the fetch capability, and once using a Python script. Same plan, same URLs, same scheduler settings, different language for the capability that does the actual network work.

Both runs completed successfully. The scheduler produced the same trace structure both times. The rate limiter behaved identically. And because the results were recorded in a content-addressed replay store, either run could be replayed deterministically later — with no network access at all.

Here's what that actually means, and why we think it matters.

---

## What a "Capability" Is

In Conclave, a *capability* is any unit of work that touches the outside world. Fetching a URL is a capability. Calling an API is a capability. Querying a database is a capability.

The key idea: capabilities are **separate binaries** with a defined interface. The runtime doesn't care how they're implemented. It just:

1. Writes the binary to a temporary file and makes it executable
2. Sends a JSON request on stdin
3. Reads a JSON response from stdout
4. Records the result

That's the entire protocol. One JSON line in, one JSON line out.

The request looks like this:

```json
{
  "capability": "fetch(Url)->Html",
  "inputs": { "url": "https://example.com" },
  "context": { "seed": 1337, "virtual_time": 0, "determinism_profile": "replayable" }
}
```

And the response:

```json
{
  "output": { "type": "Html", "data_b64": "PCFkb2N0eXBlIGh0bWw..." },
  "duration_ms": 213
}
```

That's it. Any language that can read stdin and write stdout can implement a Conclave capability.

---

## The Rust Version

The Rust capability binary is a ~100-line program that uses `ureq` for HTTP. It reads the JSON request, extracts `inputs.url`, makes the GET request, base64-encodes the response body, and prints the result.

We built it, computed its SHA-256 hash, and pinned that hash in the manifest:

```
conclave install-cap ./target/release/conclave-cap-fetch
→ installed: sha256:321c3a0129d2867501f341a022505c52c6751793fa1a20235a1135cc4b2cdf7b
```

That hash *is* the capability's identity. Not a name. Not a version string. The exact bytes of the binary, hashed. If the binary changes — even by one byte — the hash changes, the manifest hash changes, and the seal is broken.

---

## The Python Version

The Python capability is about 60 lines. It uses only the standard library — no dependencies. It has a shebang line at the top (`#!/usr/bin/env python3`) so the OS knows how to run it directly.

```python
#!/usr/bin/env python3
import base64, json, sys, time, urllib.request

def main():
    req = json.loads(sys.stdin.readline().strip())
    url = req.get("inputs", {}).get("url")
    t0 = time.perf_counter()
    with urllib.request.urlopen(url, timeout=30) as resp:
        body = resp.read()
    elapsed_ms = int((time.perf_counter() - t0) * 1000)
    print(json.dumps({
        "output": {"type": "Html", "data_b64": base64.b64encode(body).decode()},
        "duration_ms": elapsed_ms,
    }), flush=True)
```

Install it the same way:

```
conclave install-cap ./examples/cap_fetch.py
→ installed: sha256:3e82e6a9c582330ec4cb0726fe59889ace945b5ca09f20b63669965b2927d147
```

Different hash, different binary, same contract.

---

## The Execution Traces

Here are the actual traces from both runs against three real URLs:

**Rust binary:**
```
t=     0ms  DISPATCH   F1
t=     0ms  DISPATCH   F2
t=   214ms  COMPLETE   F1
t=   214ms  DISPATCH   E1
t=   229ms  COMPLETE   E1
t=   229ms  DISPATCH   S1
t=   314ms  COMPLETE   S1
t=   456ms  COMPLETE   F2
t=   456ms  DISPATCH   E2
t=   471ms  COMPLETE   E2
t=   471ms  DISPATCH   S2
t=   556ms  COMPLETE   S2
t=  1000ms  DISPATCH   F3
t=  1396ms  COMPLETE   F3
t=  1396ms  DISPATCH   E3
t=  1411ms  COMPLETE   E3
t=  1411ms  DISPATCH   S3
t=  1496ms  COMPLETE   S3
t=  1496ms  DISPATCH   A
t=  1500ms  COMPLETE   A
```

**Python script:**
```
t=     0ms  DISPATCH   F1
t=     0ms  DISPATCH   F2
t=   130ms  COMPLETE   F1
t=   130ms  DISPATCH   E1
t=   145ms  COMPLETE   E1
t=   145ms  DISPATCH   S1
t=   230ms  COMPLETE   S1
t=   367ms  COMPLETE   F2
t=   367ms  DISPATCH   E2
t=   382ms  COMPLETE   E2
t=   382ms  DISPATCH   S2
t=   467ms  COMPLETE   S2
t=  1000ms  DISPATCH   F3
t=  1428ms  COMPLETE   F3
t=  1428ms  DISPATCH   E3
t=  1443ms  COMPLETE   E3
t=  1443ms  DISPATCH   S3
t=  1528ms  COMPLETE   S3
t=  1528ms  DISPATCH   A
t=  1532ms  COMPLETE   A
```

The timing differs — Python vs. Rust, different network conditions — but the **structure is identical**. Every event happens in the same order. Every node follows the same dependency chain. And both runs produce the same final node count: 10.

---

## The Thing That Doesn't Change: F3 at t=1000ms

Look at both traces again. In both runs, F3 — the third fetch — dispatches at exactly t=1000ms.

This isn't a coincidence. It's the rate limiter.

The scheduler is configured to allow 2 fetch requests per 1000ms window. F1 and F2 use up the first window at t=0. F3 is ready, but the window is exhausted. The scheduler advances the virtual clock to t=1000ms — the start of the next window — and only then dispatches F3.

This happens the same way regardless of which capability binary is running. The rate limiter lives in the Conclave runtime, not in the capability. The capability just does the network work. The scheduler enforces the policy.

This separation is deliberate. Capabilities should be dumb. Policy belongs to the runtime.

---

## What Gets Stored (And How Replay Works)

After a live run, the results from each capability call are stored in a content-addressed replay store, keyed by capability signature and node ID. The next time you run the same artifact, if a replay entry exists, the capability binary is never invoked — the result is replayed directly.

This means:

- **First run**: live, network access, capability subprocess spawned for each fetch
- **All subsequent runs**: sealed replay, no network, no subprocess, identical output

The replay store entries are just bytes: the base64-encoded response body plus the measured duration. The duration is stored too, so the virtual clock advances by the same amount on replay — preserving timing behavior.

The whole execution is summarized by a `trace_hash`. If you replay it and the trace hash matches, you know the run was byte-for-byte identical to the original.

---

## Why Language Doesn't Matter (And Why That's the Point)

The test started as a curiosity — "could we write a Python cap?" — but it demonstrates something more fundamental.

Capabilities in Conclave are **content-addressed programs with a stable interface**. The runtime doesn't know or care whether the bytes it's given are a compiled binary, a Python script, a shell script, or anything else. It hashes the bytes, stores them, retrieves them by hash, makes them executable, and runs them.

This has a few consequences:

**You can write capabilities in whatever language makes sense.** Fast, low-latency fetch? Write it in Rust. Complex parsing logic where you want rich libraries? Python or Node. Interacting with a legacy system? Maybe a shell script wrapping an existing CLI. The interface is the contract, not the language.

**Upgrading a capability is explicit and auditable.** You don't "update" a capability. You install a new one — which gets a new hash. You update the manifest to point to the new hash. You reseal. The old artifact still works exactly as before. The new artifact is a distinct, versioned thing.

**The capability's identity is verifiable.** If someone gives you an artifact and claims it uses a specific fetch implementation, you can check the hash in the manifest against the binary in the capability store. If they match, it's the right binary. If they don't, the seal is broken and the runtime won't run it.

---

## What's Next

The current fetch capability is a demonstration. It fetches real URLs and records the results for replay. Future capabilities will handle more complex interactions: LLM calls, database queries, file I/O — each one sealed by hash, each one replayable.

The Python test wasn't just about Python. It was about showing that the architecture holds: the scheduler enforces policy, the capability does work, and the runtime ties them together with cryptographic guarantees.

Any language. Same trace structure. Verifiable results.

That's the thesis.

---

*Conclave is open source. The full example — plan IR, manifest, Python and Rust capabilities, conformance tests — is in the repository.*
