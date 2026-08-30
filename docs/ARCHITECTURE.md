# AutoVaxx Architecture

**Status:** Approved Phase 1 foundation architecture; Phase 1 exit review pending

**Date:** 2026-08-30

**Decision scope:** Initial single-computer/single-facility product with an intentional path to multiple workstations

## 1. Architectural goals

- Complete the vaccination documentation workflow without internet connectivity.
- Keep PHI on the user's computer unless an authenticated user explicitly authorizes a defined external transmission.
- Make clinical behavior deterministic, versioned, explainable, and testable.
- Treat AI and speech as optional local assistants, never as authorities.
- Preserve the full clinical and audit history through append-only revisions.
- Keep the MVP deployable as one desktop process plus explicitly managed local model processes.
- Establish boundaries that permit later multi-workstation deployment without building distributed infrastructure now.

## 2. System context

```text
                         explicit, authorized transmission only
                    +----------------------------------------------+
                    |                                              v
+-------------------+--------------------+               +------------------+
| AutoVaxx desktop on one workstation   |               | PREIS / approved |
|                                        |               | external system  |
| React UI -> Tauri IPC -> Rust core      |               +------------------+
|                         |               |
|                         +-> SQLite      |
|                         +-> OS keychain |
|                         +-> local files |
|                         +-> Ollama      |
|                         +-> whisper.cpp |
+----------------------------------------+

No cloud AI. No background PHI synchronization. Core workflow remains local.
```

## 3. Logical layers

```text
+-------------------------------------------------------------------+
| React + TypeScript                                                |
| workflow presentation, forms, review UI, accessibility            |
+---------------------------- Tauri commands/events -----------------+
| Rust application layer                                            |
| use cases, authorization, transactions, workflow orchestration     |
+----------------------+----------------------+----------------------+
| Domain               | Ports                | Deterministic rules  |
| entities/value types | repositories         | screening            |
| state machines       | AI/speech/export     | documentation        |
| corrections          | clock/identity       | registry readiness   |
+----------------------+----------------------+----------------------+
| Adapters                                                          |
| SQLite | OS keychain | Ollama | whisper.cpp | files | PREIS/HL7    |
+-------------------------------------------------------------------+
```

Dependency direction points inward: adapters depend on ports and domain types. The domain does not import Tauri, SQLite, Ollama, whisper.cpp, HL7, or PREIS-specific code.

### React frontend

Responsibilities:

- Render workflow state and structured forms.
- Capture user intent and show validation/rule results.
- Display AI proposals separately from accepted clinical fields.
- Avoid local persistence other than non-PHI presentation preferences.

The frontend must not execute SQL, hold database keys, make authorization decisions, call model endpoints directly, or contact external registries.

### Tauri command boundary

Expose narrow, task-oriented commands such as `save_screening_draft`, `evaluate_encounter`, `confirm_administration`, and `prepare_registry_export`, not generic database or filesystem access. Each command validates input shape, authenticates the session, authorizes the action, invokes one application use case, and returns a typed result with a correlation identifier that contains no PHI.

### Rust application and domain

The Rust core is the trusted computing boundary for:

- Authentication and authorization.
- Workflow state transitions.
- Transaction and audit coordination.
- Deterministic clinical/documentation/registry validation.
- Canonical data model and correction semantics.
- Network destination policy and transmission authorization.
- AI/speech provider invocation and output validation.

### Adapter boundary

All environment-specific dependencies implement explicit Rust traits. Initial conceptual ports:

```rust
trait PatientRepository { /* typed patient and revision operations */ }
trait EncounterRepository { /* workflow and clinical revisions */ }
trait AuditRepository { /* append and verify */ }
trait ClinicalRuleEngine { /* deterministic evaluation */ }
trait LanguageModelProvider { /* structured proposal only */ }
trait SpeechToTextProvider { /* local transcript only */ }
trait RegistryAdapter { /* validate, render, transmit */ }
trait SecretStore { /* non-exportable key retrieval */ }
trait BackupService { /* encrypted backup and restore */ }
```

These are boundary sketches, not committed APIs. Concrete traits should be designed from use cases when implementation begins.

## 4. End-to-end write path

