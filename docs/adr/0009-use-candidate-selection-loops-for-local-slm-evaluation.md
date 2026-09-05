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

Both 3B models improved from 0.80 to 1.00 micro recall on the four-case smoke suite and subsequently
completed 50/50 repeatability runs with perfect measured recall, precision, injection containment, and
task robustness. The extra completeness call increased the conflicting-case latency, but warm p95
remained below 1.7 seconds for both models on the tested machine.

The result is not a production-quality model comparison. Four fixtures do not represent documentation
diversity, the instruction filter is bypassable, deterministic repeats are correlated, semantic
entailment still requires human review, and neither custom model license is approved for distribution.
The tested Ollama API also has no authentication boundary, and its internal llama server reported
permissive CORS; loopback alone does not isolate it from hostile local processes or browser origins.

## Follow-up Actions

- Expand to at least 25 independently authored cases across supported documentation categories.
- Add paraphrased, multilingual, encoded, and split-line injection attacks and benign imperative prose.
- Measure peak unified memory, forced OOM behavior, and an owned-worker kill path.
- Add OS process isolation and browser-origin/CORS tests; keep developer Ollama services ephemeral.
- Complete legal review of the Qwen Research and Llama 3.2 licenses before model selection.
- Keep product integration and PREIS connectivity behind separate architecture and authorization gates.
