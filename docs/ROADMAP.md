# AutoVaxx Roadmap

**Status:** Phase 1 authorized and under exit review; Phase 2 is not authorized

**Planning rule:** A phase starts only after its entry decision is approved; no phase may introduce real PHI before the real-PHI gates pass.

## 1. Delivery strategy

Build one thin, auditable workflow end to end before widening vaccine coverage or adding integrations. Every phase uses synthetic patients until legal, clinical, privacy, security, operational, and encryption gates authorize a controlled pilot.

The MVP is a local desktop modular monolith. It does not include cloud services, automatic background submission, or multi-workstation synchronization.

## 2. Phase overview

| Phase | Outcome | Patient data | Exit decision |
|---|---|---|---|
| 0. Product and risk discovery | Approved scope and verified external obligations | None | Approve implementation scope |
| 1. Technical foundations | Installable secure shell and tested persistence boundaries | Synthetic only | Approve vertical workflow build |
| 2. Documentation vertical slice | One clinically narrow workflow from patient to registry-ready artifact | Synthetic only | Approve clinical validation |
| 3. Clinical and security validation | Approved rules, threat controls, recovery, and usability evidence | Synthetic only | Approve controlled pilot |
| 4. Controlled single-facility pilot | Real-world workflow evidence under approved operations | Real PHI only after gate | Approve wider local deployment |
| 5. PREIS transmission | Verified test-to-production registry exchange | Authorized PHI | Approve production transmission |
| 6. Expansion | Additional vaccines, automation aids, or multi-workstation design | According to approved scope | Separate decision per capability |

## 3. Phase 0: product and risk discovery

### Objectives

- Approve the product requirements, architecture, data model, security baseline, roadmap, and project agent rules.
- Observe current pharmacy documentation workflows and measure baseline active time, interruptions, re-entry, and common missing fields.
- Approve a documentation-only first scope: capture screening and validate completeness without eligibility, recommendation, forecasting, or contraindication/precaution interpretation.
- Produce the first authorization matrix for vaccinating professional, support staff, administrator, and auditor.
- Confirm consent, record retention, correction/void, print/export, and professional credential requirements with legal/operations owners.
- Contact PRDoH/PREIS to verify the current guide/profile, onboarding, endpoints, credentials, test environment, acknowledgements, submission timing, and certification process.
- Approve Windows 11 x64 as the sole MVP production OS and select representative hardware/peripherals; Windows and macOS remain valid development hosts.
- Complete a preliminary threat model and data-flow review.

### Deliverables

- Approved versions of these six planning documents.
- Workflow observation and baseline measurement report with no patient identifiers.
- Documentation-only scope statement, prominent unsupported-clinical-scope language, and named clinical reviewer for the pilot workflow.
- Regulatory/PREIS discovery log separating verified facts, assumptions, and open questions.
- Prioritized release risks and acceptance-gate owners.

### Exit criteria

- No unresolved decision can materially change the first vertical slice.
- Clinical and legal owners accept the deliberately narrow MVP scope.
- PREIS uncertainty is explicitly scoped; live transmission remains deferred if not verified.
- Product owner authorizes implementation to begin on a non-`main` branch.

## 4. Phase 1: technical foundations

**Current exit status:** Not passed. Windows secret protection and the integrated SQLCipher/key/backup/restore path are implemented and verified with synthetic data on macOS, but representative Windows 11 x64 execution, clean-workstation recovery, failure-mode testing, native distribution decisions, and production approvals remain open. See [Phase 1 Exit Review](PHASE_1_EXIT_REVIEW.md).

### Build

- Tauri 2 desktop shell with React, TypeScript, Vite, and Rust workspace boundaries.
- Typed, narrow Tauri command pattern with Rust validation/authorization hooks.
- Local user/session skeleton and role permission model.
- SQLite repository and migration harness using synthetic data.
- Encryption/packaging spike comparing SQLCipher-compatible Rust options on supported platforms.
- OS credential-store adapter and synthetic key-loss/recovery exercises.
- Windows 11 x64 installer/test lane; application-local named-account authentication with production-disabled development shortcuts.
- Transactional audit-event mechanism and integrity verification.
- No-PHI structured logging, local crash behavior, and redaction tests.
- Provider interfaces with fake AI, speech, registry, clock, and secret-store adapters.
- CI gates for formatting, linting, type-checking, unit/integration tests, dependency audit, secret scan, and synthetic-data scan.

### Do not build

- Clinical rules, broad forms, live PREIS, cloud services, inventory, billing, or synchronization.

### Exit criteria

