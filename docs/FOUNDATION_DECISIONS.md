# AutoVaxx Foundation Decisions

**Status:** Phase 1 authorization recorded; unresolved real-PHI and external-verification gates remain

**Reviewed:** 2026-08-30

**Scope:** The minimum product, architecture, security, and operating decisions for Phase 1 with synthetic data. The product owner authorized Phase 1 and approved Decisions 1, 2, and 6 on 2026-08-30. This document does not authorize real-PHI use, clinical eligibility logic, live PREIS transmission, or Phase 2.

## 1. How to read this review

Status labels have precise meanings:

- **DECIDED:** already established by the product constraints and safe to treat as a project constraint.
- **RECOMMENDED:** the proposed MVP choice; product-owner approval is still required before it becomes a constraint.
- **REQUIRES EXTERNAL VERIFICATION:** software can preserve the capability boundary, but legal, clinical, registry, or deployment owners must resolve the policy or factual requirement.
- **DEFERRED:** intentionally excluded from the MVP or from Phase 1.

“Blocks Phase 1” means it prevents synthetic-data foundation work. It does not mean the question can remain unresolved before real PHI or a clinical pilot. “External verification” never means that engineering may invent a temporary regulatory answer.

## 2. Summary decision table

| Decision | Status | Recommendation | Why | Blocks Phase 1? | Requires External Verification? | Deferred Work |
|---|---|---|---|---|---|---|
| 1. MVP clinical scope | **APPROVED** | Begin with documentation completeness and workflow validation only (Option A); capture screening but make no eligibility, recommendation, contraindication, or precaution judgment. | It delivers workload reduction without presenting an unvalidated rule set as clinical clearance. | No; approved 2026-08-30. | Clinical review is required before any later clinical rule package or clinical pilot. | Eligibility, forecasting, recommendations, contraindication/precaution packages. |
| 2. First operating system | **APPROVED** | Windows 11 x64 is the only MVP production target; Windows and macOS may be development hosts. | Matches pharmacy desktops and sharply reduces installer, device, encryption, and support matrices. | No; approved 2026-08-30. | Confirm facility hardware, Windows edition/lifecycle, peripherals, and IT policy before pilot. | macOS/Linux production support. |
| 3. Database encryption | **RECOMMENDED** | Use SQLCipher-compatible whole-database encryption, selected through a Phase 1 packaging/licensing spike; do not substitute field encryption or disk encryption alone. | Preserves SQLite transactions and encrypts database/journal pages with less schema complexity. | No; the spike is Phase 1 work, but its success blocks Phase 1 exit and all real PHI. | Security/licensing review and supported Rust/Windows package selection. | Selective field encryption only if later threat modeling justifies defense in depth. |
| 4. Encryption key management | **RECOMMENDED** | Generate one random database key per database; wrap it with Windows-protected installation key material; use a separate portable backup recovery design. | A key beside the database defeats theft protection, while machine-bound protection alone defeats disaster recovery. | No; interfaces and a synthetic prototype can proceed. | IT/security must approve recovery custody and Windows protection scope before PHI. | Hardware-backed/enterprise escrow and rotation automation. |
| 5. Backup and restore | **RECOMMENDED** | Support auditable manual backups plus configurable scheduled encrypted local backups; allow an explicitly selected removable-media destination. | Single-workstation availability requires routine backups, while manual-only operation is easy to neglect. | No; exact retention does not block foundation work. | Facility policy must set destination, custody, schedule, retention, RPO/RTO, and recovery owners. | Cloud backup, network-share automation, centralized backup. |
| 6. Local authentication | **APPROVED** | Use application-local named username/password accounts with Argon2id verifiers; defer Windows account integration. | Gives consistent attribution and roles without requiring a facility identity-provider project. | No; approved 2026-08-30. | Facility policy must approve timeout, recovery, emergency access, and account administration before PHI. | Windows/Entra/AD federation, passkeys, smart cards. |
| 7. Authorization model | **DECIDED** | Rust-enforced permissions, four initial roles, and multiple roles per user; administrator has no clinical access by default. | Frontend hiding is bypassable; explicit capabilities keep clinical and administrative authority separate. | No. | Legal/operations must confirm who may exercise clinical permissions before pilot. | Custom roles and centralized policy administration. |
| 8. Administration confirmation | **DECIDED** | A recent re-authenticated vaccinating professional explicitly attests to a versioned snapshot in one audited transaction. AI and support-only users cannot invoke it. | This operation turns documentation into an assertion that a physical act occurred. | No. | Clinical/legal owners must approve the displayed attestation text and override policy before pilot. | Multi-factor or device-bound signatures. |
| 9. Finalization versus administration | **DECIDED** | Confirmation creates `ADMINISTERED_PENDING_DOCUMENTATION`; finalization is a second explicit action and is never automatic. | Physical administration and documentation completeness are different facts and may fail at different times. | No. | Facility policy must approve late-entry and incomplete-post-dose handling before pilot. | Automated workflow assistance that never replaces explicit finalization. |
| 10. Corrections, voids, immutability | **DECIDED** | Immutable post-attestation/finalized revisions; correction or void appends a linked revision with reason and actor. Cancellation is pre-administration only. | Preserves historical truth and downstream reconciliation. | No. | Retention and correction authority require policy confirmation before PHI. | Policy-driven purge after retention obligations; cross-system correction automation. |
| 11. Consent model | **REQUIRES EXTERNAL VERIFICATION** | Model consent/refusal/withdrawal and evidence generically; do not claim any signature method is legally sufficient. | Consent authority, evidence, retention, and minor/representative rules are policy and legal questions. | No; implement an extensible evidence model only after Phase 1. | Puerto Rico counsel/pharmacy policy and clinical workflow owner. | Device-specific signature capture and remote consent. |
| 12. VIS content management | **RECOMMENDED** | Use immutable, versioned, integrity-checked local content packages; only approved official documents may satisfy VIS evidence. | Offline use must preserve the exact edition delivered and must not let AI alter official content. | No. | Owners must approve language sources, freshness checks, stale/unknown blocking, and emergency policy. | Automated online content update service. |
| 13. AI provider architecture | **DECIDED** | Depend on a narrow `LocalAiProvider`; connect to a separately provisioned loopback Ollama runtime first; keep llama.cpp replaceable. | Isolates API drift and keeps AI optional and proposal-only. | No. | Model/license/hardware and deployment hardening review before PHI. | App-managed runtime/model downloads; llama.cpp adapter. |
| 14. AI data retention | **RECOMMENDED** | Do not retain raw prompts, source interview text, raw responses, rejected values, or source spans after review; retain minimum provenance and disposition. | Minimizes a new PHI corpus while preserving accountability for accepted changes. | No. | Retention owner must approve any future raw-artifact use. | Opt-in encrypted raw artifact retention for a separately approved purpose. |
| 15. Speech/audio retention | **RECOMMENDED** | Audio is local, private, bounded, temporary, and deleted after transcription; the MVP has no “retain recording” feature. | Raw audio is high-risk PHI and is not needed for documentation after review. | No. | Validate deletion, crash cleanup, and facility recording policy before PHI. | Explicit recording retention and consent workflow. |
| 16. Barcode scope | **DEFERRED** | Phase 1 defines a scanner/parser port only; the first vertical slice may accept keyboard-wedge input as an untrusted proposal but performs no external lookup or invented resolution. | Product-code semantics and device support can expand scope without proving core value. | No. | Verify symbologies, code sources, scanners, and product mappings before semantic parsing. | GS1 parsing, product/lot/expiry extraction, external lookup. |
| 17. PREIS scope | **REQUIRES EXTERNAL VERIFICATION** | Build canonical data and the adapter boundary; render a candidate only against a currently verified profile. Keep transport, credentials, ACKs, and live transmission disabled. | The April 2022 guide does not establish the current production contract. | No; registry adapter fakes and canonical data can proceed. | PRDoH must answer the discovery checklist before candidate conformance or live integration claims. | Enrollment, transport, ACK/retry/reconciliation, production submission. |
| 18. Canonical immunization model | **DECIDED** | Use an immutable, code-aware `VaccinationAdministration` aggregate independent of UI, AI, HL7, and PREIS. | Clinical history must survive UI and registry-profile changes. | No. | Current PREIS-required fields remain a mapping-profile question. | FHIR/other registry mappings and broader clinical concepts. |
| 19. Patient identity and duplicates | **RECOMMENDED** | Separate given/middle/first surname/second surname; use typed external identifiers; deterministic duplicate candidates require human disposition and never auto-merge. | Preserves Puerto Rico naming patterns and avoids irreversible identity errors. | No. | Confirm minimum patient/registry fields and local matching policy before pilot. | Audited merge/unmerge workflow and probabilistic matching. |
| 20. Local time and timestamps | **DECIDED** | Store UTC instant plus IANA zone, numeric offset, and entered local time where clinical meaning matters; default display zone is `America/Puerto_Rico`. | Preserves the exact local clinical assertion and remains portable outside Puerto Rico. | No. | None for foundation work; export formatting follows verified profiles. | Multi-zone facility preferences and trusted time services. |
| 21. Operational logging versus audit | **DECIDED** | PHI-free operational diagnostics are separate from encrypted, minimum-necessary, append-only audit events. | Diagnostic convenience must not become an uncontrolled PHI copy. | No. | Facility policy must set retention and audit-review cadence before PHI. | Centralized privacy-preserving operations monitoring. |
| 22. Network policy | **DECIDED** | Default deny; Phase 1 permits only validated loopback providers. Signed software/content import is offline; PREIS and external egress remain disabled. | The safest local-first baseline is no ambient network path for patient data. | No. | IT/security approves update and later destination allowlists. | Online signed updates and PREIS egress adapters. |
| 23. MVP UI languages | **RECOMMENDED** | Build the UI architecture for English and Spanish from the first component; ship bilingual workflow chrome, while clinical content requires separate approval. | Retrofitting i18n is expensive and Puerto Rico workflows are bilingual. | No; untranslated placeholder content may use synthetic development copy. | Clinical/legal review of patient-facing and clinical translations before pilot. | Additional languages and localization workflows. |
| 24. MVP boundaries | **DECIDED** | Keep the MVP to one production workstation/facility, documentation workflow, local storage, optional local assistance, and registry-candidate generation only; the OS remains Decision 2. | A narrow product can be validated for safety and workload reduction. | No. | Specific clinical/legal/PREIS gates still apply before their affected capabilities. | See the explicit deferred list in Decision 24. |

