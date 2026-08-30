# AutoVaxx Product Requirements

**Status:** Phase 1 scope approved; broader product requirements remain subject to clinical, legal, security, and PREIS validation

**Audience:** Product, pharmacy operations, clinical reviewers, engineering, privacy/security reviewers

**Initial deployment:** One computer, one facility, multiple named local users

**Product type:** Local-first desktop immunization documentation application for Puerto Rico

Authoritative sources linked below were checked on 2026-08-30. Their future currency must be reverified before clinical-content or integration releases.

## 1. Product intent

AutoVaxx reduces the time and rework required to document a vaccination while preserving clinical accountability. It guides a healthcare worker from patient identification through screening, consent, Vaccine Information Statement (VIS) delivery, vaccine selection, administration confirmation, final documentation, registry-readiness validation, and audit review.

AutoVaxx is a documentation system, not an autonomous clinician. Local artificial intelligence may transcribe and propose structured values. The first implementation uses deterministic, versioned code for workflow, documentation, temporal, product/lot, and registry-readiness validation only. It captures screening responses but does not determine clinical eligibility, recommend vaccines, forecast doses, or interpret contraindications/precautions. Only an authenticated, authorized healthcare professional may attest that a vaccine was administered.

## 2. Evidence and regulatory posture

These requirements are product and engineering requirements, not legal advice. Before a production pilot, the facility must complete legal, privacy, clinical, and PREIS onboarding reviews.

Verified design inputs:

