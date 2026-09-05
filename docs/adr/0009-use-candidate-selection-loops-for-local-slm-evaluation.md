# ADR-0009: Use Candidate-Selection Loops for Local SLM Evaluation

## Status

Accepted for the synthetic evaluation harness only on 2026-09-05. This does not approve either model
license, product integration, real PHI, clinical reasoning, PREIS access, or automated patch application.

## Deciders

AutoVaxx founder and implementation evaluator.

## Context

Directly asking a 3B model to emit exact quotes, SHA-256 digests, UTF-8 byte offsets, claims, and edit
coordinates failed closed. Qwen2.5 3B initially copied the JSON wrapper and invented provenance values.
After deterministic materialization was introduced, both evaluated 3B models still omitted one side of
a conflicting-source fixture and achieved only 0.80 micro recall.

The harness needs to test whether a small local model can contribute useful classification without
granting it authority over provenance, paths, filesystem writes, clinical outcomes, or registry access.

## Decision

Use an explicit bounded graph:

```text
manifest -> deterministic candidates -> SLM selection -> omitted-candidate completeness pass
         -> deterministic provenance -> SLM finding choice -> deterministic claim/edit/diff
         -> human review
```

- Python assigns candidate IDs and owns all hashes and UTF-8 byte offsets.
- A deterministic pilot filter makes instruction-like lines non-selectable while preserving them as
  labeled untrusted context for the model.
- The SLM selects IDs only. A second call reconsiders omitted candidates using the remaining absolute
  deadline; Python validates and unions the IDs.
- The SLM's drafting role is limited to choosing one verified finding ID. Python copies its exact quote
  and builds the structured proposal.
- The loopback Ollama client rejects redirects and proxy environment configuration and pins the model
  by full digest, GGUF format, parameter size, and license-text digest.
- More than 64 selectable candidates or 12,288 bytes of combined generation input fail closed before
  model transport.
- HTTP timeout is `DEADLINE_OVERRUN` because the server API does not prove cancellation. A truthful
  `TIMED_OUT` state requires an owned worker whose termination can be observed.
- The process has no PREIS credentials, endpoints, transmission tools, or product filesystem authority.

## Options Considered

- **Direct rich JSON from the SLM.** Rejected after measured provenance fabrication and schema-grammar
  incompatibility.
- **Move immediately to an 8B critic.** Deferred because the bounded completeness loop closed the
  observed recall gap on both 3B models with lower memory pressure.
- **Select every line deterministically.** Rejected because it would inflate false positives and cease
  to test model discrimination.
- **Adopt LangGraph.** Deferred; the two-call loop is small enough to keep explicit and auditable.

## Consequences

The initial four-case smoke suite showed perfect results for both 3B models. A subsequent 25-fixture,
50-run comparison found 1.00 recall and precision for Llama 3.2 3B and Qwen3 4B, while Qwen2.5 3B
repeatedly omitted one of three conflict statements and reached 0.9677 micro recall. All three retained
1.00 injection containment and task robustness on six direct-instruction fixtures. Warm p95 remained
below 1.72 seconds on the tested machine.

The result remains provisional. Deterministic repeats are correlated, the instruction filter is
bypassable, semantic entailment still requires human review, and model distribution has not been
approved. The tested Ollama API also has no authentication boundary, and its internal llama server
reported permissive CORS; loopback alone does not isolate it from hostile local processes or browser
origins.

## Follow-up Actions

- Add paraphrased, multilingual, encoded, and split-line injection attacks and benign imperative prose.
- Measure peak unified memory, forced OOM behavior, and an owned-worker kill path.
- Add OS process isolation and browser-origin/CORS tests; keep developer Ollama services ephemeral.
- Record explicit artifact/distribution approval before selecting the Apache-2.0 Qwen3 candidate.
- Keep product integration and PREIS connectivity behind separate architecture and authorization gates.