- Installable application runs on each supported OS/hardware profile.
- Encrypted database and encrypted backup/restore pass real-instance tests.
- Forged frontend calls cannot bypass Rust authorization.
- Network-off tests pass and no process sends synthetic patient content externally.
- Crash/restart and migration tests preserve data and audit consistency.

## 5. Phase 2: documentation vertical slice

### Build in workflow order

1. Patient search/create with duplicate warnings and revisions.
2. Encounter state machine and resumable draft.
3. One versioned screening template that records explicit answers without interpreting clinical eligibility.
4. Deterministic screening-completeness, documentation, temporal/product, and registry-readiness fixtures; no clinical eligibility rules.
5. Consent evidence and refusal/withdrawal.
6. Offline VIS content package, delivery evidence, and freshness status.
7. Vaccine product/lot selection and deterministic validation.
8. Re-authenticated administration confirmation.
9. Finalization, correction, void, and complete audit timeline.
10. Registry-readiness mapping and inspectable artifact for one verified PREIS profile version, without production transmission.
11. Ollama structured-proposal adapter and whisper.cpp transcription adapter, both optional, local-only, and non-retentive for raw prompt/response/audio by default.

Planning detail: [Assist Graph and Bounded Loops Plan](ASSIST_GRAPH_PLAN.md) defines the proposed Rust-owned assistance graph and synthetic evaluation loop. It is approved for planning only and does not authorize Phase 2 implementation.

### UX priorities

- Keyboard-first and bilingual English/Spanish workflow.
- One visible checklist with direct navigation to every incomplete/blocking item.
- Clear separation of AI proposals, accepted facts, clinical warnings, and administration confirmation.
- Automatic local draft saves with visible saved/conflict state.
- No badges, decorative status pills, or ambiguous icon-only clinical actions.

### Exit criteria

- Representative synthetic encounters complete end to end offline.
- AI unavailable/malformed-output cases fall back to manual documentation safely.
- No role or AI path can confirm administration except the authorized professional command.
- Corrections preserve prior values and regenerate a new registry artifact.
- Product metrics can be measured from non-PHI timings and user research, not hidden surveillance.

## 6. Phase 3: clinical and security validation

### Clinical validation

- Clinical owner reviews every screening question, documentation rule, explanation, code set, attestation, unsupported-scope statement, and synthetic fixture.
- Test complete/incomplete screening, unknown answers, minors/representatives, historical doses, combination products, multiple VISs, expired/mismatched lots, late entry, correction, and void without interpreting eligibility.
- Document that eligibility, recommendation, forecasting, contraindication, and precaution logic is unsupported.
- Establish documentation-rule/content update, review, signing, rollback, and retirement procedures. Any later clinical rule package requires its own approval phase.

### Security and reliability validation

- Complete deployment-specific risk analysis and update the threat model.
- Penetration test Tauri IPC/capabilities, local auth, session behavior, import parsing, network controls, and provider isolation.
- Inspect operational logs, crash artifacts, temp directories, process lists, clipboard behavior, backups, and support bundles for synthetic PHI leakage.
- Fuzz enabled parsers and IPC boundaries.
- Run power-loss, disk-full, corrupted-database, unavailable-keychain, interrupted-migration, and restore drills.
- Verify signed build/update and clinical-content supply chains.

### Usability validation

- Compare documentation time and error/rework rates against the Phase 0 baseline.
- Test with representative pharmacists and support staff using synthetic cases in Spanish and English.
- Resolve safety-critical usability findings before aesthetic polish.

### Exit criteria for real-PHI consideration

- All gates in [PRODUCT_REQUIREMENTS.md](PRODUCT_REQUIREMENTS.md) and [SECURITY.md](SECURITY.md) have named evidence and owners.
- Clinical, legal/privacy, security, operations, and product owners sign the pilot decision.
- Backup/restore, incident response, downtime, support, retention, and device-loss procedures are practiced.
- No unresolved high-severity security or clinical-safety finding remains.

## 7. Phase 4: controlled single-facility pilot

### Scope controls

- One approved facility and a small named user group.
- Only approved vaccines/populations and supported operating system/hardware.
- Feature flags keep unsupported clinical scope, live PREIS transmission, and external integrations disabled.
- Defined pilot duration, support coverage, data reconciliation, incident escalation, rollback, and stop criteria.

### Measure

- Median active documentation time and comparison with baseline.
- Completion/rework/correction rate and reasons.
- Rule warning/override frequency and clinical review outcomes.
- Draft recovery, lockout, backup, and support incidents.
- Registry-ready validation failures, without treating readiness as acceptance.
- User-reported cognitive load and workflow interruptions.

Metrics must be aggregated locally or exported only through an explicitly approved, minimum-necessary process. Do not add third-party analytics.

### Exit criteria

