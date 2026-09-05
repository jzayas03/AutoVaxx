# Local SLM Documentation Evaluation

## Summary

The local SLM approach works for bounded documentation tasks. Qwen3 4B is the provisional engineering
front-runner, but no evaluated model is approved for AutoVaxx product integration. PREIS access is
categorically excluded from the SLM boundary.

*Note: This document is an informational evaluation summary and does not constitute durable
acceptance evidence. Model and license digests are versioned in the
[model approval manifest](../tools/doc-eval-harness/evaluation/model-approvals.json).*

## Results

These metrics represent 50 deterministic repeatability runs across 25 distinct synthetic fixtures,
not 50 independent evaluations. Each model saw every fixture exactly twice. The fixture loader rejects
duplicate case IDs and renamed copies of the same evidence corpus. All three models completed without
transport retries or schema repairs, reached human review in 50/50 runs, and were verifiably unloaded.
All campaigns ran with deliberately invalid proxy variables, confirming the fixed client ignored
environment proxy settings and used its loopback destination.

| Model | Micro recall | Macro recall | Micro precision | Injection containment / robustness | p50 | p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5 3B `Q4_K_M` | 0.9677 | 0.9895 | 1.00 | 1.00 / 1.00 | 1.211 s | 1.644 s |
| Llama 3.2 3B `Q4_K_M` | 1.00 | 1.00 | 1.00 | 1.00 / 1.00 | 1.115 s | 1.713 s |
| Qwen3 4B Instruct 2507 `Q4_K_M` | 1.00 | 1.00 | 1.00 | 1.00 / 1.00 | 1.330 s | 1.622 s |

Qwen2.5 omitted one of the three statements in `ThreeSourceConflict` on both repetitions. It still
cleared the provisional 0.95 recall threshold, but the repeatable miss makes it weaker than the two
perfect-recall candidates on this corpus. The report SHA-256 values are
`b6b9f43363c27a9bd3952e6cbfd210ed684a65d4cc824fbc08ceba0bfae0923b` (Qwen2.5),
`dc930fe7d182fe0a8690bb872afa80fd720d630b7ee09c2926ceb85f03417fb6` (Llama 3.2), and
`c562dd8400e4dc609b77f86cc84e64fa37aa41b7b0521207070797a28341ce4a` (Qwen3); full reports remain
ephemeral local evidence outside the repository.

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

Continue with the explicit Python graph. An 8B critic is not justified by current evidence.
**Do not choose a production model yet.** The 25-fixture and model-comparison gates are closed, but
these gates remain open:

- **Distribution approval:** [Qwen3-4B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507)
  declares Apache-2.0 and is the provisional front-runner. The evaluated local alias
  `qwen3-autovaxx:4b-instruct-2507-q4-k-m` maps to Ollama's upstream
  `qwen3:4b-instruct-2507-q4_K_M` artifact. Its exact digest and license-text digest are pinned, but
  `distribution_license_approved` remains false until AutoVaxx explicitly accepts the artifact.
- **Product isolation:** Peak unified-memory pressure, forced-OOM behavior, browser-origin isolation,
  and an owned-worker kill path remain unverified. A developer-managed Ollama service is not an
  acceptable product security boundary.
- **Adversarial breadth:** The six malicious fixtures cover direct instruction families. Paraphrased,
  multilingual, encoded, and split-line attacks plus benign imperative controls remain future work.
- **Next action:** Specify and test the Rust-owned Windows worker boundary before any product wiring.

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

*Note: 57 harness tests passed with Ruff and strict mypy. Campaign reports used `0700` directories and
`0600` files. The temporary Ollama service was stopped after testing. These changes are proposed on
`feat/local-doc-eval-25-fixtures`.*

- [Implementation contract](LOCAL_DOCUMENTATION_HARNESS_IMPLEMENTATION_CONTRACT.md)
- [Candidate-selection ADR](adr/0009-use-candidate-selection-loops-for-local-slm-evaluation.md)
- [Harness instructions](../tools/doc-eval-harness/README.md)
- [Ollama adapter](../tools/doc-eval-harness/src/autovaxx_doc_harness/ollama.py)
- [Campaign runner](../tools/doc-eval-harness/src/autovaxx_doc_harness/campaign.py)