### 2.1 Consequence matrix

This matrix makes the required consequence review explicit; the detailed records below explain the tradeoffs and controls.

| Decision | Security | Clinical | Regulatory/policy | Operational | Engineering |
|---|---|---|---|---|---|
| 1. Scope | Avoids an unvalidated clinical authority path. | Professional retains eligibility judgment; completeness is not clearance. | Clinical approval gates later packages and pilot wording. | Existing assessment process remains necessary. | Smaller deterministic engine; preserve a future package port. |
| 2. OS | One hardening/secret-store baseline. | No direct clinical change. | Facility lifecycle/device policy must be approved. | Smaller installer/peripheral/support matrix. | Windows CI and native packaging become mandatory; portable domain stays isolated. |
| 3. Encryption | Protects copied database/journal content; key/corruption risks remain. | Availability failures must not lose attested history. | Risk/licensing review and real-PHI gate. | Native packaging, backup, and recovery procedures required. | SQLCipher build/binding/migration spike and real-instance tests. |
| 4. Keys | Separates keys from data; portable recovery increases custody risk. | Key loss can make records unavailable. | Recovery custodians and access policy require approval. | Provisioning, recovery, and rotation drills required. | Windows protection adapter plus backup key-wrapping design. |
| 5. Backup | Encrypted artifacts reduce media-disclosure risk. | Restore must preserve exact revisions and audit. | Retention, legal hold, and restore authority remain policy. | Scheduled plus manual operations, alerts, and drills. | Versioned container, consistent snapshot, staged verified cutover. |
| 6. Authentication | Named accounts, lockout, re-auth reduce misuse. | Attestations remain attributable. | Facility approves account/recovery/emergency rules. | Local provisioning and support burden. | Argon2id, session state, release-disabled dev shortcut, recovery tests. |
| 7. Authorization | Least privilege and Rust checks resist UI bypass. | Only clinical role confirms/finalizes/corrects. | Professional authority and dual-control questions remain external. | Multiple roles cover small-facility staffing. | Named capabilities and command-boundary denial tests. |
| 8. Confirmation | Recent auth and immutable snapshot protect the strongest transition. | Exact professional attestation; no AI/support invocation. | Attestation/override/late-entry text requires approval. | More deliberate step and recovery workflow. | Atomic state/revision/audit transaction and snapshot fingerprint. |
| 9. Finalization | Prevents UI/autosave from asserting completeness. | Separates physical act from completed record. | Incomplete-post-dose/late completion policy remains external. | Users may resume pending documentation after interruption. | Explicit state machine and crash recovery; no auto-finalize. |
| 10. Corrections | Append-only history limits silent alteration. | Original assertion and later correction/void remain distinguishable. | Reasons, authority, retention, and notices require policy. | More deliberate reconciliation and storage. | Stable roots, immutable revisions, current pointer, regenerated artifacts. |
| 11. Consent | Evidence/artifacts receive encrypted least-privilege storage. | Missing required consent blocks under approved policy. | Legal/facility sufficiency is explicitly unresolved. | Supports varied methods without forcing signature hardware. | Versioned evidence model; device integrations deferred. |
| 12. VIS | Signed/hashed packages resist tampering and silent substitution. | Exact applicable edition/language delivered before dose. | Current/stale/emergency and language policy need approval. | Offline packages require controlled import/update cadence. | Immutable artifacts, mappings, signature/hash verification, rollback. |
| 13. AI provider | Loopback validation and no tools limit disclosure/action. | Suggestions remain nonauthoritative. | Model/license/deployment review before PHI. | Optional runtime can be unavailable without blocking work. | Stable provider contract, typed errors, deadlines, replaceable adapters. |
| 14. AI retention | Minimizes secondary PHI corpus and backup exposure. | Accepted value remains attributable without keeping rejected text. | New raw retention requires purpose/retention approval. | Less patient-specific debugging. | Ephemeral review store plus minimum provenance schema. |
| 15. Speech | Private temp handling and deletion reduce audio leakage. | Transcript requires human review; recording is not evidence. | Recording policy is avoided in MVP. | No recording retrieval/support workflow. | Bounded format/process, cleanup on all paths and startup. |
| 16. Barcode | Untrusted bounded input; no external lookup disclosure. | Unknown data cannot become an invented product. | Code/device sources require later verification. | Manual fallback remains. | Port and optional keyboard-wedge capture only; parsers deferred. |
| 17. PREIS | No credentials/egress yet; artifacts remain encrypted. | Canonical record is not distorted by unverified mapping. | PRDoH discovery/conformance is mandatory. | Manual candidate inspection only; no acceptance claim. | Adapter seams/fakes now; mapper only for verified-render profile. |
| 18. Canonical model | Minimum necessary structured facts and immutable references. | Preserves clinical meaning/provenance across mappings. | PREIS-required subset remains profile-specific. | One source for documents and candidates. | Aggregate independent of UI/AI/HL7 with versioned codes. |
| 19. Identity | Minimization and typed identifiers reduce misuse. | Avoids wrong-patient merge harm. | Required demographics/sex-related field need verification. | Human duplicate review adds work but preserves safety. | Separate surnames, normalization, versioned deterministic candidates. |
| 20. Time | Avoids ambiguous audit/event ordering. | Preserves actual local administration assertion and late entry. | Export syntax follows verified profile. | Visible timezone mismatch may require correction. | UTC + zone + offset + local value; injected clock and boundary tests. |
| 21. Logs/audit | Keeps PHI out of weak diagnostic sinks; protects audit. | Audit reconstructs meaningful clinical changes. | Retention/review/export policy remains external. | Safe diagnostics are less verbose. | Separate schemas/stores, atomic append, integrity chain, redaction tests. |
| 22. Network | Default deny minimizes exfiltration paths. | Offline care documentation does not fail open. | Future destinations need explicit approval. | Updates/transmission are deliberate and may be delayed. | Rust allowlists, narrow adapters, CSP/capability controls, offline tests. |
| 23. Languages | No material new data exposure; translated content stays versioned. | Clinical translations are never assumed correct. | Patient-facing/clinical wording needs review. | Bilingual workflow adds translation maintenance. | Message catalogs and locale formatting from the first component. |
| 24. Boundaries | Fewer integrations and attack surfaces. | No autonomous or broad clinical claims. | External gates stay attached only to affected capabilities. | Smaller training/support footprint. | No cloud/sync/platform frameworks; defer until measured need. |

## 3. Detailed decisions

### Decision 1 — MVP clinical scope

**Status: APPROVED 2026-08-30.** The product owner approved Option A for Phase 1. Puerto Rico clinical approval remains required before any later clinical-rule package or pilot claim.

**Why it matters.** A screen that appears to “clear” a patient can influence care even if labeled as documentation assistance. The first release can reduce transcription, omission, and rework without taking on eligibility, schedule, or contraindication logic.

**Options and tradeoffs.**

| Option | Advantages | Disadvantages and risk |
|---|---|---|
| A. Documentation-only | Smallest clinically safe scope; fastest to validate; deterministic and explainable; useful offline. | Does not recommend a vaccine or determine eligibility; users must follow their existing clinical process. |
| B. Limited contraindication/precaution rules | Can catch selected clinical hazards. | “Limited” may be mistaken for complete clearance; needs a named clinical owner, authoritative sources, versioned fixtures, update governance, and unsupported-scope behavior. |
| C. Full recommendation/eligibility | Highest possible decision support. | Large, frequently changing, population-dependent clinical program; unacceptable MVP scope and validation burden. |

**MVP recommendation.** Choose Option A. The first rule engine validates workflow and documentation facts only:

- screening is recorded against the required template and every required answer is explicit (`YES`, `NO`, `UNKNOWN`, or `DECLINED` as allowed); it does not interpret answers as eligibility;
- consent evidence and applicable VIS delivery are present;
- product, manufacturer, lot, expiration, dose, unit, route, anatomical site, vaccinator, facility, and administration time are present and internally well-formed;
- product/lot mismatch, an expiration date earlier than the asserted administration date, missing required evidence, invalid state/role, or impossible timestamp ordering blocks confirmation/finalization;
- registry readiness is a separate deterministic mapping check and is not clinical clearance.

The engine must not recommend a product, determine whether a dose is due, apply intervals, interpret pregnancy/immunocompromise/allergy answers, calculate contraindications/precautions, forecast, diagnose, or state that vaccination is clinically safe. Screening answers remain visible to the vaccinating professional, and the UI must state that AutoVaxx has not evaluated clinical eligibility.

**Consequences.** Security exposure and clinical-change governance are smaller. Clinically, the professional remains fully responsible for assessment under the facility’s existing process. Regulators and reviewers must not be shown “clear” or “eligible” states. Operational training must explain unsupported clinical logic. Engineering should retain a versioned deterministic rule-package boundary, but Phase 1 must not build a general rules DSL.

**External verification and deferred work.** A Puerto Rico-licensed clinical owner must review the documentation checklist, attestation display, expired-product behavior, screening template, and unsupported-scope language before a clinical pilot. Every future clinical rule package requires separate scope approval, sources, fixtures, override policy, versioning, and release evidence.

### Decision 2 — First supported operating system

**Status: APPROVED 2026-08-30.** Windows 11 x64 is the sole MVP production target; Windows and macOS are development hosts.

