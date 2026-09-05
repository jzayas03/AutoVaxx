# ADR-0008: Isolate the Local Documentation Evaluation Harness

## Status

Accepted as the provider-independent foundation for a synthetic-only developer evaluation. It does
not authorize a product runtime, clinical use, real PHI, PREIS access, or patch application. The later
Ollama evaluation extension is recorded in ADR-0009.

## Context

AutoVaxx needs evidence about whether a small local language model can extract cited facts and
propose bounded documentation edits on a 16 GB workstation. `ASSIST_GRAPH_PLAN.md` already assigns
product orchestration, authorization, and I/O enforcement to Rust. Building a second Python product
runtime would create conflicting security boundaries before the model has passed an evaluation.

The evaluation itself handles untrusted documents and untrusted model output. A useful harness must
therefore prove deterministic provenance, output containment, finite retries, and a terminal human
review boundary independently of model quality.

## Decision

Build an installable Python developer tool under `tools/doc-eval-harness` with these boundaries:

- It accepts synthetic UTF-8 `.md`, `.txt`, and evidence-only `.py` files from a strict manifest.
- It exposes a narrow provider protocol so deterministic fakes and separately approved local adapters
  exercise the same state-machine boundary.
- Model envelopes are Pydantic-validated. Exact findings close over a full-file SHA-256 digest and
  UTF-8 byte range before they can enter the verified index.
- Drafts target one manifest-selected `.md` or `.txt` file. Every non-whitespace replacement byte is
  covered by a non-overlapping claim that references at least one verified finding. This proves
  structural traceability, not semantic entailment.
- Deterministic code enforces removed-plus-inserted byte limits, changed-line limits, a 20% removal
  cap, UTF-8 boundaries, and the pilot prohibition on whole-file replacement.
- One transient extraction retry and one shared structural repair are allowed under one injected,
  monotonic absolute deadline. OOM and disk-full failures are terminal and are never retried.
- A successful run publishes an inert unified diff and a metadata report, then stops at
  `AWAITING_HUMAN_REVIEW`. There is no resume or apply command.
- Artifact roots must already exist outside the source repository. Run directories and files use
  descriptor-relative, no-follow operations; a bundle is published without overwriting names.
- Timeout, deadline-overrun, provider-unavailable, resource-exhaustion, and termination failures
  publish no artifacts. Schema, provenance, and proposal failures may publish a metadata-only report
  containing digests and error classifications, never source or model text.

## Consequences

The control suite can test failure behavior without trusting a model server. A separately approved
Ollama adapter may implement the provider protocol, while the product continues to use the Rust
provider boundary. Review patches cannot mutate the repository.

The harness does not prove correctness of proposed prose, clinical safety, HIPAA compliance, PREIS
conformance, model sandboxing, or production readiness. Descriptor-relative operations narrow the
artifact race surface, but OS sandboxing and process termination need a later capability probe before
an actual model is introduced.

## Alternatives Considered

- **Add the harness to the Rust product now.** Rejected because model capability is not established
  and Phase 2 runtime implementation remains gated.
- **Use LangGraph or another agent framework.** Deferred because this graph is small, explicit, and
  more auditable as ordinary typed code.
- **Let the model emit a patch directly.** Rejected because paths, provenance, budgets, and diff
  headers must remain deterministic.
- **Give the model PREIS or filesystem tools.** Rejected. External transmission and file mutation are
  outside the documentation-evaluation scope and must never be model capabilities.
