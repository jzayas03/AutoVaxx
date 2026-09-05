# Synthetic Windows Worker Supervisor — Implementation Contract

## Status and approval boundary

Draft for implementation approval, 2026-09-05. Based on main commit `9123d1b` and
[ADR-0010](adr/0010-use-a-private-ipc-owned-worker-for-patient-bearing-local-ai.md).
The owner authorized preparation of this contract. Implementation approval and Windows acceptance
are **OPEN**. Merging this document records the proposal; it does not approve execution of its plan.

The proposed deliverable is a standalone Rust supervisor with an adversarial synthetic helper. It
tests the process boundary before a model is introduced. It is independent of the product's Phase 2
delivery and cannot satisfy Phase 1 exit or authorize real PHI. No Tauri command, UI, clinical domain,
database, speech adapter, PREIS integration, prompt, model, or model runtime changes are included.

## Existing mechanisms and proposed layout

The existing `LocalAiProvider` has only `propose_fields(&str)` and its implementation returns
`ProviderUnavailable`. There is no product worker call site. Keep that behavior. The Python
documentation harness evaluates model selection; it does not own the proposed Windows process.

Use a standalone Cargo package under `tools/worker-isolation-probe/`, outside the application Cargo
package. Its library holds the protocol, state machine, and Windows supervisor; separate binaries
provide the test controller and adversarial helper. Integration tests invoke the real supervisor.
Keep Windows API calls inside one reviewed adapter with RAII handle ownership and explicit safety
comments. Non-Windows builds exercise only protocol/state tests and return `UNSUPPORTED_PLATFORM`
for process execution; they never substitute an unsandboxed child.

| Proposed area | Responsibility |
| --- | --- |
| `src/protocol.rs` | Bounded frame codec and strict request/response types |
| `src/supervisor.rs` | State transitions, absolute deadlines, cleanup, typed outcomes |
| `src/windows/` | AppContainer, token/ACL inspection, process/job ownership, cancellable pipe I/O |
| `src/bin/controller.rs` | Execute fixed synthetic cases; write content-free evidence |
| `src/bin/adversarial-worker.rs` | Fixed adversarial modes; no general shell or script runner |
| `tests/` | Portable transition tests and explicit Windows integration suite |
| `README.md`, `Cargo.lock` | Reproduction commands, threat assumptions, locked dependencies |

Prefer existing dependency families (`serde`, `serde_json`, `sha2`, `uuid`, `thiserror`) and a minimal
Microsoft Windows binding crate. Before adding them, record exact direct versions, enabled Windows
API features, transitive lock changes, licenses, and advisories in the implementation review.
Dependency approval is part of the separate implementation decision. Do not copy the Tauri stack,
introduce an async runtime by default, or design a general worker framework.

## Threat model and probe environment

Assume the helper may emit hostile bytes, stop reading, flood output, ignore cancellation, crash,
allocate memory, spawn children, or attempt access outside its grants. Treat its claimed success or
denial as untrusted. The controller and approved OS observation tools independently verify results.
The kernel, administrator, and supervisor are trusted for this experiment; resistance to a hostile
administrator or same-user process that can debug the supervisor is not established by private IPC.

Run only on an explicitly selected disposable Windows test account/VM containing no patient data or
real credentials. Create synthetic decoy files, registry keys, and credential entries owned by that
account. Denial probes target those decoys, not existing application databases or credential stores.
Network probes require a controlled test network and operator-owned receivers. They send fixed
non-content markers only; do not scan a facility network or contact arbitrary public hosts.

The controller owns a fresh run directory, package staging area, AppContainer identity, and evidence
manifest. Validate canonical paths, reject reparse points and unexpected ownership/ACLs, and confine
cleanup to objects recorded in that manifest. Cleanup must never recursively delete an unresolved
path or remove an existing shared AppContainer profile. No machine-wide ACL, firewall, loopback
exemption, or security-policy relaxation is permitted.

## Creation and authority contract

