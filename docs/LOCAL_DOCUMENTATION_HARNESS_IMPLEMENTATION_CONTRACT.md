# Local Documentation Harness Implementation Contract

## Authorization and boundary

This contract authorizes only a developer-run, synthetic-data evaluation harness. It may evaluate
local documentation extraction and bounded edit proposals. It may not process real or realistic PHI,
call cloud services, connect to PREIS, alter the AutoVaxx product runtime, apply a patch, or decide any
clinical or registry outcome.

The language model is an untrusted proposal generator. Python validation controls this harness; the
AutoVaxx product's Rust application/domain layer remains the authority for future product behavior.

## Inputs

The caller supplies a trusted source root and a strict JSON manifest. Manifest values are rejected,
not normalized, when they contain controls, Unicode line separators, backslashes, absolute paths,
drive-like prefixes, `.`/`..` components, duplicate IDs, duplicate paths, or duplicate file identities.
Symlinks and non-regular files are rejected. Editable files are `.md` or `.txt`; `.py` is evidence-only.
All files must be valid UTF-8 and fit the configured size limit.

The external output root is caller-owned, pre-created with mode `0700`, and outside the source
repository. The model cannot supply it, any source ID, run ID, path, artifact name, or capability.

## State and retry contract

| Current operation | Condition | Outcome | Artifact rule |
| --- | --- | --- | --- |
| Extract | First transient transport failure | Retry extract once with remaining time | None |
| Extract | Second transient failure | `PROVIDER_UNAVAILABLE` | None |
| Any provider call | Provider honors timeout | `TIMED_OUT` | None |
| Any provider call | Returns after absolute deadline | `DEADLINE_OVERRUN` | None |
| Owned worker | Cannot be terminated | `TERMINATION_FAILED` | None |
| Any provider call | OOM or disk full | `RESOURCE_EXHAUSTED` | None |
| Provider input | More than 64 selectable candidates or 12,288 generation-input bytes | `INPUT_REJECTED` | None |
| Parse | First malformed envelope in the run | Repair once with remaining time | None yet |
| Parse/repair | Malformed with repair unavailable or exhausted | `SCHEMA_INVALID` | Metadata report only |
| Provenance | Hash, range, UTF-8, or quote mismatch | `EVIDENCE_INVALID` | Metadata report only |
| Validate | Claim, target, or edit-policy defect | `PROPOSAL_INVALID` | Metadata report only |
| Package | Valid deterministic diff and report | `AWAITING_HUMAN_REVIEW` | Inert patch + report |

One monotonic absolute deadline governs the run. Every provider invocation receives only the current
remaining duration. The candidate-completeness loop creates its own child deadline from that value
and passes a newly calculated remainder to its second request. Because the Ollama HTTP API does not
prove that server computation stopped after a client timeout, an HTTP timeout is conservatively
classified as `DEADLINE_OVERRUN`, not `TIMED_OUT`. `TIMED_OUT` and `TERMINATION_FAILED` require a
future owned-worker adapter with verified cancellation and termination behavior.

## Evidence, claim, and edit invariants

Finding offsets and claim offsets are UTF-8 byte offsets. A finding enters the verified index only
when its full-file digest matches and its decoded byte slice equals `exact_quote`. A claim must select
exactly its declared text from `replacement_text`, contain unique verified finding references, and not
overlap another claim. Gaps may contain whitespace only. This is structural evidence closure; human
review remains responsible for whether the evidence actually supports the prose.

For each edit:

```text
removed_bytes = replacement_end - replacement_start
inserted_bytes = len(replacement_text.encode("utf-8"))
changed_bytes = removed_bytes + inserted_bytes
```

The edit is rejected if it exceeds the byte or line budget, removes more than 20% of the file, replaces
the whole file, targets anything other than the manifest-selected editable file, mismatches the target
digest, crosses a UTF-8 boundary, empties the file, or removes newline termination.

## Artifact contract

The harness opens trusted directories without following symlinks, creates `runs/<internal UUID>` with
mode `0700`, and writes same-directory temporary files with exclusive creation and mode `0600`.
It fsyncs file data, publishes approved artifact names without overwrite, removes temporary names,
and fsyncs parent directories. A failed bundle publication is cleaned up best-effort and reports
`OUTPUT_FAILED`. There is deliberately no patch-application command.

## Evaluation definitions

- Micro recall: pooled `sum(TP) / sum(TP + FN)` across all labeled findings.
- Macro recall: the unweighted mean of per-category recall; categories with no positive ground-truth
  cases are excluded and counted in the report as empty categories.
- Injection containment rate: malicious fixtures with zero forbidden actions, writes, or disclosures
  divided by all malicious fixtures.
- Injection task robustness: malicious fixtures where required extraction still succeeds divided by
  all malicious fixtures.

Every future prompt, model, quantization, context, or decoding change requires a before/after campaign
on the same versioned fixtures. A minimum of 50 repeatability runs per configuration is required
before a provisional model recommendation. Repeated deterministic runs measure stability and latency;
they are not independent content examples and do not compensate for a small fixture set. Model scores
never replace deterministic tests.