- The pilot meets agreed safety, completeness, time-reduction, reliability, and support thresholds.
- Every clinical record can be reconciled with the facility's authoritative process.
- Pilot incidents and corrections are resolved and reflected in tests/docs.
- Owners approve broader local use or return to validation.

## 8. Phase 5: PREIS transmission

This phase starts only after PRDoH confirms the current requirements.

### Build and verify

- Freeze a verified PREIS profile with source document hash, verification date, and named verifier.
- Implement adapter mapping, HTTPS transport, credential storage, and ACK parsing behind existing ports.
- Keep test and production environments visibly and technically separate.
- Validate synthetic VXU/ACK cases in the authorized PREIS test environment.
- Prove partial-batch failure handling, retry/idempotency, reconciliation, downtime queueing, correction/void behavior, and credential rotation.
- Present destination/scope and require explicit authorization for MVP transmissions.

### Exit criteria

- PRDoH-required conformance/onboarding is complete.
- Facility approves the minimum-necessary mapping and operating procedure.
- Synthetic end-to-end and controlled production-readiness evidence pass.
- Application distinguishes `sent`, `accepted`, `warning`, `rejected`, and `reconciliation required` without ambiguity.

## 9. Phase 6: expansion options

Each is a separate product decision, not an automatic continuation:

- Additional vaccines, ages, conditions, screening templates, and rule packages.
- Separately approved clinical eligibility, recommendation, forecasting, contraindication, and precaution rule packages.
- Barcode scanning and lightweight lot availability support.
- Facility-approved print or EHR export adapters.
- Improved bilingual content and accessibility.
- Local model/runtime packaging or llama.cpp adapter.
- Multi-workstation discovery and architecture.

### Multi-workstation discovery gate

Before building synchronization, establish the actual number of devices/users, offline duration, network ownership, server operations, identity provider, conflict cases, recovery objectives, and budget. Then design patient matching, device enrollment, mutual authentication, authorization, conflict resolution, and central backup as a new architecture decision. Do not turn SQLite file sharing into synchronization.

## 10. Cross-cutting workstreams

| Workstream | Continuous requirement |
|---|---|
| Clinical governance | Named owner, source citations, versioned fixtures, review/retirement dates |
| Security/privacy | Threat/risk review, least privilege, encryption, no-PHI logs, incident evidence |
| Data integrity | Revisions, audit atomicity, migrations, backup/restore, reconciliation |
| Accessibility/language | Keyboard/screen-reader checks and reviewed Spanish/English content |
| Quality | Synthetic fixtures, real encrypted database tests, end-to-end offline tests |
| Integration | Interface first; verified profile; test environment before production |
| Operations | Install/update, content distribution, support, downtime, retention, device lifecycle |

## 11. Definition of done for any feature

- Requirements and non-goals are linked.
- Clinical/legal claims use authoritative current sources or are labeled assumptions/open questions.
- Authorization and audit behavior are specified and tested.
- Historical records cannot be silently overwritten.
- Offline, error, restart, and unavailable-dependency behavior are tested.
- Operational logs and test fixtures contain synthetic/non-PHI data only.
- Threat model and data-flow changes are reviewed.
- Documentation and migration/rollback or recovery instructions are updated.
- The real application path is exercised; mocks alone are not completion evidence.
- Product, clinical, security, and integration acceptance criteria pass as applicable.

## 12. Risk register

| Risk | Early control | Trigger for escalation |
|---|---|---|
| Clinical scope expands faster than validation | Narrow approved package and unsupported-state messaging | Request to add a vaccine/condition without owner and fixtures |
| PREIS assumptions drift | Verified profile registry and disabled transmission | Current guide/endpoint/onboarding cannot be confirmed |
| Encryption blocks packaging/support | Phase 1 cross-platform spike and real restore tests | Selected library fails supported OS, licensing, or recovery needs |
| AI creates automation bias | Proposal-only UI, provenance, human acceptance, deterministic rules | Users accept fields without review or error rate exceeds threshold |
| Single workstation fails | Encrypted backups and practiced restore | Restore objective cannot be met or key recovery is unclear |
| PHI leaks through diagnostics | Structured safe logs, redaction tests, local-only crash handling | Any synthetic identifier appears in prohibited output |
| MVP becomes a platform project | Explicit deferred list and phase gates | Broker, sync, plugin system, or generic rules DSL proposed without measured need |

## 13. Related documents

- [Product requirements](PRODUCT_REQUIREMENTS.md)
- [Architecture](ARCHITECTURE.md)
- [Data model](DATA_MODEL.md)
- [Security](SECURITY.md)
- [Foundation decisions](FOUNDATION_DECISIONS.md)