1. Validate the fixed helper mode, protocol policy, approved executable digest, local paths, and
   evidence destination before creating a process. Stage the helper in a controller-owned directory;
   prevent replacement from digest verification through process creation. Use an explicit executable
   path and minimal working directory; never resolve the helper or DLLs through a caller's `PATH`.
2. Create a run-specific AppContainer with zero declared network capabilities. Inventory its profile,
   registry, inherited access, system-library access, and shared package grants. Empty capabilities
   and an empty environment are not proof of complete filesystem or registry denial.
3. Create an unnamed Job Object with kill-on-last-handle-close, neither breakaway flag, a hard job
   committed-memory limit, an active-process limit, and completion monitoring. The worker must not
   inherit or duplicate the job handle. Validate nested-job behavior under the CI parent job.
4. Create separate request and response pipes plus a bounded, discard-only stderr channel. Inherit
   only worker-side handles via an explicit handle list; keep supervisor-side handles non-inheritable
   and close parent copies of worker-side handles promptly. A sentinel unrelated inheritable handle
   must fail to appear in the helper.
5. Create suspended using `STARTUPINFOEX` with AppContainer security capabilities and the explicit
   handle list. Assign the job and independently query the token, job membership, limits, and grants
   before resume. Prefer atomic job assignment through the process attribute job list where verified;
   otherwise test controller death during the create/assign window and require external test cleanup
   of any suspended orphan. Never send a request before all checks pass.
6. Resume only after successful verification. On any intermediate error, terminate/reap the created
   process and close handles. If cleanup cannot be proved, report a terminal cleanup failure.

No application/data/secret handles, user configuration, proxy settings, credentials, or general tool
interface enter the worker. The executable and fixed fixture package are read-only. Record required
Windows loader/system-library access separately; the ADR's minimal-file-access objective is not a
claim that a Win32 process accesses literally only its executable.

Probe writes into the AppContainer's own storage/registry as well as general temp, profile, package,
and decoy application paths. Any writable content sink is a failed no-retention gate, even if it is
private to the container and later deleted. The experiment may document that failure using synthetic
markers, but cannot convert it into an approved scratch directory. Mitigation or an ADR revision is
required before product work. Standard AppContainer compatibility and strict no-writable-storage
feasibility are open questions.

## Protocol and resource policy

Version 1 uses a four-byte unsigned little-endian payload length followed by strict UTF-8 JSON.
Reject zero length, lengths above 64 KiB, invalid UTF-8, unknown or duplicate fields, unknown versions,
and invalid enum values before processing. Check the length before allocating the body; partial
headers/bodies share the same deadline. Cap nesting at four levels and each string at 4 KiB.

A request contains `protocol_version`, controller-generated `run_id`, `request_id`, a fixed `case_id`,
and bounded synthetic fixture fields. Any decoy destination comes from the controller's trusted case
manifest. Worker messages contain those IDs, a fixed result code, and bounded numeric measurements.
They cannot choose paths, hostnames, follow-up commands, policy limits, or state transitions.
Exactly one request and one response are allowed per worker. Duplicate responses, trailing bytes,
early responses, mismatched IDs, and output after cancellation are rejected. Stderr is never logged;
drain/discard at most 64 KiB, then abort. Accepted evidence never embeds worker-controlled text.

Use dedicated pipe I/O threads so a blocked read or write cannot block the supervisor timer. The
implementation must define how pending synchronous operations are cancelled and how completion and
thread exit are observed. Cancellation API success is not completion. Unbounded `join`, waiting for
EOF while a descendant retains a pipe, and detaching a stuck reader are contract failures.
Closing the request pipe is the cooperative cancellation signal for this one-request protocol;
the helper must monitor EOF independently while running its test operation.

