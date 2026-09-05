# ADR-0010: Use a Private-IPC Owned Worker for Patient-Bearing Local AI

## Status

Accepted for planning only on 2026-09-05. This decision supersedes only the patient-bearing runtime
topology in Foundation Decision 13 and Architecture ADR-006. It does not authorize Phase 2 runtime
implementation, real PHI, a model or binary distribution, clinical reasoning, PREIS access, or a
production deployment.

Ollama remains approved only as an ephemeral, synthetic developer-evaluation service. If the owned
worker cannot satisfy the Windows isolation and performance gates below, local AI remains unavailable
in the product until a new architecture and threat review is approved.

## Context

The synthetic documentation campaign established that a small local model can perform bounded ID
selection inside a deterministic graph. It did not establish a product security boundary. The tested
Ollama API is an unauthenticated HTTP service, and Ollama permits requests from local-address browser
origins by default. Loopback prevents remote routing but does not authenticate the AutoVaxx process,
exclude other local processes, prove cancellation, or prevent a compromised webview or browser origin
from attempting to reach the service.

The original foundation decision selected a separately provisioned Ollama service for the first
product adapter. That decision predated the measured CORS behavior and owned-termination gap. Keeping
it for patient-bearing inference would make the facility-managed service, its configuration, and its
local HTTP surface part of the PHI boundary while AutoVaxx could neither authenticate requests nor
terminate overrun computation.

Windows Job Objects can constrain and terminate a process tree, but Microsoft explicitly treats
security limits as per-process controls rather than a Job Object property. AppContainer supplies the
separate least-privilege process, filesystem, and network isolation boundary. Both mechanisms are
needed: AppContainer for authority and Job Objects for lifecycle and resource control.

## Decision

### Runtime topology

Patient-bearing local inference uses an AutoVaxx-owned worker process supervised by the Rust
application layer:

```text
React webview
  -> narrow Tauri command
  -> Rust authorization + assist-session graph
  -> bounded inherited stdin/stdout pipes
  -> AppContainer-isolated inference worker
  -> read-only approved runtime + model files

Worker has no listening socket, network capability, database handle, secret-store handle,
registry/PREIS configuration, application command channel, or arbitrary tool interface.
```

The initial implementation candidate is a minimal worker linked to or wrapping a pinned llama.cpp
runtime because llama.cpp supports GGUF models and grammar-constrained output without requiring its
HTTP server. That is a candidate, not an approval: the binary, bindings, model artifact, license,
packaging, GPU behavior, and patch process require a separate implementation contract and evidence.

An externally provisioned or HTTP-listening model service is not a patient-bearing MVP path. A future
facility inference service would require a new ADR, mutual process/service authentication, encrypted
transport where applicable, an allowlisted identity and destination, facility operations ownership,
and equivalent cancellation and data-retention controls.

### Rust supervisor boundary

Rust remains the only control plane. It must:

- create the worker suspended with its AppContainer security attributes already applied, assign the
  Job Object before resuming it, and fail closed if any control cannot be verified;
- configure the Windows AppContainer with no network capabilities and access only to the exact
  read-only, hash-verified runtime/model package needed for the approved worker;
- place the complete worker process tree in an unnamed Job Object with no breakaway flags,
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, explicit job-memory and active-process limits, and completion
  monitoring;
- inherit only the required standard-input, standard-output, and explicitly approved diagnostic
  handles; command arguments, environment variables, filenames, and process titles contain no PHI;
- construct a minimal environment block with no proxy, cloud, registry, credential, user-profile, or
  developer-tool configuration;
- send one versioned, length-prefixed request at a time over anonymous pipes, enforce total and field
  byte limits before writing, and reject unsolicited, oversized, trailing, or malformed frames;
- treat all worker output as untrusted and re-run schema, encoding, provenance, allowlist, and
  deterministic domain validation in Rust;
- scope a worker to one authenticated assist session and one patient/encounter context; terminate and
  verify exit on context change, review completion, cancellation, logout, lock, application exit, or
  policy failure; and
- retain only the minimum approved non-content provenance. It never records rendered prompts,
  transcripts, responses, source spans, patient content, or worker standard output/error in
  operational logs.