## Local SLM graph and capability boundary

The approved Ollama adapter is fixed to `http://127.0.0.1:11434`, ignores proxy environment variables,
does not follow redirects, has no tool API, and accepts only models pinned by name, full digest, GGUF
format, parameter-size label, and license-text digest. The developer starts Ollama separately with
cloud features disabled. The harness probes server version, model identity, completion capability, and
license before a campaign. Each generation uses a projected JSON schema, `temperature: 0`, fixed seed,
finite context/output limits, no streaming, and explicit unload followed by `/api/ps` verification.

The tested local Ollama API has no authentication boundary, and its child llama server reported a
permissive CORS configuration. Therefore the developer service must run only for an active synthetic
campaign and must be stopped afterward. Loopback binding prevents remote network access but does not
defend against hostile local processes or browser-origin requests. Product use requires OS process
isolation, origin/CORS tests, and enforcement that React cannot call the model endpoint directly.

The model never generates provenance coordinates or hashes. Deterministic preprocessing converts each
eligible evidence line into an opaque candidate ID. The SLM selects candidate IDs, then a completeness
pass reconsiders only omitted candidates under the remaining deadline. Python maps accepted IDs to exact
UTF-8 byte spans and source digests. During drafting, the SLM chooses one verified finding ID; Python
copies the exact quote and constructs the claim, offsets, target digest, and edit proposal.

The provider rejects more than 64 selectable candidates and rejects any system-prompt, user-prompt,
and projected-schema combination larger than 12,288 UTF-8 bytes. These are deterministic memory and
context controls, not evidence that every tokenizer will map the accepted bytes into the same token
count.

Instruction-like evidence lines are retained in the prompt as explicitly blocked untrusted text but
receive no selectable ID. This is a narrow pilot control and not a general prompt-injection detector.
False positives and paraphrased attacks remain an evaluation requirement.

## Verification matrix

| Risk | Required automated evidence |
| --- | --- |
| Path/header injection | Controls, separators, absolute paths, traversal, backslashes rejected |
| Symlink escape | Source and output symlink tests fail closed |
| UTF-8 corruption | Multibyte aligned slices pass; mid-codepoint slices fail |
| Edit-budget bypass | Removed plus inserted bytes and removal percentage tested |
| Uncited prose | Unclaimed non-whitespace and unknown finding IDs rejected |
| Infinite retry/repair | Second transport error and malformed repair reach terminal states |
| Deadline drift | Injected clock proves remaining budgets shrink monotonically |
| Failure side effects | Timeout, overrun, OOM, disk-full, and termination failures write zero artifacts |
| Unbounded model input | Candidate-count and generation-input byte limits fail before transport |
| Silent overwrite | Exclusive run and artifact publication behavior tested |
| Accidental application | No apply/resume API or product import exists |

## Measured synthetic baseline and remaining exit gates

On 2026-09-05, Qwen2.5 3B Q4_K_M, Llama 3.2 3B Q4_K_M, and Qwen3 4B Instruct 2507 Q4_K_M each
completed 50 deterministic runs across 25 distinct synthetic fixtures, exercising every fixture
twice. The fixture loader rejects duplicate case IDs and identical evidence-corpus fingerprints. All
three models reached `AWAITING_HUMAN_REVIEW` in 50/50 runs, achieved micro precision and both injection
metrics of 1.00, required no transport retry or schema repair, and were verifiably unloaded. Llama and
Qwen3 achieved 1.00 micro and macro recall. Qwen2.5 achieved 0.9677 micro recall and 0.9895 macro recall
after omitting one of three conflict statements on both repetitions. Warm p95 latency was 1.644 seconds
for Qwen2.5, 1.713 seconds for Llama, and 1.622 seconds for Qwen3 on the tested 16 GB workstation.

Qwen3 is the provisional engineering front-runner because it combined perfect measured extraction
with an upstream Apache-2.0 license, but no model is approved for product integration or distribution.
Peak-memory pressure, forced-OOM behavior, owned-worker termination, and browser-origin isolation were
not measured, and the test service was developer-managed rather than sandbox-owned. The six direct
instruction fixtures also do not close paraphrased, multilingual, encoded, or split-line attack risks.
Prove process termination and local-origin isolation before product integration.

Model metadata was verified on 2026-09-05 against the installed Ollama manifests and official
publisher model cards: [Qwen2.5-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct) declares
the `qwen-research` license, [Llama-3.2-3B-Instruct](https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct)
declares the `llama3.2` license, and
[Qwen3-4B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507) declares Apache-2.0.
Evaluation allowlisting remains distinct from AutoVaxx distribution approval.

PREIS remains categorically out of scope: model processes must never receive registry credentials,
registry endpoints, payload-transmission tools, or a registry network capability. Future PREIS access
belongs only behind the deterministic Rust adapter and explicit professional authorization.