Initial **probe-only** policy: one worker at a time; no retries; 256 MiB total committed job memory;
one active process normally, two only for the explicit descendant test; 64 KiB per protocol frame;
5 s work budget, 250 ms cooperative cancellation reserve, 1 s exit-observation budget.
Each case records its exact limits. Memory allocation probes stop at an enforced OS limit and must
not exhaust the host. These values are engineering test inputs, not approved inference latency,
model-memory budgets, or clinical usability thresholds.

## Deadlines, terminal states, and evidence

Set work deadline `D = monotonic_start + work_budget` once. Request cancellation at `D - reserve`;
initiate hard termination no later than `D`. The bounded cleanup deadline `K = D + exit_budget` is
declared at startup and never extends work or accepts late results. Early cancellation or validation
failure shortens these limits. Setup, blocked I/O, validation, and all waits consume remaining time.
Use an injected clock for deterministic tests and real Windows time for process tests.

| Condition | Outcome | Required observation |
| --- | --- | --- |
| Valid response within work budget, normal exit and cleanup | `SUCCEEDED` | Response validated, process handles signaled, job active count zero, I/O threads exited |
| Deadline cancellation, cooperative exit by `D` | `TIMED_OUT` | Same exit/cleanup evidence; response discarded |
| User/context cancellation, cooperative exit within its shortened deadline | `CANCELLED` | Same exit/cleanup evidence; response discarded |
| Forced job termination, exit observed by `K` | `HARD_TERMINATED` | Full job exit and I/O cleanup verified; triggering reason retained |
| Exit cannot be established by `K` | `PROCESS_TERMINATION_FAILED` | Latched supervisor quarantine; no new run in that controller |
| Token, capability, package, handle, or job mismatch | `POLICY_DENIED` | Worker never receives a request; any created process reaped |
| Invalid or excess output, crash, memory/resource failure | `MALFORMED_OUTPUT`, `WORKER_CRASHED`, `OUT_OF_MEMORY`, or `RESOURCE_LIMIT` | No retry; cleanup verified or escalated to termination failure |
| Evidence persistence fails | `EVIDENCE_WRITE_FAILED` | Nonzero controller exit; no passing acceptance record |
| Unsupported OS | `UNSUPPORTED_PLATFORM` | No worker spawned; Windows acceptance remains open |

Keep trigger and cleanup outcome separate so forced termination does not erase an OOM or protocol
failure. `PROCESS_TERMINATION_FAILED` takes precedence when cleanup is uncertain. The future product
provider's `TIMEOUT` error maps to the probe's `TIMED_OUT`; this document does not rename that API.
Retain the job handle while querying exit. Do not infer whole-tree exit from root-process exit, EOF,
a worker acknowledgement, a single completion-port message, or a reusable PID. Cross-check retained
process handles and job accounting; supervisor crash tests use an independent observer.

No automatic worker restart occurs after any terminal result. Failure to terminate blocks further
launches until a fresh controller verifies the old run is gone; restarting alone is not proof.
Because the package has no clinical persistence, real disk-full testing uses only a quota-bounded
controller evidence volume. A fake worker disk-full result tests routing only and cannot prove real
filesystem exhaustion isolation. Never fill the host or application volume.

## Required verification matrix

Every case records `PASS`, `FAIL`, or `NOT_RUN`. Skips, missing observation, and unsupported native
controls must not yield a passing Windows acceptance status. Fault injection supplements native
tests; it never replaces them.