The worker receives no writable filesystem path by default. If the selected runtime cannot operate
without a cache or scratch path, product integration remains blocked until a separate threat review
defines an isolated, quota-bounded, encrypted-or-content-free location and proves cleanup on every
terminal path. Convenience access to the user profile, general temporary directory, model-download
cache, or AutoVaxx application-data directory is prohibited.

### Deadline, failure, and restart policy

All work shares the Rust graph's single monotonic absolute deadline. Retrying or structural repair
never extends it.

- `TIMED_OUT` means cooperative cancellation completed and worker exit was observed before the hard
  termination grace period ended.
- `HARD_TERMINATED` means Rust terminated the Job Object after the deadline and observed the complete
  process tree exit.
- `PROCESS_TERMINATION_FAILED` means Rust could not prove complete exit. Assistance is disabled until a clean
  application restart and no automatic worker restart is allowed in the active patient session.
- `OUT_OF_MEMORY`, `RESOURCE_LIMIT`, and isolated worker `DISK_FULL` bypass retry and repair and return
  directly to manual documentation. Unknown/shared-volume exhaustion remains the blocking
  `PERSISTENCE_FULL` condition.
- A worker crash, invalid frame, model mismatch, sandbox mismatch, or capability mismatch produces no
  clinical mutation and routes to typed policy denial or manual fallback.

Manual documentation remains available unless clinical persistence itself is unhealthy. No provider
failure may weaken authentication, deterministic validation, human review, or audit requirements.

### Browser and network boundary

No model endpoint is added to the Tauri CSP, capabilities, frontend code, or webview. Product builds
must have no inference listening port at rest or during an assist session. Tests must attempt access
from the privileged webview, an ordinary browser origin, another same-user process, DNS, loopback, LAN,
and internet destinations while the worker is active. The expected result is no model endpoint to
reach and no worker network capability.

The synthetic Python/Ollama harness remains outside the product and accepts synthetic data only. Its
loopback, proxy, digest, and CORS controls are evaluation evidence, not product controls.

### PREIS and application authority

PREIS access is categorically excluded from the worker. The worker never receives registry endpoints,
profiles, credentials, payloads, acknowledgements, transmission tools, or network capability. PREIS
validation, rendering, authorization, transmission, and acknowledgement handling remain deterministic
Rust adapter responsibilities under their separate phase gates.

The worker can return proposals only. It cannot read or write SQLite, call Tauri commands, alter an
encounter state, accept a proposal, resolve a warning, confirm administration, finalize/correct/void a
record, authorize export, or transmit data.

## Acceptance gates before product wiring

All evidence uses clearly synthetic data on representative Windows 11 x64 hardware.

| Gate | Required evidence |
| --- | --- |
| Package identity | Signed/hashed worker and model package; exact runtime, model, quantization, license, prompt-template, schema, and decoding identifiers |
| Pre-execution containment | Worker is created suspended inside the AppContainer; Job Object assignment, handle list, environment, and file grants are verified before resume |
| Process-tree ownership | An adversarial helper attempts child creation and breakaway; every permitted descendant remains in the job and exits on close/termination |
| Network denial | Worker attempts DNS, loopback, LAN, internet, and proxy access; OS enforcement denies each path and packet/process evidence shows no worker egress |
| Filesystem and credential denial | Worker attempts AutoVaxx data, user profile, general temp, registry, credential store, and unrelated model files; every access is denied |
| Browser-origin isolation | Tauri webview and external-browser fixtures cannot discover or call an inference endpoint; CSP/capability regression checks remain closed |
| IPC confinement | Only allowlisted handles are inherited; malformed, oversized, duplicate, trailing, stalled, and unsolicited frames fail closed |
| Deadline enforcement | Injected-clock tests prove shrinking budgets; cooperative timeout, forced kill, child-process kill, and failed-termination states are distinct |
| Resource exhaustion | Peak committed memory is measured; job-memory limit, forced OOM, CPU pressure, and worker-only disk-full route to bounded manual fallback |
| Lifecycle cleanup | Completion, rejection, cancellation, timeout, crash, logout, lock, context change, and app exit leave no worker or patient-bearing artifact |
| Authority isolation | Forged worker output cannot bypass Rust schema/provenance validation, authorization, expected revisions, human review, or clinical state rules |
| Operational evidence | Process arguments, environment, logs, crash evidence, support bundles, temp locations, and installer artifacts contain no synthetic PHI markers or secrets |
| Performance | Cold/warm latency, peak memory, time-to-fallback, and manual-form availability meet approved thresholds without weakening a control |