**Why it matters.** Each production OS multiplies installer signing, webview, native encryption library, secret store, backup path, model runtime, peripheral, update, and support testing. Tauri uses Microsoft Edge WebView2 on Windows and supports MSI/NSIS packaging; Windows installers must be produced and tested on Windows. See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) and [Windows installer guidance](https://v2.tauri.app/distribute/windows-installer/).

**Options.** Windows 11 x64 only has the smallest test/support matrix and best fit for the stated facility environment. Windows plus macOS doubles production validation and keychain/signing behavior. Adding Linux introduces distro, webview, packaging, secret-store, driver, and support fragmentation without a demonstrated deployment need.

**Recommendation.** Make **Windows 11 x64 the sole MVP production target**. Developers may work on Windows or macOS, but a macOS development pass is not production certification. Release, SQLCipher, DPAPI, installer/update, Ollama, whisper.cpp, keyboard-wedge scanner, printer, backup/restore, and offline acceptance tests must run on representative Windows 11 x64 hardware.

**Consequences and migration.** This reduces operational and engineering burden but excludes facilities that cannot use Windows 11. Phase 1 should avoid Windows APIs in domain/application crates; Windows-specific behavior stays behind adapters. macOS and Linux production support each require a new packaging/security/peripheral matrix and are deferred.

**External verification.** Before real PHI, record the facility’s Windows edition, patching/device-management policy, CPU/RAM/storage, WebView2 policy, printer/scanner models, endpoint protection, local-admin restrictions, and installer/update channel.

### Decision 3 — Database encryption

**Status: RECOMMENDED.** The architecture can begin, but a successful implementation spike blocks Phase 1 exit and all real PHI.

**Why it matters.** A copied local database, journal, or temporary file must not expose PHI. Field encryption alone leaves schema/index/metadata gaps and complicates queries, migrations, and audit atomicity. Full-disk encryption is valuable deployment defense but does not replace application-controlled database/backup protection when files are copied from an unlocked device.

**Options.** SQLCipher preserves the SQLite API and encrypts database and journal pages with per-page integrity checks. Application-level field encryption offers selective cryptographic boundaries but creates key, query, migration, and missed-field risk. Filesystem-only encryption is operationally useful but is not portable with the database and may be transparent while the OS session is unlocked. Building another encrypted storage engine is unjustified.

**Recommendation.** Use a SQLCipher-compatible build as the production database. Supply a random raw key from the `SecretStore`; do not derive it from an application login password. Keep `cipher_plaintext_header_size` disabled unless a proven platform need requires otherwise. Use the library’s supported backup/export and rekey mechanisms rather than file copying an open database. SQLCipher documents page encryption/integrity, raw key support, encrypted journals, and `rekey`; see [design](https://www.zetetic.net/sqlcipher/design/), [API](https://www.zetetic.net/sqlcipher/sqlcipher-api/), and [key material guidance](https://www.zetetic.net/sqlcipher/database-key-material/).

**Consequences.** The Phase 1 spike must compare maintained Rust bindings/builds, SQLite feature compatibility, Windows x64 packaging, migrations, WAL/journal/temp behavior, corruption detection, backup/restore, performance, security updates, attribution, and Community versus Commercial licensing/support. SQLCipher Community and Commercial distributions have different licensing/support terms; no package choice is approved by this document. See [SQLCipher license information](https://www.zetetic.net/sqlcipher/license/).

**Synthetic exception.** Clearly marked development/test databases may be plaintext only when the build and database are irreversibly classified `SYNTHETIC_ONLY`, display a persistent warning, cannot be upgraded in place to PHI mode, and are rejected by production builds. Real PHI is blocked unless database and backup encryption are active and tested.

### Decision 4 — Encryption key management

**Status: RECOMMENDED.** Recovery custody requires external IT/security approval before PHI; the design can be prototyped with synthetic data.

**Why it matters.** A database key stored beside the database negates file-theft protection. Conversely, a key bound only to one Windows profile or device can make a valid backup unrecoverable after device loss.

**Options and tradeoffs.** A user-password-derived database key couples record availability to password resets and encourages weak human entropy. A key stored in a local file is portable but offers little theft protection. User-scoped DPAPI is stronger isolation but can fail under shared/multiple Windows accounts and profile loss. Machine-scoped Windows protection plus restrictive access controls fits a managed single workstation, but a local administrator remains in the trust boundary. A hardware/enterprise vault improves custody at significant deployment cost.

**Recommended design.**

1. Generate a cryptographically random 256-bit database encryption key (DEK) for each database at protected initialization. Never reuse it for backup encryption.
2. Generate or retrieve installation key-encryption material through a Windows `SecretStore` adapter. Protect the stored DEK using Windows DPAPI/CNG-compatible facilities and restrictive filesystem/credential ACLs appropriate to the chosen deployment account model.
3. Do not expose the database key to React, application users, logs, environment variables, process arguments, backup manifests, or support tools.
4. Treat a database replacement or restored database as having its own key identity. Store only opaque key IDs with data; never the keys.
5. Give every backup a new random backup DEK and encrypt a versioned backup envelope. Wrap that key for a facility-held portable recovery secret or other approved offline recovery mechanism, separate from the workstation. A designated custodian may see a one-time recovery secret; ordinary users never see cryptographic keys.
6. Restore to another authorized workstation only after the recovery secret is provided, the envelope is authenticated, the database is integrity-checked, and the new installation re-wraps the database key under its local secret store.

Microsoft documents that default DPAPI protection is tied to a user/machine context and that losing that context can make data unrecoverable; machine scope broadens which local accounts can decrypt and therefore must be paired with access control. See [Microsoft DPAPI example and limitations](https://learn.microsoft.com/en-us/windows/win32/seccrypto/example-c-program-using-cryptprotectdata) and [Windows data protection guidance](https://learn.microsoft.com/windows/uwp/security/data-protection).

**Consequences.** Security improves against copied files but not against a malicious OS administrator or malware running while the app is unlocked. Operationally, lost local key material without a valid portable backup/recovery secret means the live database is unrecoverable; the product must say so plainly. Engineering must model unavailable key store, corrupted wrapped key, cloned installation, recovery-secret error, and partial rekey failures.

**Prototype gates before PHI.** Verify DPAPI/CNG/credential-store scope under the actual Windows account model; ACLs; install/upgrade/uninstall behavior; key zeroization limits; backup restore to a clean workstation; recovery-secret rotation; database `rekey` with pre-operation backup and rollback; and total key-loss behavior. Enterprise escrow, HSM/TPM-bound keys, and automated rotation are deferred.

### Decision 5 — Backup and restore model

**Status: RECOMMENDED.** Retention details do not block Phase 1; tested recovery blocks real PHI.

**Why it matters.** Local-first removes cloud dependency but concentrates availability risk on one device, its keys, and the facility’s backup habits.

**Options.** Manual encrypted backup is simple and intentional but easy to forget. Configurable scheduled local backup improves recovery point but needs destination monitoring, retention, disk-full behavior, and user notification. The MVP should support both because a single workstation is the primary availability risk.

**Recommendation.** Implement both auditable manual and configurable scheduled encrypted local backup, with an explicitly selected local/removable destination and no cloud or implicit network target.

**Backup concept.** A versioned AutoVaxx backup container contains a minimal non-PHI header (format version, creation software/schema versions, cryptographic algorithm identifiers, opaque installation/backup IDs), authenticated encrypted payload, and integrity/authentication data. The encrypted payload contains a transactionally consistent encrypted database snapshot plus required content/rule metadata. The header is authenticated as associated data. Cryptographic primitives and libraries must be selected through security review; do not design a proprietary cipher.

**Workflow and controls.**

- A facility administrator may create an on-demand backup or configure a future schedule/destination. Restore staging and cutover require authentication within the previous five minutes.
- Scheduled backups run only to an explicitly configured local path or removable volume; there is no cloud or implicit network destination.
- Removable media is supported as a selected destination because it enables off-device recovery, but the application neither requires nor claims that the media itself is encrypted.
- Creation uses a supported consistent-snapshot API, verifies the resulting container, records success/failure as an audit event, and reports safe non-PHI operational status.
- Restore always stages into a new location, authenticates/decrypts, checks schema compatibility, SQLite integrity/foreign keys, audit chain, current-revision pointers, and content/rule references, then shows a summary. It never silently overwrites the active database.
- Cutover requires a second explicit confirmation and preserves the prior database as a protected rollback artifact until the approved retention window expires.
- Backup retention is configurable by count/age only after facility policy defines it. Disk-full, missing-media, overdue-backup, and failed-verification states are visible and auditable.

**Consequences and external verification.** Security depends on recovery-secret custody; operations must define backup owner, frequency, off-device storage, RPO/RTO, restore authority, drills, retention, legal hold, and media disposal. Cloud backup, automatic network-share discovery, and centralized backup are deferred.

### Decision 6 — Local user authentication

**Status: APPROVED 2026-08-30.** The MVP uses application-local named accounts and Argon2id verifiers; Windows identity integration remains deferred.

**Why it matters.** Administration and corrections must be attributable even when several workers use one physical pharmacy computer, and authentication must still work offline.

**Options.** Application-local accounts work on standalone/shared pharmacy PCs and keep authorization behavior consistent. Windows-account-only authentication reduces passwords but binds the product to facility account configuration and does not by itself express application roles. Supporting both doubles recovery/session testing and is premature.

**Recommendation.** Use named application-local username/password accounts. Store only versioned Argon2id password verifiers with unique random salts; choose parameters by benchmark and security review on minimum hardware. Normalize usernames for uniqueness but preserve display names separately. Never store recoverable passwords or password hints. Argon2id is specified in [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html); this review does not freeze parameters without the hardware benchmark.

**Session policy.** Default inactivity lock is 15 minutes, configurable by approved facility policy within a safe bounded range and never disabled in production. Lock on OS workstation lock where reliably detectable and clear sensitive UI state. Administration confirmation, clinical override, correction, void, PHI export, backup restore/export, role/user changes, and security configuration require authentication within the previous five minutes. Five failed attempts within 15 minutes trigger a 15-minute account lock by default, with increasing delays and an audit event; responses must not reveal whether a username exists. Exact values remain policy-configurable within tested bounds.

**Lifecycle and recovery.** Administrators can disable accounts immediately, reset another user to a forced password change, and view role-effective dates. No logged-in user can disable the last viable administrator without a tested recovery route. The MVP has no vendor master password, hidden bypass, recoverable user password, or anonymous emergency mode. Facility policy must choose either a second administrator or an approved offline installation-recovery procedure before PHI use; all recovery resets are prominent and audited.

**DEV_ONLY behavior.** A development shortcut, if needed, exists only behind a compile-time development feature, opens only `SYNTHETIC_ONLY` databases, displays a persistent banner, is rejected by release builds/CI, and cannot be enabled by runtime configuration. Production binaries fail closed if shortcut artifacts are detected.

**Consequences and future path.** Local account provisioning/recovery adds facility work but avoids an identity integration project. Windows Hello, Windows/Entra/AD federation, passkeys, smart cards, and MFA can later implement the same identity/session ports.

### Decision 7 — Authorization model

**Status: DECIDED.** Users may hold multiple roles; effective permissions are the union, with explicit separation-of-duty checks where a future policy requires them. Rust application services authorize every operation.

**Why it matters.** Authentication identifies a user; authorization determines whether that user may perform a clinical, disclosure, administrative, or audit action.

**Options and tradeoffs.** A single “staff” role is simple but grants excessive authority. Fixed roles alone are understandable but become rigid. Full attribute-based/custom policy is flexible but overengineered for one facility. The MVP uses four understandable roles backed by named permissions, allowing multiple roles per user while keeping future policy migration possible.

**Recommendation.** Enforce the matrix below in Rust and at state-transition/repository invariants; frontend visibility is presentation only.

Legend: **A** allowed; **R** allowed with recent re-authentication; **V** read/view only; **—** denied unless the user separately holds an allowing role.

| Operation | Vaccinating professional | Clinical support | Facility administrator | Auditor/privacy reviewer |
|---|---:|---:|---:|---:|
| Search/view patient | A | A | — | V |
| Create patient/update draft demographics | A | A | — | — |
| Correct demographics referenced by finalized history | R | — | — | — |
| Create encounter | A | A | — | — |
| Enter screening answers | A | A | — | — |
| View documentation warnings | A | A | — | V |
| Acknowledge/override an allowed warning | R | — | — | — |
| Document consent/refusal/withdrawal evidence | A | A | — | — |
| Document VIS delivery | A | A | — | — |
| Select vaccine / enter lot and administration details in draft | A | A | — | — |
| Confirm physical administration | R | — | — | — |
| Finalize record | A, or R if confirmation re-authentication is stale | — | — | — |
| Correct or void post-attestation record | R | — | — | — |
| Generate patient-facing clinical documents | A | A before finalization; V after | — | V when within audit scope |
| Create registry candidate | A | — | — | V |
| Authorize PHI file export/future transmission | R | — | — | — |
| Manage facility nonclinical configuration | — | — | A | — |
| Manage users/roles/security configuration | — | — | R | V for review |
| Configure/create scheduled or manual backup | — | — | R | V for evidence |
| Restore backup | — | — | R | V for evidence |
| Review audit history | Own/relevant clinical events | — | Security/admin events only | V |
| Export audit evidence | — | — | — | R, minimum necessary |

“View” is still permission-checked and patient access is audited. Facility administrator is not a super-clinician. A dual-role administrator/professional receives both permissions under one named identity. The database and state machine also enforce invariants so a forged Tauri call cannot bypass Rust.

**Consequences and verification.** Clinical/legal owners must confirm which licenses/titles qualify for vaccinating permissions and whether correction, void, export, or audit export needs dual control. Phase 1 can implement these named capabilities with synthetic command-boundary denial tests. Custom roles are deferred.

### Decision 8 — Administration confirmation

**Status: DECIDED.** This is a non-delegable, explicit clinical attestation by a user holding `CONFIRM_ADMINISTRATION`.

**Why it matters.** This is the moment the system records that a physical vaccine administration occurred; accidental, delegated, inferred, or replayed confirmation would create a false clinical record.

**Options and tradeoffs.** Autosave/passive confirmation minimizes clicks but is unsafe. A single explicit click without fresh identity is weak on shared computers. Re-authenticated explicit attestation against a visible snapshot adds friction but provides the strongest practical MVP accountability. Routine two-person approval would add burden without a verified requirement and is deferred unless policy requires it.

**Recommendation.** Require the re-authenticated single-professional snapshot attestation and atomic transition described below.

**Attested view.** Immediately before confirmation, the professional sees patient identity, vaccine/product and code snapshot, manufacturer, lot, expiration, dose/unit, route/site, asserted administration time, facility/workstation, vaccinator identity, screening-completion status and answers, consent evidence, applicable VIS edition/language/delivery time, unresolved blocks/warnings, and a clear statement that the MVP has not determined clinical eligibility.

**Preconditions.** Recent re-authentication and expected revision are required. Missing required documentation, unanswered required screening, absent consent/VIS evidence, VIS after administration, expired product at the asserted administration time, product/lot mismatch, invalid dose/route/site encoding, unsupported workflow state, or wrong role blocks confirmation. Documentation-only blocks are not overrideable merely for convenience. Nonblocking warnings require explicit acknowledgement; any future clinical-rule override must be allowed by the approved rule package and requires a coded reason plus narrative.

**Transaction semantics.** Confirmation records the professional, professional credential snapshot, UTC and local time, IANA zone/offset, workstation/facility, session authentication context, and a cryptographic fingerprint/list of all reviewed revision IDs. One transaction appends the immutable attested immunization revision, state transition to `ADMINISTERED_PENDING_DOCUMENTATION`, and audit event. AI/provider identities cannot call or be delegated this command.

**Crash and late entry.** If the transaction commits, restart returns the exact pending-documentation state. If it does not commit, no administration attestation exists; the application must not guess whether the physical act occurred. The professional can create an explicit late-entry attestation with the actual administration time and reason under approved policy.

**External verification.** A Puerto Rico-licensed clinical reviewer and facility policy owner must approve the attestation language, required review display, nonoverrideable checks, late-entry process, and any future clinical warning override.

### Decision 9 — Finalization versus administration

**Status: DECIDED.** Administration and finalization are distinct explicit actions.

**Why it matters.** A dose can be physically given even if the application crashes or noncritical documentation remains; combining the facts risks either losing the dose assertion or falsely claiming a complete record.

**Options and tradeoffs.** One combined action is convenient but cannot represent interruption. Automatic finalization when fields appear complete hides a legal/clinical record transition inside validation. Two explicit states/actions add one deliberate step and support honest crash recovery.

**Recommendation.** Keep both states and never auto-finalize; reuse the recent administration re-authentication only within the defined five-minute window.

`ADMINISTERED_PENDING_DOCUMENTATION` is created only by the successful transaction in Decision 8. At that moment all facts necessary to identify the physical act must be present and immutable. Only post-administration narrative, document-generation metadata, or registry-only fields that policy does not require at attestation may remain incomplete; the application must list them. Missing patient identity, product, lot/manufacturer/expiration, dose/unit, route/site, vaccinator/facility, administration time, screening completion, consent, or VIS evidence cannot be deferred.

`FINALIZED` requires deterministic documentation validation for the selected documentation profile, resolution of all finalization blocks, an authenticated vaccinating professional, expected revision, and a second deliberate action. It never happens automatically. If the professional’s administration re-authentication is still within five minutes, no additional password prompt is needed; otherwise finalization requires re-authentication. Finalization appends its own revision/transition/audit event atomically.

After a crash, committed administration reopens pending documentation; committed finalization reopens locked/finalized. The application never rolls a state forward from UI memory. Late completion and abandonment escalation require facility policy before pilot.

### Decision 10 — Corrections, voids, and immutability

**Status: DECIDED.** Post-attestation and finalized clinical facts are append-only.

**Why it matters.** In-place editing makes it impossible to prove what was originally attested or exported and can silently rewrite clinical history.

**Options and tradeoffs.** Mutable rows plus timestamps are simple but insufficiently preserve prior values. Full event sourcing preserves everything but adds unnecessary projection complexity. Stable roots plus immutable relational revisions preserve history while remaining natural in SQLite.

**Relational recommendation.** Keep a stable root (`immunization_event`) and immutable `immunization_revision` rows with monotonically increasing revision number, `supersedes_revision_id`, status, correction/void reason code and narrative, actor, occurred/recorded timestamps, and the evidence/attestation/finalization snapshot. A guarded `current_revision_id` is a convenience pointer updated in the same transaction; it does not erase history. Documents and registry artifacts reference exact revision IDs and are immutable.

**Operation semantics.**

- **Cancel before administration:** ends a draft/encounter without an administration event; reason and audit remain.
- **Correct documentation:** appends a revision that preserves the assertion that administration occurred while changing identified facts; requires `CORRECT_RECORD`, recent re-authentication, reason, expected current revision, and deterministic revalidation.
- **Void erroneous administration record:** appends a `VOID` revision stating that the recorded event must not be treated as a valid administration. It does not assert reversal of a physical dose and never deletes the original.

Regenerated documents and registry candidates use the new revision, receive new identifiers/hashes, and identify correction/void status. Previously exported artifacts remain in history and create an explicit reconciliation obligation; they are never rewritten. Live registry correction semantics remain disabled until PREIS verification.

**Consequences and external verification.** More rows and joins buy unambiguous history. Facility/legal policy must define authorized correction/void reasons, late entry, required notices, retention, and whether any action needs second-person review. Hard deletion and automated external correction are deferred.

### Decision 11 — Consent model

**Status: REQUIRES EXTERNAL VERIFICATION.** The data capability can be designed without claiming legal sufficiency.

**Why it matters.** Consent is both a workflow precondition and a policy/legal fact whose required actor, method, evidence, and retention can vary.

**Software capability.** A versioned consent record represents: patient/representative identity snapshot; relationship and asserted authority evidence type; vaccine/procedure scope; language and interpreter; form/policy identifier and version; presentation/consent/refusal/withdrawal timestamps; method (for example verbal, written, electronic, witnessed, or imported artifact); optional artifact reference/hash and witness; recorder/reviewer; status; and correction chain. Refusal creates no administration. Withdrawal after administration does not erase the historical event.

**Options and tradeoffs.** A typed attestation is operationally simple but may not meet a facility requirement. Signature image/device capture adds biometric-like artifacts, device drivers, retention, and evidentiary questions. Scanned forms preserve existing workflow but add file/import risk. The model supports evidence types without making one universal.

**MVP recommendation.** Build the structured model and policy-driven requiredness. Do not make device-specific signature capture a Phase 1 or default MVP commitment. An optional artifact is encrypted and immutable if an approved workflow requires it. The UI states what was recorded, not that consent is legally valid.

**External questions.** Puerto Rico counsel/pharmacy policy must determine who may consent for which patients, representative authority evidence, remote/verbal/witness rules, required language/accessibility, withdrawal effects, electronic-signature sufficiency, form content/version, and retention. Clinical workflow owners must approve when missing/ambiguous consent blocks administration.

### Decision 12 — VIS content management

**Status: RECOMMENDED.** Exact stale-content policy requires clinical/legal approval before pilot, not before Phase 1.

**Why it matters.** Offline operation must deliver the exact applicable official material and later prove which edition/language was provided before the dose.

**Options and tradeoffs.** Live web retrieval may be current but breaks offline operation and makes availability part of administration. Unversioned bundled PDFs work offline but silently age. Versioned, integrity-checked packages add update governance while preserving reproducibility and safe offline use.

**Recommendation.** Install immutable, signed/versioned local content packages containing vaccine/CVX association, VIS type/GDTI where applicable, official edition date, language, publisher/source URL, exact local file, media type, cryptographic hash, package version, retrieval/installation date, verifier, and currency metadata. Delivery records reference the exact artifact and record the edition date and `provided_at` time.

CDC says the applicable current VIS must be provided before each dose for covered vaccines and specifies edition/provided dates in the record. CDC also publishes current VIS and mapping tables. See [CDC VIS instructions](https://www.cdc.gov/vaccines/hcp/about-vis/instructions.html), [current VIS index](https://www.cdc.gov/vaccines/hcp/current-vis/index.html), and [VIS URL/mapping information](https://www.cdc.gov/iis/code-sets/vis-url-table.html).

**Update and offline behavior.** Phase 1/initial MVP uses an administrator-imported signed package obtained outside the patient workflow. Import verifies publisher allowlist, package signature, file hashes, manifest/schema, and version/effective dates; activation is audited and prior packages remain for historical reproducibility. Offline operation uses the installed package and displays its last verified date. Failure to reach the internet never silently marks it current.

Known superseded, expired, corrupted, missing, or unknown-currency applicable VIS content should block administration by default. Any emergency override must be an externally approved policy with authorized actor, reason, and audit; engineering must not invent it. Official content may be rendered but never generated, rewritten, summarized as a substitute, or translated by AI. Automatic online updates are deferred.

### Decision 13 — AI provider architecture

**Status: DECIDED.** The domain depends on `LocalAiProvider`, never on Ollama-specific types.

**Why it matters.** Provider APIs, models, hardware behavior, and runtime packaging change; none should leak into clinical state or grant the model authority.

**Options and tradeoffs.** Direct Ollama calls are quickest but couple the application to one API. Embedding one runtime simplifies connectivity but enlarges native packaging and patching. A narrow provider port with a separately provisioned local runtime adds an adapter but keeps manual documentation independent and permits llama.cpp later.

**Recommendation.** Use the fake-first provider contract below, implement Ollama first only after the foundation phase, and keep runtime management outside the MVP application.

**Conceptual contract.** The provider reports health/readiness, local endpoint identity, available model descriptors/digests, capabilities (including schema-constrained output), and performs a bounded extraction request returning untrusted structured proposals with provider/model/prompt-template/schema/decoding provenance. It supports an absolute wall-clock deadline, cooperative cancellation, ownership-specific hard termination, and typed unavailable, timeout, hard-terminated, termination-failed, malformed-output, unsupported-capability, out-of-memory, provider-disk-full, resource-limit, model-digest-mismatch, and policy-denied errors. It has no domain-write, workflow, file, credential, registry, or arbitrary tool capability.

The `OllamaProvider` is first; `LlamaCppProvider` may follow. Both upstream runtimes support local HTTP interfaces and structured output mechanisms, but provider differences remain inside adapters. See [Ollama API](https://docs.ollama.com/api/introduction) and [llama.cpp server](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md).

**Runtime ownership.** The initial MVP connects to a separately installed, facility-provisioned Ollama runtime. AutoVaxx does not download models, launch an elevated service, edit provider configuration, silently pull a model while a patient is open, or terminate the facility-managed service. On timeout/cancellation it aborts the request, quarantines the provider for the active assist session, and returns to manual documentation. A later packaging decision may manage a pinned child process after supply-chain, licensing, patching, sandbox, resource, and support review; that process tree must run in a Windows Job Object with memory limits and hard termination after the absolute deadline.

**Identity, locality, and failure.** Resolve and validate the destination as IP loopback on every patient-bearing request; reject remote names, proxies, redirects, wildcard binds, and cloud endpoints. Verify the runtime-reported model digest against the approved synthetic-evaluation manifest before the first patient-bearing assist session. The frontend cannot contact the provider. Timeouts/cancellation return to manual entry, do not mutate drafts, and never relax deterministic validation. Out-of-memory and isolated provider-runtime disk-full errors bypass retry/repair; unknown/shared-volume or clinical persistence exhaustion remains a blocking integrity failure. No OpenAI, Anthropic, Google, or other cloud AI API may receive PHI.

### Decision 14 — AI data retention

**Status: RECOMMENDED.** Raw-retention expansion requires a new privacy/security decision.

**Why it matters.** Patient prompts, transcripts, model responses, and rejected guesses can duplicate or expand PHI beyond the clinical record without improving care.

**Options.** Retaining full prompts/responses improves debugging but duplicates high-risk PHI and makes provider behavior part of the clinical record. Retaining nothing loses evidence that assistance influenced an edit. Minimum structured provenance provides accountability with much less sensitive duplication.

**Recommendation.** During a short-lived review session, hold the minimum raw source, response, structured proposals, source spans, and confidence needed to let the user verify suggestions. On accept/reject/cancel/timeout/logout/crash recovery, delete raw prompts, interview text/transcript, raw model response, rejected values, and spans after the approved short recovery window. Do not place them in logs or backups.

Persist only: assist-session ID; purpose/source type; provider and approved model identifier/version/digest; prompt-template identifier/version/hash and schema/decoding version (not prompt content); time; deterministic validation outcome; target field names; reviewer identity/time; accepted/rejected/cancelled disposition; cleanup disposition; and references to the resulting ordinary clinical revision. The audit event for an accepted edit references the assist session and prompt-template hash. Accepted clinical values exist once in that revision; rejected values are not retained. Confidence may be retained only as nonclinical provenance tied to the decision, never as clinical certainty.

**Consequences.** Debugging relies on synthetic eval fixtures and reproducible template/model versions, not patient prompts. Source spans are useful only during review unless the source itself is deliberately retained as an approved clinical note. Any future raw retention requires purpose, access, consent/policy, encryption, retention, deletion, backup, and disclosure review.

### Decision 15 — Speech and audio retention

**Status: RECOMMENDED.** The first MVP does not retain recordings.

**Why it matters.** Raw audio can contain extensive incidental PHI and creates consent, access, backup, disclosure, and deletion obligations that transcription assistance does not require.

**Options and tradeoffs.** In-memory/pipe audio minimizes residue but may be harder with some runtimes. Plaintext temporary files are incompatible with the approved design; SSD wear leveling also makes ordinary overwrite an unreliable sanitization claim. Per-session encrypted temporary files are compatible but need key sanitization and crash cleanup. Retained recordings aid replay but create a major new record class. The MVP uses memory/pipe when practical and ciphertext-only temporary files otherwise, with no retention feature.

**Recommendation.** Prefer bounded in-memory/anonymous-pipe transfer to a controlled whisper.cpp process. If a file is unavoidable, encrypt the validated audio bytes with a fresh per-session key held outside the file before writing to an application-private temporary directory with restrictive ACLs, a random nonidentifying filename, size/duration limits, and no path in logs or process arguments. On success, failure, cancellation, timeout, session lock, or logout, sanitize the key, close handles, and delete the ciphertext. At startup, remove only validated AutoVaxx-owned orphaned ciphertext and record a non-PHI cleanup result. Pagefile, hibernation, crash-dump, endpoint-security collection, and device-encryption policy require Windows evidence before real PHI.

whisper.cpp’s example server accepts uploaded audio and warns that it should be sandboxed and inputs validated; the example is not itself a production security architecture. See [whisper.cpp server guidance](https://github.com/ggml-org/whisper.cpp/blob/master/examples/server/README.md).

The transient transcript follows Decision 14. The MVP offers no “retain recording” option. This reduces storage, consent, access, backup, disclosure, and breach scope. If a facility later proves a clinical need, recording retention becomes a separate feature with consent, encryption, access, retention, export, correction, and deletion policy.

### Decision 16 — Barcode scope

**Status: DEFERRED.** Barcode semantics are not part of Phase 1.

**Why it matters.** A scanner can improve speed, but an unresolved or mis-mapped product/lot code can create a precise-looking clinical error.

**Options and tradeoffs.** Manual entry has more typing but clear human ownership. Keyboard-wedge capture adds little device complexity. Native drivers, semantic GS1 parsing, installed code resolution, and external lookup add progressively greater validation, supply-chain, network, and support burdens.

**Layered scope.** Scanner input, symbology parsing, identifier extraction, product/manufacturer resolution, lot/expiration extraction, and external lookup are separate capabilities. Treating them as one feature risks turning a simple keyboard-wedge device into an unverified product database.

**Recommendation.** Phase 1 defines a `BarcodeInput`/parser boundary and fake adapter only. If the first vertical slice includes scanner convenience, support keyboard-wedge input into a focused field with length/character/rate limits. Preserve the raw scan only transiently, label every parsed value as a proposal, validate it against installed versioned code data, and require human review. On unknown/ambiguous resolution, show “unresolved” and require manual selection/entry; never infer manufacturer/product/lot/expiration.

Native device APIs, GS1/2D parsing, product-code mappings, camera scanning, signature hardware, and external lookup services are deferred until representative scanners and authoritative code sources are verified. External lookup with patient context is prohibited without a new disclosure/network review.

### Decision 17 — PREIS scope

**Status: REQUIRES EXTERNAL VERIFICATION.** No live PREIS capability is authorized.

**Why it matters.** Registry conformance and acceptance depend on the current jurisdiction profile and onboarding contract, not merely on producing syntactically plausible HL7.

**Options and tradeoffs.** Embedding PREIS fields into the domain couples history to an external profile. Implementing live transport from the 2022 guide risks incorrect disclosure and false acceptance claims. Canonical data plus a versioned adapter boundary allows safe progress; rendering waits for a verified profile and transport waits for conformance.

**Recommendation.** Implement only canonical/port foundations in Phase 1 and apply the staged MVP boundary below.

**MVP boundary.** Build the canonical model, deterministic registry-validation port, versioned PREIS mapper boundary, immutable candidate artifact, and human-readable inspection. Render a PREIS candidate only after a specific profile has status `VERIFIED_FOR_RENDER`. “Registry ready” means only that the exact local artifact passed that local versioned validation; it never means sent or accepted.

The official PRDoH guide currently available to this project is an April 2022 local HL7 v2.5.1 guide describing VXU/ACK and HTTPS exchange concepts. It is evidence for discovery, not proof of the current production contract, endpoint, onboarding, credentials, tests, or field usage. See [PRDoH PREIS local HL7 guide](https://www.salud.pr.gov/CMS/DOWNLOAD/8575).

**PRDoH discovery checklist.** Record responder, date, source, and document hash for every answer:

1. Current implementation guide name, version/revision, effective/retirement dates, and errata.
2. Supported business operations/messages (including VXU/QBP/RSP if any) and whether batch/real-time modes differ.
3. Required/required-if-available/optional/prohibited fields, segments, cardinalities, code systems, value sets, and Puerto Rico extensions.
4. Patient identity/deduplication fields, assigning authorities, two-surname handling, and demographic requirements.
5. Immunization, refusal, historical dose, correction/delete/void, funding, eligibility, facility, provider, VIS, and observation requirements.
6. Current CVX/MVX/NDC and local code-set sources, update cadence, and conformance expectations.
7. Enrollment, agreements, facility/provider identifiers, credentialing, contacts, and approval owners.
8. Test and production base URLs, transport protocol, HTTP/SOAP details, TLS/certificate/mTLS requirements, IP allowlisting, and DNS policy.
9. Credential type, issuance, storage, expiry, rotation, revocation, recovery, and separation between test/production.
10. Message/batch size, ordering, rate limits, timeouts, maintenance windows, and downtime procedures.
11. ACK format and timing; accepted/warning/rejected semantics; application versus transport acknowledgement; error code catalog.
12. Retry, idempotency, duplicate detection, partial-batch failure, replay, reconciliation, and correction/void behavior.
13. Test cases, synthetic-data rules, conformance/certification process, expected evidence, and production cutover criteria.
14. Submission timing obligations and manual contingency/reconciliation process.
15. Minimum-necessary data approval, retention of payloads/ACKs, audit expectations, and incident/support escalation.
16. Whether registry candidate files/manual upload are permitted, and the approved format/handling if so.

Enrollment, endpoint configuration, credentials, transport, ACK processing, retry/reconciliation, conformance testing, and production transmission remain deferred and disabled.

### Decision 18 — Canonical immunization model

**Status: DECIDED.** The model is independent from HL7 segments, PREIS transport, UI forms, and AI schemas.

**Why it matters.** The internal record must preserve the clinical assertion and its provenance even when screens, AI schemas, code sets, or registry profiles change.

**Options and tradeoffs.** An HL7/PREIS-shaped schema simplifies one mapper but distorts local meaning and becomes brittle. An all-JSON document is flexible but weakens constraints and queries. A typed relational aggregate with bounded versioned extensions provides integrity while preserving mapping flexibility.

**Proposed `VaccinationAdministration` aggregate.** “Required” below is an application recommendation for documenting an administered-here event, not a claim about current PREIS requirements.

| Value group | Canonical values | Classification |
|---|---|---|
| Identity/revision | event ID, revision ID/number, supersedes, status, source (`ADMINISTERED_HERE`/`HISTORICAL`), author/reason/times | Local workflow required; correction integrity |
| Subject/context | patient ID and exact patient revision, encounter, facility, workstation | Clinical/local required; registry mapping candidate |
| Product | internal product reference; CVX/NDC or other external-code references with system/version; display snapshot | Clinical required; external-code references; registry mapping candidate |
| Manufacturer/lot | manufacturer reference/MVX snapshot, lot number, expiration date | Clinical documentation required for administered-here event; registry mapping candidate |
| Administration | actual local datetime, UTC instant, zone/offset, dose amount/unit, route, anatomical site/laterality when applicable | Clinical required; registry mapping candidate |
| Professionals | administering professional identity/credential snapshot; ordering professional when applicable | Clinical/local required; some values profile-dependent |
| Evidence | screening response revision, consent revision, applicable VIS delivery revisions, documentation validation | Local workflow required; selected values may map externally |
| Attestation/finalization | confirmer/time/re-auth context, reviewed revision fingerprint, finalizer/time/profile/result | Local workflow and accountability required; generally not raw registry fields |
| Provenance | original source, recorder, late-entry indicator/reason, historical-source confidence | Required when applicable; registry mapping profile-dependent |
| Program/financial | funding source, eligibility/program fields | Optional canonical extension until workflow or verified profile requires it |
| Registry lifecycle | validation/artifact/submission references | Derived external workflow; never part of the physical administration fact |

Derived display names, age, “expired at administration,” completeness, hashes, and registry fields are recalculated from immutable inputs and versioned rules; they are not silently stored as new clinical facts. External code references include system, code, version/effective date, and historical display snapshot. CDC’s IIS core-data material is an input to interoperability discovery, not a substitute for current PREIS verification; see [CDC IIS Core Data Elements](https://www.cdc.gov/iis/core-data-elements/).

### Decision 19 — Patient identity and duplicates

**Status: RECOMMENDED.** Minimum fields and matching policy require operational/PREIS verification before pilot.

**Why it matters.** Wrong-patient documentation is clinically dangerous, while forcing Puerto Rico names into a single surname field weakens search, display, and registry mapping.

**Options and tradeoffs.** A single free-form name is simple but loses structure. A U.S.-centric first/middle/last model mishandles two surnames. Automatic probabilistic merge reduces duplicate counts but can irreversibly combine patients. Separate name components plus deterministic candidate evidence and human disposition is safer for the MVP.

**Recommendation.** Use the canonical identity and non-merging duplicate process below.

**Canonical identity.** Preserve legal given name(s), middle name(s), first surname, second surname, suffix, preferred/display name, date of birth, structured address/municipality/postal code, phone/email when needed, preferred language, and typed external identifiers with assigning authority, type, validity, and provenance. Do not force two surnames into one last-name field. Do not collect SSN merely because a format could carry it.

Sex/gender concepts must not be conflated. Store only distinctly named coded fields that are required for care or a verified registry profile, with code system/version, source, and explicit unknown/declined semantics. PRDoH must confirm the current registry field and allowed values; engineering must not relabel one concept as another.

**Duplicate strategy.** Search uses normalized exact/phonetic tokens only as decision support. Deterministic candidate rules may combine authoritative external identifier matches, normalized names (including surname order/diacritics), DOB, phone, and address. Exact assigning-authority identifier conflicts are high priority but do not auto-merge. Candidate scores expose matched/conflicting fields and algorithm version without declaring identity.

A user may select an existing patient, keep records separate with a disposition reason, or escalate for authorized review. The MVP has no merge. A future merge/unmerge workflow must preserve both roots, revisions, external identifiers, actor/reason, and audit history.

### Decision 20 — Local time and timestamps

**Status: DECIDED.** Store time so both the instant and the clinical local assertion can be reconstructed.

**Why it matters.** Administration, VIS, consent, audit, late entry, and exports depend on ordering and on what local time the professional actually asserted.

**Options and tradeoffs.** UTC-only loses the entered local context. Local-time-only is ambiguous outside its zone and across future daylight-saving zones. A fixed Puerto Rico offset is currently convenient but not portable. UTC plus IANA zone, offset, and entered local value carries modest storage cost and preserves both meanings.

**Recommendation.** Use the multi-part representation below for clinically meaningful times and date-only types for date-only facts.

Use UTC instants in an unambiguous RFC 3339-compatible representation (or equivalent typed integer epoch internally), plus the IANA timezone, numeric UTC offset at entry, and user-entered local datetime for administration, consent, VIS delivery, and other events whose local ordering matters. Date-only facts such as birth date, lot expiration date, and VIS edition date remain dates; never invent midnight or a timezone.

The initial facility timezone and display default is `America/Puerto_Rico`; do not hard-code a permanent `UTC-04:00` assumption into domain logic. UI displays the facility zone and flags a workstation/facility zone mismatch. `occurred_at` (when the real event happened) and `recorded_at` (when AutoVaxx committed it) are separate. Late entry requires both and a reason.

Exports derive their timestamp syntax from the verified registry profile using the preserved local value/offset. Clock access is injected and tests cover midnight/date boundaries, changed workstation clocks, malformed offsets, and future zones with daylight-saving transitions.

### Decision 21 — Operational logging versus clinical audit

**Status: DECIDED.** These are separate data systems with different purposes.

**Why it matters.** Full-payload logs are a common PHI leak, while eliminating accountable patient-linked audit would undermine access and change review.

**Options and tradeoffs.** One verbose log is easy to debug but unsafe and weakly controlled. No logs/audit prevents operations and accountability. Separate PHI-free diagnostics and encrypted minimum-necessary audit preserves both goals with additional schema/access work.

**Recommendation.** Enforce the separation below and provide no production “verbose PHI” switch.

| Concern | Operational logs | Clinical/security audit |
|---|---|---|
| Purpose | Diagnose component health and safe error classes | Attribute access, mutations, decisions, disclosures, and security actions |
| Content | event code, timestamp, component, opaque correlation ID, duration/count, non-sensitive error class, app/platform version | actor/session/workstation/facility, action/outcome, entity/root/revision references, changed field names, reason/policy code, occurred/recorded time, correlation/causation, software/schema version |
| PHI | Prohibited, including clinical values, free text, prompts, payloads, SQL parameters, and identifiers | Minimum patient-linked opaque references allowed; values remain in encrypted revisions |
| Storage | Separate local diagnostic sink with bounded rotation | Encrypted clinical datastore, append-only application path, backed up |
| Integrity | File permissions and structured schema | Atomic with mutation, ordered sequence/hash chain, integrity checks |
| Access | Support-safe, still least privilege | Role-restricted; reads/exports audited |

Developer diagnostics use synthetic reproduction, correlation IDs, safe counters, and local authenticated inspection—not a “verbose PHI” mode. Crash reporting is local-only and excludes database pages, memory dumps with PHI/secrets, frontend state, model inputs, and registry data. No cloud crash/analytics SDK receives patient data.

Facility policy sets operational-log duration, audit/record retention, review cadence, legal hold, export authority, and disposal before PHI. Hash chaining detects corruption/simple alteration but is not proof against a privileged attacker able to rewrite both database and application; stronger external anchoring is deferred.

### Decision 22 — Network policy

**Status: DECIDED.** Default deny is a product invariant.

**Why it matters.** A local-first application should not depend on ambient egress or let a generic URL/configuration silently become a PHI destination.

**Options and tradeoffs.** Broad outbound access is operationally easy but makes disclosure control unverifiable. A permanent air gap blocks legitimate updates and future registry exchange. Default deny with purpose-specific adapters/allowlists provides a safe offline core and an auditable path to later approved connections.

**Recommendation.** Permit only validated loopback/controlled child processes initially; add external categories separately under the policy below.

Phase 1 and the first documentation slice have no external network requirement. Rust owns egress policy; React/Tauri capabilities and CSP do not expose generic HTTP. Patient-bearing AI/speech may use only a validated IP-loopback endpoint or controlled child process. DNS names, redirects, proxies, wildcard listeners, and user-configured remote AI endpoints are rejected.

Future network categories are independent adapters and allowlists:

- **Software updates:** no PHI or facility identifiers; signed metadata/artifacts; administrator-controlled channel and rollback.
- **Clinical/VIS/content updates:** signed packages, publisher allowlist, staged verification/activation; no patient context.
- **PREIS:** separate test/production destinations, verified profile, TLS/certificate policy, credential reference, purpose, explicit re-authenticated authorization, artifact hashes, and audit.

An allowlist entry includes category, scheme, resolved host/IP policy, port, certificate/pinning policy where approved, environment, adapter/profile version, owner, approval/effective dates, and enabled state. Configuration cannot turn a generic URL into an approved disclosure. Network failure yields explicit unavailable/pending status and never weakens authentication, validation, or audit. Connectivity returning never automatically sends PHI.

### Decision 23 — MVP UI languages

**Status: RECOMMENDED.** Bilingual architecture does not block Phase 1, but bilingual patient-facing/clinical content review blocks pilot release for those languages.

**Why it matters.** Puerto Rico operations are bilingual, and hardcoded strings make later translation, accessibility, validation messages, and documents costly and error-prone.

**Options and tradeoffs.** English-only or Spanish-only would reduce initial translation effort but misfit the stated Puerto Rico workflow and make later string extraction/error handling expensive. Bilingual UI adds translation/review work but matches the operating environment and avoids a costly retrofit.

**Recommendation.** Ship English and Spanish UI chrome from the first vertical slice, with a locale service and message catalogs keyed by stable semantic IDs. No user-facing React string, validation message, date/number format, accessibility label, or document label is hardcoded outside the localization layer.

Language selection is per user/session with facility default and explicit fallback. Stored codes/facts are language-neutral; historical display snapshots record locale and content version where needed. Official VIS and screening/consent/clinical packages have independently reviewed language variants and never use live machine translation. Missing approved clinical content in the selected language is visible and follows approved policy; the UI must not imply a translation is official.

### Decision 24 — MVP boundaries

**Status: DECIDED.** The MVP is one production computer, one configured facility, named local users, a documentation-only vaccination workflow, encrypted local storage/backup, append-only history/audit, optional local proposal/transcription assistance, and a deterministic inspectable registry candidate after profile verification. Windows 11 x64 is the approved production OS in Decision 2.

**Why it matters.** Every additional integration, clinical claim, platform, device, or workflow expands validation and support before the core documentation-time hypothesis is proven.

**Options and tradeoffs.** A broad pharmacy platform offers more features but delays evidence and enlarges regulated scope. A bare data-entry tool misses the end-to-end workload problem. The chosen boundary is one complete, auditable documentation workflow with deliberately deferred adjacent systems.

**Recommendation.** Enforce the explicit exclusions below as phase gates rather than placeholders that can be enabled by configuration.

**Explicitly deferred:**

- live PREIS transport, credentials, ACK/retry/reconciliation, and automatic/background transmission;
- clinical eligibility, vaccine recommendation, schedule/forecasting, contraindication/precaution interpretation, diagnosis, or autonomous decisions;
- multi-workstation sync, shared SQLite files, multi-facility/multi-tenant service, cloud hosting, and cloud backup;
- cloud AI/speech, remote model endpoints, app-managed model downloads, and AI tools/actions;
- patient portal, scheduling, billing, claims, e-prescribing, full inventory/purchasing, and automated replenishment;
- automatic patient merge, record hard deletion, and policy-driven purge before retention is approved;
- semantic barcode/product lookup beyond a bounded proposal flow, camera scanning, signature-pad integrations, and broad import/OCR;
- EHR/FHIR interfaces, general file exchange, bulk administration/finalization, automated PHI export, and third-party analytics/crash reporting;
- additional production OS targets beyond the first product-owner-approved target, mobile/web clients, local server/broker/plugin marketplace, general rules DSL, and multi-workstation facility service.

Printer/document generation may use standard Windows printing/file adapters when required by the core workflow, but vendor-specific printer control and silent PHI output are excluded. Full historical immunization forecasting and broad vaccine inventory are also deferred.

## 4. FINAL MVP DECISIONS

Only already-decided constraints appear here; recommended choices move here after product-owner approval.

- Product architecture: local-first Tauri 2 desktop modular monolith; React/TypeScript/Vite presentation and Rust trusted core.
- Clinical scope: documentation completeness and workflow integrity only; no eligibility, recommendation, scheduling, forecasting, contraindication, precaution, diagnosis, or safety determination.
- Production platform: Windows 11 x64 only for the MVP; Windows and macOS remain valid development hosts, and platform behavior stays behind adapters.
- Authentication: application-local named accounts with Argon2id verifiers; no recoverable password, vendor master password, hidden bypass, or anonymous emergency account.
- Deployment shape: one computer and one facility initially; no shared SQLite file or synchronization.
- Data authority: canonical clinical data is independent of UI, AI, HL7, and PREIS.
- Database posture: SQLite-compatible transactional storage; real PHI is prohibited in plaintext.
- Clinical authority: AI is proposal-only; deterministic code performs validation; AI cannot change workflow state or invoke privileged actions.
- Administration: only a recently re-authenticated, authorized vaccinating professional can confirm a physical administration against a reviewed revision snapshot.
- Finalization: separate, explicit, never automatic; administration may persist safely as `ADMINISTERED_PENDING_DOCUMENTATION`.
- History: post-attestation/finalized clinical revisions and audit events are append-only; correction/void preserves prior values.
- Audit: every meaningful mutation and its audit event commit atomically.
- PHI locality: PHI remains on the workstation unless an authenticated user explicitly authorizes a defined approved disclosure.
- AI/speech locality: patient data never goes to cloud AI/speech; patient-bearing provider traffic is controlled local process or validated loopback only.
- Network: deny by default; the offline core workflow does not depend on internet access.
- PREIS: canonical/mapping boundaries may be built; live transmission is disabled until current PRDoH requirements and conformance are verified.
- Logs: PHI is prohibited in operational logs, crash reporting, analytics, and support artifacts; encrypted minimum-necessary audit is separate.
- Testing: synthetic patients only until all real-PHI gates pass.
- MVP exclusions: no cloud data plane, autonomous clinical decision-making, automatic patient merge, multi-workstation sync, or background PHI transmission.

## 5. PHASE 1 AUTHORIZATION — CLEARED

The following decision-review blockers were cleared by explicit product-owner approval on 2026-08-30:

1. **Decision 1 approved:** documentation-only first clinical scope.
2. **Decision 2 approved:** Windows 11 x64 as the only MVP production target.
3. **Decision 6 approved:** application-local named accounts as the initial identity boundary.
4. The [Phase 1 implementation contract](#9-phase-1-implementation-contract) was approved and Phase 1 was authorized on a non-`main` branch.

The final SQLCipher Rust distribution, recovery-secret custody, exact retention periods, legal consent policy, clinically approved content, and current PREIS profile did **not** block synthetic scaffolding. They remain explicit later gates. Phase 1 exit evidence is recorded in [PHASE_1_EXIT_REVIEW.md](PHASE_1_EXIT_REVIEW.md).

## 6. BLOCKERS BEFORE REAL PHI

1. Windows 11 x64 production hardware/account/peripheral/patching baseline and deployment risk analysis are approved.
2. The chosen SQLCipher build/license passes Windows packaging, migration, WAL/journal/temp, performance, corruption, update, and security review.
3. Database key protection, ACLs, total key-loss behavior, portable recovery custody, rekey, and clean-workstation restore are tested.
4. Manual and scheduled encrypted backup, off-device custody, integrity verification, staged restore/cutover, retention, RPO/RTO, and a restore drill pass.
5. Authentication, bounded timeout/lockout, account disable/reset/recovery, recent re-authentication, and the full Rust permission matrix pass bypass tests.
6. Puerto Rico legal/pharmacy review approves consent evidence, representative authority, correction/void, records/backup/audit retention, export/print, professional authority, late entry, and incident/downtime policy.
7. A Puerto Rico-licensed clinical owner approves the documentation checklist, screening/VIS/consent workflow, expired-product block, attestation, late-entry behavior, supported products/populations, translations, and synthetic fixtures; unsupported clinical scope is prominent.
8. Signed/versioned VIS and other content packages, official language sources, currency/stale policy, rollback, and offline behavior pass review.
9. No-PHI log/crash/temp/process-argument/clipboard/support-bundle tests and local AI/speech locality/cleanup tests pass.
10. The threat model, build/update supply chain, installer signing, dependency/model provenance, endpoint protection assumptions, incident response, breach assessment, downtime, media disposal, and support ownership are approved and tested.
11. The deploying organization documents its HIPAA security risk analysis and administrative/physical safeguards; software controls alone are not a compliance claim. See the current [HHS Security Rule summary](https://www.hhs.gov/hipaa/for-professionals/security/laws-regulations/index.html).
12. If registry-candidate output is enabled in the pilot, its exact profile is verified for render, mapping fixtures pass, and export handling is approved. Live transmission remains separately blocked.

## 7. EXTERNAL QUESTIONS

### Puerto Rico legal/pharmacy policy

- Which professional licenses/titles may administer, attest, finalize, correct, void, export, and supervise support staff?
- What consent methods, form content/version, representative authority evidence, witness/signature artifacts, language/accessibility, refusal, and withdrawal behavior are required?
- What are the record, consent artifact, audit, payload, backup, and operational-log retention/legal-hold/destruction requirements?
- What are the required late-entry, correction, void, patient-copy, print/export, downtime, and reconciliation procedures?
- Are any actions subject to dual control, mandatory notice, or specific professional credential display?
- What facility policies govern emergency access, password/account recovery, inactivity timeout, removable media, support access, and incident/breach response?

### Puerto Rico clinical review

- Approve Option A documentation-only scope and the exact disclaimer/unsupported-scope language.
- Approve the required screening template/answer states without interpreting eligibility.
- Approve required pre-attestation fields, expired-product blocking, mismatch/time checks, warning acknowledgement, and nonoverrideable conditions.
- Approve administration attestation, late-entry, pending-documentation, finalization, correction/void, and abandonment/escalation semantics.
- Approve supported product/workflow scope, code/content sources, VIS mapping/currency policy, and English/Spanish clinical content.
- Name the owner, review cadence, authoritative sources, fixtures, and release process for any later clinical rule package.

### PREIS / Puerto Rico Department of Health

- Resolve every item in the 16-point [PREIS discovery checklist](#decision-17--preis-scope).
- Specifically confirm the active profile/errata, permitted operations, required data, patient identity/two-surname handling, code sets, correction/void semantics, enrollment, endpoints, credentials, TLS, test cases, ACK/retry/idempotency, conformance, submission timing, and support contacts.
- Confirm whether an inspectable candidate/manual upload artifact is permitted and what handling requirements apply.

### IT/security deployment policy

- Confirm representative Windows 11 edition/hardware, Windows-account use, least privilege, patching, WebView2, endpoint protection, disk encryption, screen/physical security, USB/removable-media, printers/scanners, and local-admin policy.
- Approve DPAPI/CNG/credential-store scope and ACL design for the actual account model, plus recovery-secret custodians and rotation/revocation.
- Set backup destination, schedule, off-device custody, RPO/RTO, retention, restore drill, and lost-device/key/media procedures.
- Set authentication timeout/lockout/password/recovery bounds and audit-review responsibilities.
- Approve software/content update channels, signing keys, model/runtime source and checksum policy, firewall/loopback controls, and any future allowlisted destination.

## 8. Remaining recommendations requiring product-owner approval

Decisions 1, 2, and 6 are approved and no longer appear in this list.

1. SQLCipher-compatible whole-database encryption as the target, subject to the Phase 1 packaging/licensing spike (Decision 3).
2. Per-database random keys with Windows-protected local wrapping and a separate portable backup recovery mechanism (Decision 4).
3. Manual plus scheduled encrypted local backup, including explicitly selected removable media (Decision 5).
4. Versioned local VIS packages and default fail-closed behavior for missing/corrupt/known-stale/unknown-currency applicable content, subject to clinical/legal policy (Decision 12).
5. Minimum-provenance AI retention and no raw audio retention in the MVP (Decisions 14–15).
6. Separate-surname patient identity and human-only duplicate disposition (Decision 19).
7. English and Spanish UI architecture/chrome from the first vertical slice (Decision 23).

## 9. PHASE 1 IMPLEMENTATION CONTRACT

**Authorization:** Approved by the product owner on 2026-08-30 for synthetic data only. Phase 1 does not authorize real PHI, production deployment, or Phase 2.

Its implementation task must obey these testable constraints:

1. Use only synthetic patients and a visibly/build-time marked `SYNTHETIC_ONLY` data mode; no production secrets, endpoints, or PHI.
2. Target Windows 11 x64 for production packaging; keep domain/application crates platform-neutral and put Windows behavior behind adapters.
3. Build the Tauri 2 → narrow typed command → Rust application/domain → port → adapter boundary; React gets no SQL, secrets, generic filesystem/shell/HTTP, or authorization authority.
4. Implement named permission checks and denial tests in Rust; users may hold multiple roles and administrator has no clinical access by default.
5. Define workflow/state/revision types, but do not implement clinical eligibility, recommendations, contraindication/precaution logic, PREIS transport, or production features outside the approved Phase 1 list.
6. Implement persistence and migration harnesses against SQLite repositories with foreign keys, expected revisions, explicit transactions, and atomic audit append; no generic update/delete path for post-attestation/finalized/audit data.
7. Run a documented SQLCipher/Rust/Windows packaging and licensing spike. Real-PHI mode cannot exist until encrypted database, journals/temp behavior, migration, integrity, backup, and restore pass on real Windows builds.
8. Define `SecretStore` and `BackupService` ports; prototype per-database random keys, Windows protection scope, independently encrypted versioned backup containers, staged restore, and total key-loss behavior with synthetic data.
9. Implement safe structured operational logging and encrypted audit concepts separately; add automated synthetic markers proving no patient-like values enter logs, crash output, temp paths, or process arguments.
10. Define fake-first `LocalAiProvider`, `SpeechToTextProvider`, `RegistryAdapter`, `Clock`, filesystem/export, and barcode ports. No adapter receives generic network/file/tool authority.
11. Permit no external egress in Phase 1. Patient-bearing provider tests accept only IP loopback/controlled child process and reject remote endpoints, redirects, and proxies; the deterministic manual path works with all providers absent and network disabled.
12. Establish localization message catalogs and locale-aware formatting from the first UI component; clinical content remains versioned and separately reviewed.
13. Add CI gates for Rust/TypeScript format, lint, type-check, tests, dependency/license review, secret scan, synthetic-data policy, Tauri capability/CSP review, and production rejection of development-auth/plaintext shortcuts.
14. Verify crash/restart, stale-revision conflict, denied forged command, key-store unavailable, backup/restore, disk-full/error classification, provider unavailable, and offline behavior before Phase 1 exit.
15. Stop at the Phase 1 exit review. Do not start the clinical vertical slice, use real PHI, or enable registry transmission without a new approved plan.

## 10. Evidence boundary

This review uses official sources as engineering inputs, not as legal advice or proof of deployment compliance. Sources were checked on 2026-08-30 and must be reverified when their capability is implemented:

- [HHS Security Rule summary](https://www.hhs.gov/hipaa/for-professionals/security/laws-regulations/index.html)
- [HHS guidance on encryption/key separation](https://www.hhs.gov/hipaa/for-professionals/breach-notification/guidance/index.html)
- [CDC VIS instructions](https://www.cdc.gov/vaccines/hcp/about-vis/instructions.html)
- [CDC IIS Core Data Elements](https://www.cdc.gov/iis/core-data-elements/)
- [PRDoH PREIS local HL7 guide](https://www.salud.pr.gov/CMS/DOWNLOAD/8575)
- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) and [installer guidance](https://v2.tauri.app/distribute/windows-installer/)
- [SQLCipher design](https://www.zetetic.net/sqlcipher/design/), [API](https://www.zetetic.net/sqlcipher/sqlcipher-api/), [key guidance](https://www.zetetic.net/sqlcipher/database-key-material/), and [licensing](https://www.zetetic.net/sqlcipher/license/)
- [Microsoft DPAPI example/limitations](https://learn.microsoft.com/en-us/windows/win32/seccrypto/example-c-program-using-cryptprotectdata) and [data protection guidance](https://learn.microsoft.com/windows/uwp/security/data-protection)
- [RFC 9106: Argon2](https://www.rfc-editor.org/rfc/rfc9106.html)
- [Ollama API](https://docs.ollama.com/api/introduction), [llama.cpp server](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md), and [whisper.cpp server](https://github.com/ggml-org/whisper.cpp/blob/master/examples/server/README.md)

## 11. Related documents

- [Product requirements](PRODUCT_REQUIREMENTS.md)
- [Architecture](ARCHITECTURE.md)
- [Data model](DATA_MODEL.md)
- [Security](SECURITY.md)
- [Roadmap](ROADMAP.md)