| ID | Test | Passing evidence |
| --- | --- | --- |
| W01 | Create failures at every ownership transition, including job assignment | No resumed uncontained helper; no surviving owned process/handle; no request sent on failure |
| W02 | Capability/token/ACL and package tampering, executable replacement/reparse attempts | Denial before request; known Windows library reads inventoried |
| W03 | Read/write decoys, container-local storage and registry, credential access | Forbidden access denied by OS and observed independently; unexpected writable sink fails gate |
| W04 | IPv4/IPv6 loopback, DNS, controlled LAN/egress, proxy and listener attempts | Receiver/process observations show denial; an unreachable receiver alone is insufficient; positive controls prove receiver readiness |
| W05 | Normal child, breakaway child, brokered process launch | Normal descendant remains in job; escape/broker attempts denied; root and descendants exit |
| W06 | Hostile frame lengths, UTF-8/JSON, IDs, duplicate/trailing data, floods | Typed failure, bounded allocation, no worker text in evidence, verified cleanup |
| W07 | Stop reading stdin, partial stdout, endless stderr, descendant retains pipe | Timer remains responsive; I/O cancellation completes and threads/children exit within cleanup bound |
| W08 | Cooperate, ignore cancellation, race deadline, fake termination failure | Correct distinct outcomes; no late success, retry, deadline extension, or restart after failed exit |
| W09 | Job memory/process-limit pressure and crash | Bounded host impact, correct trigger and cleanup record, no retry |
| W10 | Kill controller during setup and after resume; repeat 50 start/stop cycles | Independent observer finds no unexplained survivor, handle growth, or residual marker |
| W11 | Evidence disk full, interrupted publication, permission or cleanup failure | Nonzero exit and incomplete/failed evidence; never a success summary |
| W12 | Static Tauri packaging/CSP check and same-user handle/listener probe | Helper absent from app bundle; no new frontend capability/endpoint; non-admin handle access examined |

Real browser/webview integration, authenticated session events, manual-form responsiveness, accepted
clinical proposals, GPU/model performance, and installer signing remain later product acceptance
tests. W12 cannot be reported as completion of those ADR-0010 gates. The standalone controller tests
equivalent cancellation events only. Run Windows native cases on CI when supported and separately on
representative Windows 11 x64; `windows-latest` success alone is not Windows 11 workstation evidence.

## Durable evidence and delivery sequence

The controller writes a strict, versioned report with source commit/dirty status, binary and policy
digests, compiler/OS build/architecture, synthetic case IDs, limits, monotonic durations, observed
exit/cleanup status, case results, and observer type. No raw pipes, prompts, decoy values, credentials,
absolute user paths, or uncontrolled OS error strings enter reports. Evidence digests identify the
complete retained report and its support files; digests alone do not preserve missing evidence.

Stage reports within a fresh controller-owned directory with restrictive Windows ACLs. Flush and
publish the final manifest only after cleanup and complete writes; a partial bundle is incomplete.
Record retention location, expiry, and named reviewer in the acceptance record. Keep a sanitized
summary in the repository and complete synthetic evidence in an access-controlled durable location
chosen by the owner; CI artifacts without an explicit expiry/retention decision are temporary.

1. Approve this standalone experiment scope, exact dependency delta, and disposable test environment.
2. Deliver one implementation PR containing the package, portable tests, Windows native probes, and
   explicit CI invocation. Extend the required-job aggregator if a new job is added; preserve existing
   application gates. No network denial probe runs on a developer workstation by default.
3. Execute native tests, repair defects, retain evidence, and document unresolved gate failures.
   Missing Windows 11 access yields `NOT_RUN`, not acceptance.
4. Review results with the owner. Only after acceptance propose a separate runtime/model integration
   contract; repeat the isolation gates with the actual runtime before any product enablement.

Definition of done for the experiment: package and meaningful tests delivered, required checks green,
all intended native cases executed with durable evidence, no unresolved containment/cleanup failure,
and a named Windows 11 reviewer signs the acceptance record. Model choice, real PHI authorization,
clinical scope, and PREIS remain outside that acceptance.

## Sources and interpretation

Primary sources checked 2026-09-05. Numerical probe budgets and the test policy above are AutoVaxx
design proposals, not vendor guarantees.

- [Process creation attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute): AppContainer capabilities, explicit handle inheritance, and job-list attributes.
- [AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation): authority isolation does not justify assuming all container-local storage is inaccessible.
- [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects): lifecycle/resource controls, nested jobs, breakaway behavior, and notification limitations.
- [CancelSynchronousIo](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelsynchronousio): cancellation initiation and operation completion are separate observations.