Passing model-quality evaluation does not satisfy any isolation gate. Passing isolation gates does not
approve the model, clinical scope, real PHI, or PREIS transmission.

## Consequences

- Rust can enforce and observe provider lifetime instead of trusting a facility-managed HTTP service.
- Removing the listening socket eliminates the current Ollama local-API/CORS attack surface from the
  product topology.
- AppContainer and explicit file grants reduce worker authority; the Job Object bounds resource use
  and makes process-tree cleanup testable.
- Per-assist-session startup may increase cold latency and memory churn. Those costs are accepted until
  representative measurements justify a safer pooling design.
- Packaging and patch ownership move into AutoVaxx's release boundary. Binary/model provenance,
  installer size, GPU compatibility, update cadence, and support obligations become release gates.
- AppContainer compatibility with the selected runtime/GPU stack is unproven. Failure keeps local AI
  disabled; it does not justify falling back to an unauthenticated loopback service.
- A compromised privileged operating-system administrator remains outside the MVP protection claim.
  Deployment controls and risk acceptance are still required.

## Alternatives considered

### Separately provisioned Ollama over loopback

Retained for synthetic development evaluation and rejected for patient-bearing MVP use. The local API
does not authenticate callers, local browser origins are permitted by default, AutoVaxx does not own
the service lifecycle, and HTTP cancellation does not prove inference stopped.

### App-owned Ollama or llama-server on a random loopback port

Rejected as the default. A random port is not authentication, a bearer token would add secret
distribution without removing the local listener, and third-party server/CORS/tool surfaces enlarge
the trusted boundary. This may be reconsidered only with native authenticated transport and equivalent
OS isolation.

### In-process inference library

Deferred. It removes IPC and listener exposure but places native parser/model crashes, memory
exhaustion, and cancellation inside the trusted AutoVaxx process, undermining deterministic manual
fallback and independent hard termination.

### No local AI

Always remains the safe operational fallback. The deterministic manual workflow must be complete and
testable without a model runtime.

## Implementation sequence requiring separate authorization

1. Build an adversarial synthetic worker that exercises process, handle, filesystem, network,
   cancellation, and resource boundaries without linking a model runtime.
2. Prove the AppContainer plus Job Object supervisor on representative Windows 11 x64 hardware.
3. Select and review a minimal llama.cpp-based worker interface; pin binary and model artifacts.
4. Re-run the approved extraction evaluation through the product worker protocol.
5. Run the complete acceptance matrix, threat review, dependency/license review, and usability latency
   study before proposing any patient-bearing enablement.

Each step is a separate implementation plan and PR. None is authorized by this ADR.

The [Synthetic Windows Worker Supervisor contract](../SYNTHETIC_WORKER_IMPLEMENTATION_CONTRACT.md)
specifies the proposed first experiment, its fixed protocol, native test matrix, evidence, and open
implementation approval. It also requires probing container-local storage rather than assuming that
AppContainer configuration satisfies the no-writable-storage objective.

## Primary sources

- [Ollama local API authentication](https://docs.ollama.com/api/authentication)
- [Ollama local-only and browser-origin configuration](https://docs.ollama.com/faq)
- [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [Microsoft AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation)
- [Microsoft Create Process in Sandbox APIs](https://learn.microsoft.com/en-us/windows/win32/secauthz/createprocessinsandbox)
- [Microsoft anonymous-pipe handle inheritance](https://learn.microsoft.com/en-us/windows/win32/ipc/pipe-handle-inheritance)
- [Tauri Content Security Policy](https://v2.tauri.app/security/csp/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [llama.cpp CLI and grammar-constrained output](https://github.com/ggml-org/llama.cpp)