- The current HHS Security Rule summary requires reasonable and appropriate administrative, physical, and technical safeguards for electronic protected health information (ePHI), including access control, audit controls, integrity, authentication, transmission security, contingency planning, and risk analysis. The rule is technology-neutral and risk-based. See [HHS: Summary of the HIPAA Security Rule](https://www.hhs.gov/hipaa/for-professionals/security/laws-regulations/index.html).
- CDC states that an applicable current VIS must be provided before each dose and identifies required record elements such as VIS edition date, date provided, administration date, manufacturer, lot number, and administering professional/facility information. See [CDC: Instructions for Using VISs](https://www.cdc.gov/vaccines/hcp/about-vis/instructions.html).
- Puerto Rico Department of Health published a PREIS local HL7 v2.5.1 implementation guide, revision 2.2 dated April 2022, covering VXU and ACK messages and describing HTTPS-based exchange. See [PRDoH: PREIS Local HL7 v2.5.1 Implementation Guide](https://www.salud.pr.gov/CMS/DOWNLOAD/8575).
- The 2022 PREIS guide is a useful verified source, but it is not proof of the current production endpoint, onboarding process, account configuration, testing process, or complete current field requirements. Those remain discovery items.

## 3. Product principles

1. AI extracts and structures information; it does not make clinical decisions.
2. Deterministic, versioned rules perform clinical and documentation validation.
3. AI cannot create, finalize, sign, or transmit an administration event.
4. Only an authenticated professional with the required local role may confirm administration.
5. Every meaningful change creates an audit event in the same database transaction.
6. Finalized clinical history is append-only. Corrections create linked revisions and preserve prior values.
7. PHI never appears in operational application logs, diagnostics, analytics, crash reports, or telemetry. The required patient-linked audit ledger is a separate encrypted clinical/security record, not a diagnostic logging sink, and contains the minimum identifiers needed for accountability.
8. Tests and demonstrations use synthetic patients only.
9. Patient data remains on the computer unless a user explicitly authorizes a defined external transmission.
10. Patient data is never sent to cloud AI services.
11. External systems, AI runtimes, speech runtimes, and export formats are behind interfaces.
12. The core workflow works without internet access.

## 4. Users and authorization

| Role | Primary needs | Allowed actions in the MVP |
|---|---|---|
| Vaccinating professional | Finish accurate documentation quickly | Create/edit drafts, review screening and rule results, confirm administration, finalize/correct records, create and authorize registry-ready exports |
| Clinical support staff | Prepare the encounter without making the clinical attestation | Find/create patients, enter demographics, capture screening answers, prepare consent/VIS evidence and vaccine details; cannot confirm administration or finalize a clinical record |
| Facility administrator | Configure local users and facility metadata | Manage users/roles, local content packages, backup settings, and facility configuration; cannot modify clinical facts unless separately authorized as a vaccinating professional |
| Auditor/privacy reviewer | Review access and changes | Read authorized records and audit history; no clinical mutation or administration confirmation |

One person may have more than one role. Authorization is checked in Rust at every privileged command; hiding a frontend control is not authorization.

## 5. Primary workflow

```text
Patient
  -> Screening
  -> Consent
  -> VIS
  -> Vaccine selection
  -> Vaccine administration
  -> Documentation
  -> Registry-ready record
  -> Audit log
```

### 5.1 Workflow states

| State | Meaning | Exit condition |
|---|---|---|
| `DRAFT` | Work may be incomplete and must not represent an administered dose | Required pre-administration steps are complete |
| `READY_TO_ADMINISTER` | Deterministic checks have run; warnings and overrides are visible | Authorized professional confirms the physical administration or cancels |
| `ADMINISTERED_PENDING_DOCUMENTATION` | A professional has attested to administration; documentation may still be incomplete | All required documentation fields validate |
| `FINALIZED` | Clinical record is complete and locked against in-place overwrite | Registry-readiness validation or a correction begins |
| `REGISTRY_READY` | A versioned payload can be generated from the finalized record | Explicit export/transmission or correction |
| `CORRECTED` | A later revision supersedes the finalized version | New current revision is linked and auditable |
| `VOIDED` | The record is retained but marked invalid with a reason | No destructive deletion; replacement may be linked |

An AI proposal has no workflow authority. It cannot change the state.

## 6. Functional requirements

### 6.1 Patient and encounter

- **FR-PAT-001:** Search patients locally using normalized identifiers and demographics without exposing patient details in logs.
- **FR-PAT-002:** Warn on probable duplicates using deterministic matching; never automatically merge patients.
- **FR-PAT-003:** Preserve separate first surname and second surname fields and render names according to Puerto Rico workflows.
- **FR-PAT-004:** Store patient demographic revisions rather than overwriting finalized historical values.
- **FR-ENC-001:** Create an encounter tied to one facility, patient, responsible professional, and local timezone.
- **FR-ENC-002:** Save work locally and recover safely after an application or power interruption.
- **FR-ENC-003:** Show a clear checklist of complete, incomplete, warning, and blocking items.

### 6.2 Screening and deterministic validation

- **FR-SCR-001:** Capture structured screening answers, notes, author, source, and timestamps.
- **FR-SCR-002:** Version screening templates and preserve the exact questions and answer options used for the encounter.
- **FR-SCR-003:** In the first implementation, run deterministic documentation rules that verify the required screening template was completed and that no required answer is silently missing; do not interpret answers as eligibility or clinical clearance.
- **FR-SCR-004:** Present each documentation-rule result with severity, explanation, evidence inputs, rule identifier, and rule/content version.
- **FR-SCR-005:** Require an authorized professional to resolve blocking documentation findings. Documentation blocks such as missing required evidence or expired product are not convenience-overridable. Any future clinically approved override policy requires reason, identity, timestamp, and audit event.
- **FR-SCR-006:** Never infer an unanswered screening item as negative.

The first implementation is documentation-only. It must not recommend a vaccine, determine whether a dose is due, apply dose intervals, interpret pregnancy/immunocompromise/allergy responses, forecast, diagnose, or state that vaccination is clinically safe. A Puerto Rico-licensed clinical reviewer must approve the screening/documentation workflow, expired-product behavior, attestation language, and unsupported-scope messaging before a clinical pilot. Eligibility or contraindication/precaution logic may be added only later as a separately approved, versioned clinical rule package.

### 6.3 Consent

- **FR-CON-001:** Record who gave consent, their relationship to the patient, consent method, timestamp, language, and policy/form version.
- **FR-CON-002:** Support refusal or withdrawal without creating an administration event.
- **FR-CON-003:** Preserve consent evidence and corrections. The MVP does not assume that a typed attestation or captured signature alone satisfies every facility or Puerto Rico legal requirement.
- **FR-CON-004:** Prevent administration confirmation when required consent evidence is missing.

### 6.4 VIS

- **FR-VIS-001:** Associate every applicable vaccine with a versioned VIS record, including document type, edition date, language, source, local content hash, delivery timestamp, and delivery method.
- **FR-VIS-002:** Record that the VIS was provided before administration; a timestamp after administration is a deterministic validation failure requiring correction.
- **FR-VIS-003:** Work offline using an installed, integrity-checked content package.
- **FR-VIS-004:** Display content-package freshness and warn or block according to approved policy when the application cannot establish that the installed VIS is current.
- **FR-VIS-005:** Never let AI generate or substantively modify official VIS content.

### 6.5 Vaccine selection and administration

- **FR-VAX-001:** Select products from a versioned code set and local inventory/lots when available.
- **FR-VAX-002:** Capture at minimum the administered vaccine/code, administration date/time, dose amount/unit, route, site, manufacturer, lot number, expiration date, ordering professional when applicable, administering professional, and facility for an administered-here event.
- **FR-VAX-003:** Support barcode-assisted entry as a later enhancement; scanned values remain proposals until validated and confirmed.
- **FR-VAX-004:** Show product/lot expiration and structured mismatch checks before confirmation.
- **FR-VAX-005:** Require an explicit, deliberate administration confirmation by an authorized professional. Confirmation records identity, timestamp, workstation, and the version of the reviewed data.
- **FR-VAX-006:** Do not offer bulk or automatic administration confirmation.
- **FR-VAX-007:** Cancellation before administration and voiding after erroneous documentation are different operations with different audit semantics.

### 6.6 Documentation, corrections, and audit

- **FR-DOC-001:** Validate required clinical, legal, facility, and registry-readiness fields deterministically.
- **FR-DOC-002:** Render a human-readable vaccination record and a machine-readable registry candidate from the same finalized revision.
- **FR-DOC-003:** Lock finalized clinical revisions against in-place editing.
- **FR-DOC-004:** Correct a finalized record only by creating a new linked revision with reason, author, and timestamp.
- **FR-DOC-005:** Preserve prior revisions, void reasons, acknowledgements, and export/transmission history.
- **FR-AUD-001:** Create audit events for authentication, patient access, draft creation, meaningful field changes, rule evaluation, overrides, administration confirmation, finalization, correction, voiding, export, transmission authorization, transmission outcome, user/role changes, configuration changes, backup/restore, and audit review.
- **FR-AUD-002:** Keep PHI values out of audit event summaries where an entity/version reference and changed-field list are sufficient.
- **FR-AUD-003:** Make audit history append-only through the application and verify its integrity.

### 6.7 Registry readiness and external transmission

- **FR-REG-001:** Keep the canonical clinical model independent of PREIS/HL7 fields.
- **FR-REG-002:** Map a finalized revision through a versioned registry adapter and report field-level validation errors.
- **FR-REG-003:** Generate a deterministic PREIS candidate payload only against a verified implementation-guide profile.
- **FR-REG-004:** Label records `REGISTRY_READY` only when local validation passes; this does not mean PREIS accepted the record.
- **FR-REG-005:** Require an explicit, authorized user action for every external export/transmission batch containing PHI.
- **FR-REG-006:** Show destination, patient/record count, data categories, purpose, and payload profile before authorization.
- **FR-REG-007:** Record acknowledgements and errors without copying unrestricted PHI into operational logs.
- **FR-REG-008:** Queue no automatic background PHI transmission in the MVP.

The MVP target is a registry-ready record and inspectable export artifact. Live PREIS transmission is deferred until PRDoH confirms the current contract, endpoint, enrollment, credentials, test environment, acknowledgement behavior, retry rules, and conformance criteria.

### 6.8 Local AI and speech

- **FR-AI-001:** Accept typed notes or locally generated transcripts and propose structured values with source spans and confidence/uncertainty indicators.
- **FR-AI-002:** Require human review before accepting any proposed value into the clinical draft.
- **FR-AI-003:** Use a Rust provider interface. The initial adapter targets a loopback-only Ollama instance; a later adapter may target llama.cpp.
- **FR-AI-004:** Reject non-loopback AI endpoints when a request contains patient data.
- **FR-AI-005:** Do not give AI tools that write clinical records, change workflow state, authorize export, or access arbitrary files/network resources.
- **FR-AI-006:** Do not persist prompts, model context, or raw model responses unless an approved feature explicitly requires it; if retained, treat them as PHI and store them encrypted with a retention policy.
- **FR-AI-007:** Enforce an absolute provider deadline. Abort and quarantine externally provisioned providers; hard-terminate an app-owned child process tree after cooperative cancellation fails. Neither path may mutate clinical state or block manual documentation.
- **FR-AI-008:** Verify the runtime model digest against the approved synthetic-evaluation manifest before patient-bearing inference and record the model digest plus prompt-template hash in minimum accepted-proposal provenance.
- **FR-AI-009:** Route provider out-of-memory and isolated provider-runtime disk-full failures directly to manual fallback without retry/repair. Treat unknown/shared-volume or clinical persistence exhaustion as a blocking integrity failure.
- **FR-SP-001:** Use a speech-provider interface with whisper.cpp as the initial provider.
- **FR-SP-002:** Process audio locally through bounded RAM or anonymous pipes when possible. If a temporary file is unavoidable, store only per-session encrypted ciphertext with restrictive ACLs and cryptographically erase the key before deletion and validated orphan cleanup. The MVP does not retain raw recordings; any future retention is a separately approved feature and policy.

### 6.9 Offline behavior

- **FR-OFF-001:** Patient lookup, documentation, deterministic rules, finalization, correction, audit, local AI, and local speech work without internet when dependencies and content packages are installed.
- **FR-OFF-002:** Network unavailability must not corrupt or lose work.
- **FR-OFF-003:** Online-only actions such as content updates or authorized registry transmission must state that they are pending, not silently succeed or fail open.
- **FR-OFF-004:** The application must never weaken authentication, validation, or audit requirements because it is offline.

## 7. Non-functional requirements

### Privacy and security

- Encrypt PHI at rest before any real patient use; plaintext databases are permitted only in clearly marked synthetic development/test environments.
- Store encryption keys separately from the database and encrypted backups.
- Use named local accounts, least privilege, password hardening, session timeout/lock, and re-authentication for high-impact actions.
- Deny network access by default. Network-capable adapters use an explicit allowlist and authorization policy.
- Produce no cloud analytics or crash upload containing PHI.
- Meet the controls and release gates in [SECURITY.md](SECURITY.md).

### Reliability and data integrity

- Clinical mutation plus its audit event must commit atomically.
- Use SQLite foreign keys, transactions, integrity checks, schema migrations, and tested backup/restore.
- A crash between administration confirmation and final documentation must reopen the exact pending state without losing the attestation.
- Retries of exports/transmissions must be idempotent and traceable.

### Performance and usability targets

Targets are hypotheses until measured with representative, synthetic workflows:

- Reduce median active documentation time by at least 50% compared with the facility's measured baseline.
- Complete routine local saves within 250 ms at the 95th percentile on supported hardware, excluding model inference.
- Open patient search results within one second at the 95th percentile for the expected single-facility dataset.
- Establish and pass median and 95th-percentile time-to-fallback targets before Phase 2 exit so failed assistance releases the manual form without unacceptable delay.
- Measure accepted-as-is rate, accepted-with-correction rate, and correction actions per accepted proposal in synthetic/usability evaluation; assistance must not add more correction work than the approved manual baseline.
- Make every blocking item actionable from the workflow summary.
- Support keyboard-first use and WCAG 2.2 AA-level accessibility as a release target.
- Support English and Spanish user-facing workflows; clinical translations require review rather than machine-only translation.

### Maintainability

- Keep the React frontend free of SQL, secrets, and authorization decisions.
- Keep clinical rules, content packages, provider adapters, and registry mappings versioned and independently testable.
- Avoid microservices, distributed queues, and premature multi-tenant infrastructure in the MVP.

## 8. MVP scope

### Included

- Named local users and role-based authorization.
- Local patient and encounter workflow.
- Versioned screening, consent, and VIS evidence.
- A documentation-only, clinically reviewed workflow scope with deterministic completeness, temporal, role, product/lot, and registry-readiness validation; no eligibility or recommendation logic.
- Vaccine administration confirmation and complete documentation.
- Append-only corrections and audit history.
- Registry-readiness validation and a local, inspectable export candidate.
- Ollama and whisper.cpp adapters with graceful unavailable states.
- Encrypted local storage, encrypted backup/restore, and operational log redaction.

### Explicitly deferred

- Live PREIS transmission until current integration requirements are verified and tested.
- Multi-workstation synchronization and conflict resolution.
- Cloud hosting, cloud backup, cloud AI, patient portal, scheduling, billing/claims, e-prescribing, and full inventory management.
- Autonomous vaccine recommendation, diagnosis, administration, finalization, or submission.
- Clinical eligibility, recommendation, forecasting, contraindication, or precaution logic until a separately versioned rule package is clinically approved and validated.
- Semantic barcode/product lookup beyond a bounded proposal flow, retained raw audio, app-managed model downloads, device-specific signature capture, and additional production OS targets beyond the first approved target.
- Automatic patient merge or historical-record deletion.

## 9. Success measures

| Measure | MVP success condition |
|---|---|
| Documentation time | At least 50% median reduction against an observed facility baseline using matched workflows |
| Completeness | At least 95% of representative synthetic routine encounters reach registry-ready state without post-finalization data repair |
| Safety | Zero paths allow AI or an unauthorized role to confirm administration, finalize, or transmit PHI |
| Data integrity | Crash/restart, correction, audit, backup, and restore scenarios pass without silent loss or overwrite |
| Offline operation | The complete core workflow succeeds with network disabled |
| Deterministic validation | 100% of approved documentation and registry-readiness fixtures produce expected results; every result includes rule/content version |
| Privacy | Automated log scans and manual review find no synthetic PHI in operational logs or crash artifacts |

## 10. Acceptance gates before real PHI

1. Puerto Rico legal and pharmacy-policy review is documented.
2. Clinical owner approves the documentation-only workflow scope, screening template, attestation, unsupported-scope language, and all validation fixtures.
3. Threat model and HIPAA risk analysis are reviewed by the deploying organization.
4. Database and backups are encrypted; key loss and recovery behaviors are tested.
5. Role authorization, re-authentication, session lock, and audit tests pass.
6. No-PHI logging and offline/network-denial tests pass.
7. Backup restore succeeds on a separate test installation using synthetic data.
8. External AI endpoints are technically blocked for patient workflows.
9. Registry-ready output is validated against the then-current, verified PREIS profile.
10. Pilot protocol, incident response, support ownership, and rollback criteria are approved.

## 11. Open decisions and discovery backlog

- Which products and workflows form the first documentation-only pilot scope, without implying eligibility support?
- Which local roles may document, administer, correct, void, export, and transmit under facility policy and Puerto Rico law?
- What consent evidence and retention policy are legally and operationally required?
- What PREIS guide/version, transport, endpoint, credentials, enrollment, test cases, and acknowledgement policy are current?
- What retention periods apply to clinical revisions, audit events, export artifacts, and backups?
- Is a SQLCipher community build acceptable, or does support/commercial packaging justify a licensed distribution?
- Which Windows 11 x64 hardware, account, printer, and scanner profile represents the production pilot?

## 12. Related documents

- [Architecture](ARCHITECTURE.md)
- [Data model](DATA_MODEL.md)
- [Security](SECURITY.md)
- [Roadmap](ROADMAP.md)
- [Foundation decisions](FOUNDATION_DECISIONS.md)