```text
User action
  -> frontend sends typed intent
  -> Rust authenticates and authorizes
  -> application loads expected entity revision
  -> deterministic validation/rules run
  -> domain accepts or rejects transition
  -> one SQLite transaction appends:
       new entity revision
       workflow transition (when applicable)
       audit event
  -> transaction commits
  -> frontend receives new revision and validation results
```

Optimistic concurrency uses an expected revision number. A stale screen cannot overwrite newer work; it receives a conflict and must reload/reconcile.

## 5. AI and speech flow

```text
Microphone / typed note
  -> local speech adapter (optional)
  -> transient transcript
  -> local LLM adapter (optional)
  -> schema-constrained proposal + source spans
  -> deterministic type/code validation
  -> human review
  -> accepted fields become an ordinary audited draft change
```

The provider contracts return proposals, not domain entities. Provider output is untrusted input. It must pass schema, length, code-set, and provenance checks. AI cannot call application commands and cannot set workflow state.

Ollama initially runs on a loopback endpoint; its documented local API defaults to `http://localhost:11434/api`. The adapter must enforce loopback resolution for PHI-bearing requests rather than trusting a configurable URL. See [Ollama API documentation](https://docs.ollama.com/api/introduction).

The future llama.cpp adapter can use its loopback server interface without changing the domain contract. See [llama.cpp server documentation](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md).

Speech starts behind a whisper.cpp adapter. The upstream example server accepts local inference requests but warns that file uploads and format conversion require sandboxing and validation. AutoVaxx should prefer a tightly controlled child process or in-process binding over a broadly reachable server. See [whisper.cpp server documentation](https://github.com/ggml-org/whisper.cpp/blob/master/examples/server/README.md).

## 6. Deterministic rule architecture

The rule engine has three separate rule families:

1. **Screening-completeness rules:** required template/answers are present and explicit; answers are not interpreted as eligibility or clinical clearance.
2. **Documentation rules:** completeness, temporal ordering, lot/product consistency, expiration, role, consent, and VIS requirements.
3. **Registry-readiness rules:** canonical-to-profile mapping and field/code constraints for a specific verified PREIS profile.

Clinical eligibility, recommendation, forecasting, contraindication, and precaution rules are not part of the first implementation. They may be introduced only through a separately versioned and clinically approved rule package with explicit unsupported-scope behavior.

Each evaluation records:

- Rule identifier and version.
- Clinical/content package version.
- Input entity revision identifiers.
- Result severity (`BLOCK`, `WARN`, `INFO`).
- Human-readable explanation and structured field references.
- Resolution/override and authorized actor where permitted.

Rules are pure where possible: identical inputs and versions produce identical results. Date/time comes through a clock abstraction and is materialized in the input. Rule code and content are reviewed, signed/versioned for distribution, and exercised against approved synthetic fixtures.

An LLM may convert free text into candidate structured answers. It may not choose rules, interpret a result as clinical clearance, suppress a result, or resolve a blocking result.

## 7. Persistence and encryption architecture

SQLite is the single source of truth for the MVP. All writes occur through Rust repositories in explicit transactions. Foreign keys are enabled. Migrations are forward-only, checksummed, and tested against encrypted backup/restore copies.

Production PHI storage must use an encrypted SQLite-compatible database before real patient use. SQLCipher is the leading option because it preserves much of SQLite's operational model while adding page-level encryption. Final library, distribution, platform, support, and licensing choices require a security/packaging spike. See [SQLCipher design](https://www.zetetic.net/sqlcipher/design/) and [license information](https://www.zetetic.net/sqlcipher/license/).

Key architecture:

- Generate a random database key during protected initialization.
- For the Windows 11 x64 production target, protect the per-database random key through a Windows `SecretStore` adapter using an approved DPAPI/CNG/credential-store scope and restrictive access controls; never store it beside the database.
- Never expose the key to React, logs, command output, or backup manifests.
- Encrypt each backup container with an independent random key and an approved portable recovery wrapping mechanism; a workstation-bound database key alone is not a recovery design.
- Define tested recovery and key-loss behavior before real PHI use.
- Permit plaintext only in synthetic development/test builds with a persistent visual and runtime guard.

[DATA_MODEL.md](DATA_MODEL.md) defines the logical model. [SECURITY.md](SECURITY.md) defines control details and release gates.

## 8. Registry integration architecture

The canonical domain model is richer than and independent from an HL7 payload.

```text
Finalized clinical revision
  -> RegistryAdapter.validate(profile_version)
  -> field-level readiness results
  -> RegistryAdapter.render(profile_version)
  -> immutable payload artifact + hash
  -> user reviews destination and scope
  -> explicit authorization
  -> adapter transmits or exports
  -> acknowledgement stored and reconciled
```

The official 2022 PREIS guide verifies HL7 v2.5.1 VXU/ACK and describes HTTPS POST/SOAP transport, but current environment-specific details are not verified. The adapter therefore has separate `validate`, `render`, and `transmit` capabilities. MVP may implement the first two against a confirmed profile while leaving `transmit` disabled.

No registry payload is built by an LLM. Mapping is deterministic and fixture-tested. Raw payloads contain PHI and belong in encrypted clinical storage or a tightly controlled encrypted export, never in logs.

## 9. Offline and process model

The Tauri application and SQLite database are sufficient for the core deterministic workflow. Ollama and whisper.cpp are optional local dependencies whose absence degrades assistance, not documentation or safety.

The MVP should not add a local message broker or service mesh. A durable database table may track explicitly requested export/transmission jobs if needed. Network state is surfaced clearly. Failures remain pending/failed with retry information and never become accepted merely because connectivity is absent.

Windows 11 x64 is the product-owner-approved sole MVP production target. Windows and macOS may be used for development, but production packaging, SQLCipher, secret-store, installer/update, model runtime, printer/scanner, and recovery acceptance tests run on representative Windows 11 x64 hardware.

## 10. Future multi-workstation path

Do not implement synchronization in the MVP. Preserve the migration path by:

- Using globally unique, opaque identifiers rather than database row numbers as external identities.
- Recording facility, workstation, actor, revision, and timestamps on relevant events.
- Requiring expected revisions for writes.
- Keeping domain/use-case APIs independent from SQLite connection details.
- Representing changes as append-only revisions and audit events.
- Keeping registry transmissions idempotent with stable submission identifiers.

Likely migration:

```text
MVP: each command -> local Rust application service -> local SQLite

Later: desktop -> mutually authenticated facility service
                  -> same application/domain contracts
                  -> server-managed relational database/event stream
                  -> workstation cache/offline synchronization policy
```

This later design requires patient matching, conflict policy, server authorization, device identity, certificate lifecycle, network threat modeling, and operational support. None belongs in the MVP until there is a validated multi-workstation need.

## 11. Major architectural decisions

Every decision below is proposed until implementation approval.

| ID | Decision and rationale | Alternatives considered | Primary risks | Future migration path |
|---|---|---|---|---|
| ADR-001 | **Tauri 2 + React/TypeScript/Vite + Rust.** Uses the requested desktop stack, keeps privileged behavior in memory-safe Rust, and supports a narrow webview-to-core boundary. | Electron; native per-platform UI; browser app with local service. Electron increases runtime footprint; native UIs fragment delivery; browser hosting weakens the simple local trust boundary. | Webview vulnerabilities, over-broad Tauri permissions, unsafe commands, cross-platform packaging complexity. | Preserve domain/application crates so a different shell can reuse them; keep capabilities and IPC schemas narrow. |
| ADR-002 | **Layered modular monolith.** One deployable desktop application is enough for the MVP while ports isolate volatile dependencies. | Microservices; plugin host; tightly coupled feature folders. | A poorly enforced monolith can become tangled; abstractions can be invented too early. | Extract only measured boundaries; application/domain crates can move behind a facility service later. |
| ADR-003 | **SQLite as local source of truth, designed for SQLCipher-compatible encryption.** Fits single-computer offline use and transactional audit writes. | Embedded key-value store; local PostgreSQL; unencrypted SQLite plus disk encryption. | Packaging and extension compatibility; key loss; backups may leak plaintext; SQLCipher choice needs validation. | Repository boundary permits server database later; export/import and revision IDs support migration. |
| ADR-004 | **Rust owns authorization, state transitions, rules, persistence, and external I/O.** Prevents UI bypass and centralizes trusted behavior. | Put business logic in React; direct SQLite frontend plugin; split logic across both. | Rust commands may become a large ad hoc API; serialization mismatches. | Version typed command contracts; expose the same use cases through a future authenticated service. |
| ADR-005 | **Deterministic, versioned validation engine with documentation-only rules first.** Reproducibility is mandatory; the first implementation checks screening completeness, documentation, temporal/product facts, and registry readiness without interpreting eligibility. | LLM decisions; hard-coded UI checks; immediate clinical eligibility rules; third-party online CDS. | Users may mistake completeness for clinical clearance unless unsupported scope is prominent. | Add only separately approved/versioned clinical rule packages; later adopt a verified standards-based representation behind the same port if justified. |
| ADR-006 | **Local AI provider abstraction with Ollama first.** Reduces typing while keeping PHI local and enables later llama.cpp support. | No AI; cloud AI; embed one model runtime directly. | Loopback services may be misconfigured; model output is unreliable; resource usage; provider API drift. | Add a llama.cpp adapter or in-process provider; keep proposal schema and safety boundary stable. |
| ADR-007 | **Speech provider abstraction with whisper.cpp first.** Local transcription supports hands-busy workflows without cloud disclosure. | OS speech API; cloud transcription; no speech. | Raw audio is highly sensitive; temp-file leakage; bilingual accuracy; process hardening. | Replace child-process adapter with reviewed native binding or another offline engine. |
| ADR-008 | **Append-only finalized revisions plus atomic audit events.** Prevents silent history loss and supports corrections/accountability. | In-place updates with timestamps; event sourcing for everything; database triggers alone. | Storage growth; more complex queries; privileged local tampering remains a threat. | Revisions can synchronize as immutable events; add externally anchored integrity checkpoints if threat model requires. |
| ADR-009 | **Explicit outflow authorization and adapter-based integration.** Makes every PHI disclosure visible and keeps PREIS assumptions isolated. | Automatic background sync; manual re-entry; embed PREIS fields throughout domain. | Users may defer submissions; payload files can leak; retries can duplicate. | Add policy-approved scheduled transmission only after explicit configuration, idempotency, and current PREIS certification. |
| ADR-010 | **Single-facility configuration now, facility/workstation IDs everywhere relevant.** Avoids multi-tenant complexity without painting data into a corner. | Full multi-tenant schema; omit facility identity entirely. | Some future conflicts cannot be solved by IDs alone; small extra fields in MVP. | Introduce facility service, device enrollment, and synchronization without rewriting canonical records. |

## 12. Tauri security posture

- Use the minimum Tauri capabilities per window and plugin. Capabilities merge when a window belongs to multiple capability sets, so reviews must consider the effective union. See [Tauri 2 capabilities](https://v2.tauri.app/es/security/capabilities/).
- Apply a restrictive Content Security Policy; do not load remote scripts, fonts, analytics, or patient-facing content in the privileged webview. See [Tauri CSP guidance](https://v2.tauri.app/security/csp/).
- Disable arbitrary shell execution, unrestricted filesystem access, and arbitrary URL opening.
- Validate every IPC payload in Rust, enforce size limits, and return non-PHI errors.
- Treat imported files, HL7 messages, model output, and pasted content as untrusted data.

## 13. Verification strategy

- Domain unit tests for every workflow transition and invariant.
- Table-driven rule tests with synthetic clinical cases and approved expected results.
- Repository tests against the selected encrypted SQLite build, including foreign keys, concurrency, migration, crash, and backup/restore behavior.
- Authorization tests at the Rust command boundary, including attempts from hidden/forged frontend calls.
- Provider contract tests using fake local AI/speech adapters; no real patient content.
- Network-denial tests proving core workflow works and PHI cannot leave through unapproved destinations.
- PREIS mapping fixtures derived from the confirmed profile and synthetic patients; separate conformance tests against a PRDoH-approved test environment before enabling transmission.
- End-to-end desktop tests for administration confirmation, correction, audit visibility, and restart recovery.

## 14. Related documents

- [Product requirements](PRODUCT_REQUIREMENTS.md)
- [Data model](DATA_MODEL.md)
- [Security](SECURITY.md)
- [Roadmap](ROADMAP.md)
- [Foundation decisions](FOUNDATION_DECISIONS.md)
