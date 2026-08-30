# AutoVaxx Security and Privacy Design

**Status:** Approved Phase 1 baseline; deployment-specific risk analysis and real-PHI gates remain required

**Data classification:** Patient data, clinical records, audit data, transcripts, audio, registry payloads, and acknowledgements are sensitive and may be PHI/ePHI

**Default posture:** Local-only, deny network egress, least privilege, fail closed

## 1. Scope and responsibility

AutoVaxx is intended for regulated healthcare use, but software design alone does not make a pharmacy or deployment HIPAA compliant. The deploying organization remains responsible for risk analysis, policies, workforce authorization/training, physical safeguards, retention, incident response, contingency operations, vendor agreements, and applicable Puerto Rico law.

The [HHS Security Rule summary](https://www.hhs.gov/hipaa/for-professionals/security/laws-regulations/index.html) identifies administrative, physical, and technical safeguards and requires protection of ePHI confidentiality, integrity, and availability. It also requires risk analysis, access control, audit controls, authentication, transmission security, and contingency planning. AutoVaxx controls support those duties but do not replace them.

## 2. Security objectives

1. **Confidentiality:** Only authorized people and approved local processes access PHI.
2. **Integrity:** Clinical facts, confirmations, corrections, rules, and registry outcomes cannot be silently altered through normal application use.
3. **Availability:** Authorized staff can complete and recover documentation during internet outages and common workstation failures.
4. **Accountability:** Material access, changes, overrides, disclosures, and administrative actions are attributable and reviewable.
5. **Clinical safety:** AI cannot act as the clinical rules engine or administration authority.
6. **Data minimization:** Collect, retain, display, and disclose only what the workflow and verified obligations require.

## 3. Trust boundaries and data flows

```text
Untrusted / less trusted                       Trusted AutoVaxx boundary

User input -------------------------------> React UI
Imported file / barcode / HL7 ------------> validation -> Rust use case
Local model output ------------------------> schema/type validation -> proposal
Webview -----------------------------------> narrow Tauri IPC -> Rust authorization

                                            Rust core -> encrypted SQLite
                                            Rust core -> OS credential store
                                            Rust core -> encrypted backup/export

Explicit authorized disclosure boundary:
Rust registry adapter -> allowlisted TLS destination -> PREIS/test endpoint
```

The webview, imported data, pasted text, local model output, registry responses, and upstream content packages are untrusted. The Rust core is trusted only after dependency, configuration, capability, and code review.

## 4. Threat model

### Protected assets

- Patient identity and contact information.
- Screening, consent, VIS, immunization, and correction history.
- Local user identities, roles, professional credentials, and session state.
- Database, backup, export, raw audio, transcripts, model prompts/responses, HL7 payloads/ACKs.
- Encryption and signing keys.
- Clinical rule/content packages and application binaries.
- Audit history and registry submission evidence.

### Threat actors and failures

- Unauthorized coworker using an unlocked workstation.
- Malware or a local user copying the database, backups, exports, clipboard, or temporary files.
- Privileged operating-system administrator tampering with local data or binaries.
- Vulnerable webview/content causing command invocation or data exfiltration.
- Misconfigured Ollama/whisper.cpp/llama.cpp service listening beyond loopback.
- Prompt injection or malformed model output influencing proposed data.
- Malicious or malformed imported HL7, barcode, PDF, or content package.
- Incorrect role configuration or frontend-only authorization.
- Lost/stolen device, lost keys, failed disk, interrupted migration, or corrupt backup.
- Stale clinical/VIS/registry content.
- User transmitting to the wrong destination or retrying a submission twice.
- PHI appearing in logs, crash reports, filenames, process arguments, analytics, or support bundles.
- Compromised dependency/update or unsigned content package.

### Out of scope for the MVP, but not dismissed

- Nation-state resistance and protection from a fully compromised privileged operating system.
- Multi-workstation network compromise and distributed conflict attacks.
- Cloud infrastructure controls, because the MVP has no cloud data plane.
- Protection from authorized users deliberately photographing or manually retyping displayed PHI; policy, physical safeguards, training, and sanctions remain necessary.

## 5. Identity, authentication, and authorization

- Every user has a named account; shared clinical accounts are prohibited.
- Use application-local named accounts for the recommended MVP identity boundary. Store Argon2id password verifiers with unique salts and versioned parameters; exact parameters require a benchmark on supported Windows 11 x64 hardware.
- Facility policy approves password length, recovery, deprovisioning, and emergency access. The engineering default is a 15-minute lock after five failures within 15 minutes, with increasing delay and non-enumerating errors.
- Default to a 15-minute inactivity lock, bounded by approved facility configuration, and lock on operating-system workstation lock where detectable. Production cannot disable locking.
- Require authentication within the previous five minutes for administration confirmation, future clinical override, correction/void, backup restore/export, PHI export/transmission, user/role changes, and security configuration changes.
- Enforce permissions in Rust application services. The database layer also rejects invalid state transitions; React visibility is only presentation.
- Record login success/failure, lock, logout, privilege change, and privileged action without passwords, tokens, or unnecessary user/patient detail in operational logs.
- Emergency access is not a hidden bypass. If later required, it needs explicit policy, reason, short lifetime, prominent audit, and retrospective review.

## 6. Encryption and key management

### Database at rest

- Real PHI is blocked until the selected encrypted SQLite build passes packaging, migration, backup, corruption, and performance tests on representative Windows 11 x64 hardware.
- SQLCipher is the integrated Phase 1 candidate. Current-host behavior passes; Windows packaging/runtime and Community-versus-supported distribution remain undecided.
- Use strong random keys; never derive the database key directly from a user's login password.
- Generate one random key per database and retrieve it through a Rust `SecretStore`. `WindowsSecretStore` uses Windows Credential Manager under an application service name and opaque database reference; it refuses replacement and fails closed on missing, denied, malformed, unavailable, protection, or recovery errors.
- Persist only a versioned opaque key descriptor beside the database. Missing or unrecoverable protected material never triggers silent key generation.
- Keep the key separate from the database and do not place it in source, environment examples, process arguments, frontend state, logs, crash data, or backup manifests.
- Zeroize key material where practical and minimize its lifetime/copies in memory.

HHS breach guidance says valid at-rest encryption should be consistent with NIST storage-encryption guidance and that the confidential process/key should be kept separate from the encrypted data. See [HHS guidance on rendering PHI unreadable](https://www.hhs.gov/hipaa/for-professionals/breach-notification/guidance/index.html). Deployment counsel and the risk analysis determine how this guidance applies.

### Backups and recovery

- Backups are encrypted independently with authenticated encryption and a versioned format.
- A backup includes schema/app versions, integrity metadata, and content/rule package identifiers, but its unencrypted manifest contains no PHI.
- Each backup uses an independent random key. Its portable recovery wrapping secret is held separately from the workstation and backup; a machine-bound database key alone is not sufficient for disaster recovery.
- Backup creation first writes an independently keyed SQLCipher online snapshot, never a plaintext SQLite snapshot. A fresh AES-256-GCM content key authenticates the payload and non-PHI header; the snapshot key is inside that encrypted payload.
- Support authorized manual backups and configurable scheduled encrypted local backups. A selected removable volume is allowed; cloud and implicit network destinations are not.
- Restore requires authorization, writes an audit event, checks integrity/schema/audit chain, and never overwrites the active database until verification succeeds.
- Restore staging remains SQLCipher-encrypted. Validation covers container structure/authentication, SQLite and cipher integrity, schema/migration checksum, foreign keys, audit-chain hashes, audit JSON, and revision minima before explicit cutover.
- Test restore regularly on a separate synthetic installation. An untested backup is not a control.
- Define secure disposal for expired backups and media under facility policy.

### Export files

- Default to encrypted exports with an expiry/handling warning and destination confirmation.
- Do not silently write PHI to Downloads, Desktop, temp directories, or predictable filenames.
- Plaintext export, if a verified external workflow requires it, needs explicit warning, re-authentication, selected path, audit, and documented facility policy.

## 7. Local AI and speech isolation

- Patient data may be sent only to an approved local provider bound to loopback or invoked as a controlled child process.
- Reject hostnames/addresses that resolve outside loopback. Avoid configurable proxies for PHI-bearing model calls.
- The frontend never calls model providers directly.
- Do not enable Ollama cloud endpoints, remote model URLs, provider telemetry, or cloud fallback for patient workflows.
- Restrict model requests to the minimum context. Never expose the database, arbitrary filesystem, registry credentials, or application tools to the model.
- Validate model output as untrusted data: schema, size, encoding, allowed fields, code values, and source/provenance.
- Separate proposed values visually and structurally from accepted data. Human acceptance is an audited draft edit.
- Do not persist raw prompts, interview text/transcripts, model responses, rejected proposals, or source spans after review by default. Retain only minimum provider/model/schema provenance, field names, reviewer disposition, and the resulting clinical revision reference.
- Process speech locally. Use random non-identifying temp names, restrictive permissions, bounded file size/duration, validated formats, and guaranteed cleanup after success, failure, cancellation, and crash recovery. The MVP has no raw-audio retention feature.
- Prefer a controlled whisper.cpp child process or reviewed binding. If a server is used, bind to loopback, authenticate where supported, sandbox it, and prohibit broad CORS/network access.
- Model binaries and weights are software supply-chain inputs: verify source, license, checksum/signature, version, and supported hardware before installation.

## 8. Tauri and frontend controls

- Configure a restrictive Content Security Policy that blocks remote scripts, frames, connections, fonts, and media unless a reviewed feature needs a narrow exception.
- Define minimum Tauri 2 capabilities per window. Review the effective union when windows are assigned multiple capabilities.
- Do not expose generic shell, SQL, filesystem, HTTP, or secret-store commands to the webview.
- Validate and authorize every IPC command in Rust; apply request size/depth limits.
- Escape displayed data and avoid unsafe HTML injection.
- Do not load patient or help content from remote web pages inside the privileged webview.
- Disable production developer tools unless a controlled support mode is explicitly authorized and audited.
- Do not persist PHI in browser storage, service-worker caches, URL/query strings, DOM diagnostics, or frontend error reporting.
- Clear sensitive state on logout/session lock and prevent stale windows from mutating data through expected-revision checks.

## 9. Network and external disclosure controls

- Core application network policy is deny by default.
- Separate non-PHI software/content update traffic from PHI transmission adapters.
- Maintain an explicit destination allowlist with scheme, host, port, certificate policy, registry profile, environment (`TEST`/`PRODUCTION`), and approval metadata.
- Patient-bearing traffic requires TLS and hostname/certificate validation. Do not provide a user-facing “ignore certificate error” path.
- Before every MVP PHI export/transmission, show destination, environment, purpose, patient/record count, data categories, and artifact/profile versions; require explicit re-authenticated authorization.
- Use stable idempotency/submission identifiers and parse the actual ACK. Transport success is not registry acceptance.
- Store raw payloads and ACKs encrypted. Log only non-PHI status/error codes and internal correlation IDs.
- Never send automatically because connectivity returns. Background automatic PHI synchronization is out of MVP scope.
- Registry credentials live in the OS credential store and are scoped to the approved destination/environment.

## 10. Logging, diagnostics, and support

In these documents, **operational application logs** means diagnostic output sent to log files, consoles, crash handlers, or telemetry. The **audit ledger** is different: it is an access-controlled, encrypted record in the clinical datastore and may contain minimal patient-linked identifiers required for accountability. Audit data must never be copied into operational logging sinks.

### Prohibited in operational logs

- Names, initials tied to records, dates of birth, addresses, phone/email, identifiers.
- Screening/consent answers, notes, vaccine details, VIS evidence, clinical rule inputs.
- Audio, transcripts, prompts, responses, source spans.
- SQL values/parameters, registry payloads/ACKs, imported file content.
- Passwords, verifiers, session tokens, encryption keys, registry credentials.

### Permitted examples

- `event=registry_submission_failed submission_id=<opaque> error_class=tls_timeout`
- `event=rule_evaluation_complete rule_package=<version> result_counts={block:1,warn:0}` only when the aggregate cannot identify a patient in its context.
- `event=db_transaction_failed correlation_id=<opaque> error_class=constraint_violation`

Error messages shown to users contain enough action guidance without echoing PHI. Detailed clinical validation belongs in authenticated application views backed by the encrypted database, not logs.

Crash reporting is local-only by default. Support bundles require preview/redaction, explicit authorization, and must exclude the database, PHI files, model inputs, and secrets. No third-party analytics or cloud crash SDK is enabled in patient workflows.

## 11. Audit and integrity

- Audit events are appended in the same transaction as meaningful mutations.
- Finalized revisions and audit events have no normal update/delete path.
- Record attempted denied privileged actions as security audit events where doing so cannot create a denial-of-service condition.
- Audit reads and exports are themselves audited.
- Use an ordered hash chain to detect accidental corruption or simple tampering. Clearly state that a local privileged attacker may still rewrite both data and chain.
- Run integrity verification at startup and before/after backup, restore, and migration. A failure enters a safe recovery mode; it does not silently rebuild history.
- Access to audit data follows least privilege because audit identifiers and patterns may be sensitive.

## 12. Input, file, and content-package security

- Treat HL7, CSV, barcode, PDF, image, model output, and clipboard content as hostile.
- Apply strict size, type, encoding, recursion, and record-count limits before parsing.
- Parse in memory-safe libraries/processes where possible and keep parsers patched.
- Never interpret inbound text or README/instruction content as application commands.
- Content/rule packages require source provenance, version, hash, and signature verification before activation.
- Package activation is an administrative, audited action. Retain the prior active package for rollback and reproducibility.
- Do not automatically mark stale content as current because an update check failed. Surface last verified dates and enforce the approved stale-content policy.

## 13. Availability and contingency controls

- Use transactional writes and SQLite durability settings selected through failure testing, not defaults assumed safe.
- Recover exact workflow state after process or power loss, especially after administration confirmation.
- Perform pre-migration encrypted backup, free-space check, checksum, migration, integrity check, and safe cutover.
- Make model outages non-blocking for deterministic documentation.
- Make registry outages explicit and retryable; never lose a finalized record because transmission failed.
- Document downtime workflow, backup frequency, recovery time/recovery point objectives, responsible roles, and paper reconciliation before pilot.
- Provide a tested process for lost/stolen devices, account compromise, suspected data corruption, and unavailable keychain.

## 14. Dependency, build, and update security

- Pin Rust and JavaScript dependencies with lockfiles and review security advisories.
- Minimize Tauri plugins and native dependencies.
- Generate software bills of materials and retain build provenance for releases.
- Sign application installers/updates for each platform; verify before installation.
- Separate update metadata from patient data and never include facility/patient identifiers in update checks.
- Review SQLCipher/native library licensing and support posture before distribution.
- Verify local model and clinical-content artifacts by checksum/signature. Downloading a model is an administrative action, not an implicit result of opening a patient.
- Run static analysis, dependency audit, secret scan, and reproducible release checks in CI without PHI.

## 15. Security verification gates

Before real PHI:

1. Deployment-specific HIPAA security risk analysis and Puerto Rico legal/privacy review are complete.
2. Threat model is reviewed by engineering, clinical, security/privacy, and facility owners.
3. Encrypted database, Windows secret-store integration, encrypted backup, key-loss, migration, and restore tests pass on representative Windows 11 x64 hardware.
4. Authorization matrix tests call Rust commands directly to prove UI bypass does not work.
5. Administration confirmation, clinical override, correction/void, and disclosure require correct role and recent authentication.
6. Network tests deny non-loopback AI/speech and non-allowlisted outbound destinations.
7. Log/crash/temp/clipboard/support-bundle reviews find no synthetic PHI leakage.
8. Rule/content package signature, rollback, and stale-content behaviors pass.
9. Fuzz/property tests cover IPC payloads and enabled import/HL7 parsers.
10. Backup restore and disaster recovery drill succeeds with synthetic data.
11. External security review and penetration test findings are resolved or formally accepted.
12. Incident response, breach assessment, downtime, retention, media disposal, and workforce procedures are approved.
13. Production startup proves encrypted database, Windows SecretStore, non-development authentication, production logging, approved schema, required security configuration, and an explicit compile-time `real-phi` entitlement; no runtime checkbox can bypass them. The entitlement is not enabled in Phase 1.

Before PREIS transmission:

1. PRDoH confirms the active implementation guide/profile and onboarding contract.
2. Test and production endpoints/credentials are separately configured and visibly distinguished.
3. Synthetic conformance cases pass in the authorized PREIS test environment.
4. ACK parsing, partial failure, retry/idempotency, reconciliation, and downtime behaviors are verified.
5. The facility approves the minimum-necessary data mapping and transmission policy.

## 16. Major security decisions

| ID | Decision and rationale | Alternatives considered | Primary risks | Future migration path |
|---|---|---|---|---|
| SEC-001 | **Encrypt the database and backups before real PHI; keep keys separate in the OS credential store.** Device theft/database copying is a primary local-first risk. | Rely only on full-disk encryption; user-password-derived database key; plaintext SQLite. | Key loss, keychain compromise, native packaging complexity. | Hardware-backed keys or enterprise key escrow after a deployment risk review. |
| SEC-002 | **Named local accounts with Rust-enforced roles and re-authentication.** Clinical accountability requires attributable actions. | Shared workstation account; OS login only; frontend checks. | Local account recovery/support burden; shared OS sessions still create risk. | Federated facility identity with device-bound sessions in multi-workstation mode. |
| SEC-003 | **Network deny-by-default with explicit per-disclosure authorization.** Enforces the local-first promise and makes PHI outflow visible. | Broad network permission; automatic sync; air-gapped forever. | Deferred submissions and user friction; configuration mistakes. | Policy-approved scheduled transmission with constrained destination, purpose, and audit after certification. |
| SEC-004 | **Loopback-only AI/speech with no cloud fallback.** Preserves PHI locality while allowing assistance. | Cloud APIs; user-configurable endpoints; no AI. | Local service exposure, provider logs, model unreliability. | Reviewed in-process runtimes or managed local facility inference service with mutual authentication. |
| SEC-005 | **No PHI in operational logs; clinical/audit detail remains encrypted.** Reduces leakage through support and crash channels. | Log full payloads for debugging; disable all logs. | Harder diagnosis; accidental sensitive error strings. | Structured safe diagnostics and authorized on-device inspection, never broad PHI telemetry. |
| SEC-006 | **Tauri capabilities and commands follow least privilege.** The webview is not trusted with storage, secrets, or arbitrary I/O. | Generic shell/filesystem/HTTP plugins; direct frontend database. | Capability drift and IPC injection. | Automated capability diffing and isolation into separate windows/processes if features expand. |
| SEC-007 | **Signed/versioned application, rule, terminology, and VIS content.** Clinical correctness and supply-chain integrity require known inputs. | Online latest-at-runtime; unsigned local files. | Key/signing operations and offline update logistics. | Enterprise distribution/update service with staged rollout and rollback. |
| SEC-008 | **Append-only clinical revisions and atomic audit events.** Preserves historical truth and accountability. | Mutable rows plus timestamps; logs as audit. | Database growth; local privileged tampering. | Server-side immutable ledger or externally anchored keyed checkpoints when multi-workstation risk justifies it. |

## 17. Residual risks requiring policy or future work

- A compromised or malicious operating-system administrator may access data while the application is unlocked and may tamper with binaries/state.
- Full-disk encryption, screen privacy, workstation placement, OS patching, endpoint protection, and physical controls are deployment responsibilities.
- Users can disclose PHI by photography, clipboard, print, or manual transcription; software can reduce but not eliminate this risk.
- A stale clinical/content package can remain installed during long offline periods; policy must define blocking and emergency behavior.
- Local AI may make persuasive but wrong proposals. Human review and deterministic validation reduce but do not eliminate automation bias.
- Single-workstation storage creates availability risk unless backups and restore drills are reliable.
- Current PREIS production requirements remain unverified and may differ from the 2022 published guide.

## 18. Related documents

- [Product requirements](PRODUCT_REQUIREMENTS.md)
- [Architecture](ARCHITECTURE.md)
- [Data model](DATA_MODEL.md)
- [Roadmap](ROADMAP.md)
- [Foundation decisions](FOUNDATION_DECISIONS.md)
