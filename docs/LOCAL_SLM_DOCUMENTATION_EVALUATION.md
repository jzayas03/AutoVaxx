# Local SLM Documentation Evaluation

## Summary

The local SLM approach works for bounded documentation tasks. Neither model is ready for AutoVaxx
product selection, and PREIS access is categorically excluded from the SLM boundary.

*Note: This document is an informational evaluation summary and does not constitute durable
acceptance evidence. Model and license digests are versioned in the
[model approval manifest](../tools/doc-eval-harness/evaluation/model-approvals.json).*

## Results

These metrics represent 50 deterministic repeatability runs across four fixtures, not 50 independent
evaluations. Both models completed the evaluations without retries or schema repairs and were
verifiably unloaded afterward. Qwen also passed while deliberately invalid proxy variables were
configured, confirming the client ignored proxy settings and used its fixed loopback destination.

| Model | Human-review terminal | Micro recall | Micro precision | Injection containment | p50 | p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5 3B `Q4_K_M` | 50/50 | 1.00 | 1.00 | 1.00 | 1.043 s | 1.553 s |
| Llama 3.2 3B `Q4_K_M` | 50/50 | 1.00 | 1.00 | 1.00 | 1.098 s | 1.692 s |

## Hybrid Workflow

This explicit Python graph improved both models from 0.80 to 1.00 recall on the conflict fixture
without moving to an 8B model. The harness delegates to the SLM only for specific heuristic
operations:

1. Deterministically load and validate the manifest.
2. Deterministically extract, filter, and identify candidates.
3. **SLM selects candidate IDs.**
4. **SLM reconsiders omitted IDs in a bounded completeness pass.**
5. Deterministically construct hashes and UTF-8 provenance.
6. **SLM selects one verified finding for the draft.**
7. Deterministically construct claims, edits, validation, and the inert diff.
8. Stop for human review.

## Open Gates and Next Actions

Continue with 3B models and the explicit Python graph. An 8B critic is not justified by current
evidence. **Do not choose a production model yet** for the following reasons:

- **Insufficient test data:** Only four independent fixtures exist; the contract requires at least 25.
- **Licensing constraints:** Qwen has a slight measured latency advantage, but its official model
  repository uses the [Qwen Research License](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct/blob/main/LICENSE),
  which limits the grant to non-commercial purposes and directs commercial users to request a separate
  license. This makes it unsuitable as AutoVaxx's commercial default without new licensing authority.
  Llama 3.2 uses the custom
  [Llama 3.2 model and license](https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct), which includes
  redistribution and attribution conditions that require legal review but present a different
  commercial posture.
- **Next action:** Compare at least one genuinely permissively licensed small model.

## PREIS Boundary

The harness exposes no PREIS capability. The model itself is not the security boundary; deterministic
code, network policy, credentials, and process isolation are. The current local harness has:

- No PREIS endpoint, credentials, or transmission tool.
- No arbitrary HTTP, shell, or filesystem capability.
- A fixed `127.0.0.1:11434` Ollama destination.
- No cloud fallback.

**Future architecture:** PREIS communication must remain strictly in the deterministic Rust adapter,
with explicit professional authorization. The SLM must never receive registry credentials or
transmission capability.

**Security note:** Loopback alone is not production isolation. Ollama explicitly documents in its
[authentication documentation](https://docs.ollama.com/api/authentication) that its local API requires
no authentication. The temporary service was stopped after testing. Product integration still
requires OS process isolation, browser-origin/CORS tests, and an owned-worker termination path.

## Implemented Files

*Note: 52 tests passed; Ruff, strict mypy, dependency-lock verification, and the offline wheel/source
build passed. Reports use `0700` directories and `0600` files. Work remains uncommitted on
`feat/local-doc-eval-harness`.*

- [Implementation contract](LOCAL_DOCUMENTATION_HARNESS_IMPLEMENTATION_CONTRACT.md)
- [Candidate-selection ADR](adr/0009-use-candidate-selection-loops-for-local-slm-evaluation.md)
- [Harness instructions](../tools/doc-eval-harness/README.md)
- [Ollama adapter](../tools/doc-eval-harness/src/autovaxx_doc_harness/ollama.py)
- [Campaign runner](../tools/doc-eval-harness/src/autovaxx_doc_harness/campaign.py)
