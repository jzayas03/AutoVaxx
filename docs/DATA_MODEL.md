# AutoVaxx Data Model

**Status:** Approved logical model; Phase 1 subset implemented in migration 001

**Storage target:** Encrypted SQLite-compatible database

**Scope:** Single facility and workstation initially, with stable identities and revisions that can migrate later

## 1. Modeling goals

- Represent the complete vaccination documentation workflow without making registry payloads the canonical record.
- Preserve every finalized clinical value and every correction.
- Make administration confirmation attributable to one authenticated professional and one reviewed data revision.
- Support deterministic rules with reproducible inputs and versions.
- Generate an audit event atomically with every meaningful change.
- Avoid collecting identifiers or attachments merely because an external format can carry them.
- Permit future multi-workstation synchronization without implementing it now.

## 2. Modeling conventions

### Identifiers

- Application entities use randomly generated, opaque UUID/ULID-style identifiers. The implementation will select one format consistently.
- Integer row IDs may be internal database details but are never durable cross-system identifiers.
- External identifiers include an assigning authority and type; a naked identifier string is invalid.
- PREIS and other registry identifiers are external identifiers, not patient primary keys.

### Time

- Persist event timestamps as UTC instants plus the originating IANA timezone where local meaning matters.
- Persist date-only clinical facts, such as date of birth or a VIS edition date, as dates without invented times.
- Preserve the user-entered administration local time, UTC instant, and timezone.
- Use a trusted application clock abstraction so tests are deterministic.

### Codes and display values

- Coded values store code system, code, display snapshot, and code-set version/effective date.
- Display text is a historical snapshot; later terminology updates must not rewrite old records.
- Locally defined codes use a namespaced code system and documented mapping rules.

### Revisions and corrections

- A logical record has a stable root identifier and one or more immutable revisions.
- Draft rows may be updated during ordinary editing only with expected-revision checks and a same-transaction audit event. Drafts are not finalized historical records. Once administration is confirmed, relevant clinical revisions become immutable and all corrections append new revisions.
- A correction appends a revision with `supersedes_revision_id`, reason, author, and timestamp.
- `current_revision_id` is a convenience pointer, not deletion of history.
- Voiding changes status through a new revision; it does not delete the old record.

## 3. High-level relationship map

```text
Facility 1---* Workstation
Facility 1---* UserAccount *---* Role

Patient 1---* PatientRevision
Patient 1---* Encounter 1---* EncounterRevision
                         |---* ScreeningResponseSet 1---* ScreeningAnswer
                         |---* ConsentRecord
                         |---* VisDelivery
                         |---* ImmunizationEvent 1---* ImmunizationRevision
                         |                              |---* RuleEvaluation
                         |                              |---* RegistrySubmissionItem
                         |---* RuleEvaluation

VaccineProduct 1---* VaccineLot
ContentPackage 1---* ContentArtifact
RulePackage    1---* RuleDefinition

RegistryProfile 1---* RegistryArtifact 1---* RegistrySubmissionItem
RegistrySubmission 1---* RegistrySubmissionItem

Every meaningful action ---1 AuditEvent
```

## 4. Identity, facility, and access entities

### `facility`

| Field | Purpose |
|---|---|
| `facility_id` | Stable local facility identifier |
| `name`, `address` | Required display/documentation snapshot source |
| `external_identifiers` | Typed identifiers assigned by authorized systems |
| `timezone` | Default IANA timezone |
| `status` | Active/inactive; never hard-delete referenced facilities |
| `created_at`, `updated_at` | Administrative lifecycle metadata |

The MVP permits one active facility but does not use a singleton row or hard-coded facility values.

### `workstation`

Stores stable workstation identity, facility association, installation version, enrollment/activation status, and last integrity-check timestamps. It must not store hardware serial numbers unless justified by the risk analysis.

### `user_account`

Stores the local username, professional display name, status, password-verifier metadata, failed-attempt/lock state, role assignments, and credential/license references required by facility policy. Password plaintext and recoverable passwords are never stored.

### `role` and `user_role`

Initial roles are vaccinating professional, clinical support, facility administrator, and auditor. Permissions are named capabilities checked in Rust. Role changes are effective-dated and audited.

### `auth_session`

Stores a random session identifier hash, user, workstation, creation/expiry/lock timestamps, authentication strength, and last activity. High-impact operations may require a recent re-authentication timestamp. Session tokens never enter operational logs.

## 5. Patient model

### `patient`

The identity root contains `patient_id`, lifecycle status, creation metadata, and a `current_revision_id`. It contains no mutable demographic facts.

### `patient_revision`

| Field group | Representative data |
|---|---|
| Revision | `patient_revision_id`, `patient_id`, `revision_number`, `supersedes_revision_id`, author, reason, timestamps |
| Name | given, middle, first surname, second surname, suffix, preferred/display form |
| Demographics | date of birth, administrative sex as required by the active profile, race, ethnicity, birth place, multiple-birth indicator/order where required |
| Contact | structured physical/mailing address, municipality, postal code, phone/email when needed |
| Language | preferred language and interpreter need |
| Status | active/inactive/deceased/duplicate-candidate without destructive merge |

Fields are collected only when necessary for care, legal documentation, or a verified registry profile. A Social Security number is not part of the MVP unless a documented requirement and risk review justify it.

### `patient_external_identifier`

Stores patient, identifier type, assigning authority, encrypted value, validity dates, status, provenance, and source record. Uniqueness is scoped to assigning authority and identifier type.

### `patient_relationship`

Represents parent, guardian, legal representative, emergency contact, or other relationship. It records identity/contact data, relationship code, effective dates, and verification/provenance. It does not imply legal authority without the required evidence/policy.

### Duplicate handling

`patient_match_candidate` stores deterministic match evidence and reviewer disposition. Merging is not automatic. If a future merge workflow is approved, it must preserve both roots, record the surviving identity link, and never delete source history.

## 6. Encounter and workflow model

### `encounter`

Stable root tying patient, facility, workstation, encounter type, and current revision together.

### `encounter_revision`

Stores immutable workflow state, responsible professional, visit date/time, current screening/consent context, state transition reason, revision links, and author/time metadata.

Allowed workflow states are defined in [PRODUCT_REQUIREMENTS.md](PRODUCT_REQUIREMENTS.md). State transitions are domain rules, not free-form database values.

### `workflow_transition`

Stores encounter, from/to states, triggering user/action, timestamp, relevant entity revisions, and optional authorized reason. It supports timeline reconstruction without placing clinical narrative in operational logs.

## 7. Screening and rule model

### `screening_template`

Identifies a versioned template, clinical owner, effective/retired dates, target population/vaccines, language variants, and content hash. Published template versions are immutable.

### `screening_question`

Stores template version, stable question code, localized display text, answer type, allowed choices, requiredness, and provenance. The exact displayed question is retained as a content artifact or snapshot.

### `screening_response_set` and `screening_answer`

The response set identifies patient, encounter, template version, respondent/source, recorder, review status, and revision chain. Each answer stores question code/version, typed value, explicit `UNKNOWN`/`DECLINED` states, optional note reference, and provenance.

Missing is not equivalent to `false`. Free text is minimized and treated as PHI.

### `rule_package` and `rule_definition`

| Field | Purpose |
|---|---|
| `rule_package_id`, semantic version | Identifies an approved rule release |
| `content_hash`, signature metadata | Verifies installed package integrity |
| `clinical_owner`, approval timestamp | Establishes governance |
| `source_citations`, effective dates | Records basis and currency |
| `rule_id`, `rule_version` | Stable rule identity |
| `input_contract`, `result_contract` | Defines deterministic inputs and outputs |
| `severity`, `override_policy` | Defines workflow effect |

The initial representation is Rust code plus versioned documentation tables for screening completeness, documentation, temporal/product checks, and registry readiness. It does not interpret clinical eligibility. Separately approved clinical rule packages and a general-purpose rules language are deferred until a validated need exists.

### `rule_evaluation`

Stores subject entity/revision, rule and package versions, input fingerprint, outcome, severity, structured field references, explanation code/text snapshot, evaluation timestamp, and engine version.

### `rule_resolution`

Records acknowledge/override/correct-data actions. Overrides store authorized actor, reason code and narrative, timestamp, policy version, and the exact evaluation being resolved. Some blocks are non-overridable by design.

## 8. Consent and VIS model

### `consent_record`

| Field group | Representative data |
|---|---|
| Identity/revision | root and revision IDs, encounter, supersedes, status |
| Consenter | patient or relationship reference, name snapshot, relationship, authority evidence type |
| Scope | vaccine(s)/procedure(s) consented to or refused |
| Evidence | method, language, form/policy version, artifact hash/reference, witnessed-by if applicable |
| Timing | presented, signed/attested, withdrawn timestamps |
| Accountability | recorder, reviewer, correction reason |

Consent evidence is not assumed legally sufficient merely because it exists in the database; approved facility policy defines required evidence.

### `content_package`

Represents a locally installed, signed/versioned set of VIS documents, screening text, terminology, or other approved reference data. Stores source, publisher, version, release/retrieval dates, hash/signature, installation actor/time, and validation status.

### `content_artifact`

Stores artifact type, language, edition/effective date, source URL, content hash, media type, encrypted local path/blob reference, and package membership. Official content is immutable after installation.

### `vis_delivery`

Stores encounter, applicable vaccine selection, VIS artifact/type, official edition date, language, delivery method, delivered-to identity/relationship, `provided_at`, recorder, and revision/correction metadata. `provided_at` must precede the related administration time.

## 9. Vaccine product, lot, and administration model

### `vaccine_product`

Stores canonical product identity, CVX/MVX and other verified code mappings, display name snapshot, dose form/units, active dates, and terminology package version. Product entries are reference data, not clinical recommendations.

### `vaccine_lot`

Stores facility, product, manufacturer, lot number, expiration date, funding/source fields when required, status, provenance, and optional quantity metadata. Full inventory accounting is out of MVP scope.

### `immunization_event`

Stable root for one administered, historical, not-administered, corrected, or voided immunization record. It points to the current revision but retains all prior revisions.

### `immunization_revision`

| Field group | Required model capability |
|---|---|
| Revision | revision number, `supersedes_revision_id`, status, correction/void reason, author/time |
| Context | patient and patient revision, encounter, facility, source/type (administered here vs historical) |
| Product | product/code snapshots, manufacturer, lot, expiration |
| Administration | local/UTC time, dose amount/unit, route, site, body laterality when applicable |
| Professionals | ordering professional when applicable; administering professional and title/license reference |
| Evidence links | screening response, consent revision, all applicable VIS deliveries, rule evaluations/resolutions |
| Attestation | confirming professional, confirmation time, reviewed aggregate/version fingerprint, re-authentication context |
| Finalization | finalizing professional/time, documentation profile and validation result |

The attestation fields can be populated only by the `confirm_administration` use case after authorization and deterministic preconditions. No repository method exposes a generic way to set them.

### Historical doses

A historical dose has source/provenance and confidence fields but no AutoVaxx administration attestation. The UI and registry mapping must distinguish it clearly from a dose administered by the facility.

## 10. Registry and external disclosure model

### `registry_profile`

Stores registry name, jurisdiction, profile identifier/version, source guide URL/hash, verified date, verifier, effective dates, mapping package hash, transport capabilities, and status (`DRAFT`, `VERIFIED_FOR_RENDER`, `VERIFIED_FOR_TRANSMISSION`, `RETIRED`).

The April 2022 PREIS guide may seed a discovery profile, but production transmission requires a currently confirmed profile.

### `registry_validation`

Stores immunization revision, registry profile, validation engine version, result, field-level issues, and timestamp. The result is reproducible from immutable inputs.

### `registry_artifact`

Stores profile, source revision set, generated timestamp, content hash, encrypted payload reference/content, validation ID, and lifecycle state. Artifacts are immutable; corrections generate new artifacts.

### `transmission_authorization`

Stores the authenticated authorizer, re-authentication context, destination, purpose, patient/record counts, disclosed data-category summary, artifact hashes, and timestamp. It is required before any PHI leaves the workstation.

### `registry_submission` and `registry_submission_item`

Submission stores destination/profile, stable idempotency key, authorization, attempted/completed times, status, adapter version, and non-PHI error classification. Items link each artifact/immunization revision to its acknowledgement status.

Raw HL7 acknowledgements may contain PHI. Store them encrypted with the submission record; operational logs receive only submission ID, status class, and correlation ID.

States distinguish at least `PREPARED`, `AUTHORIZED`, `SENT`, `TRANSPORT_FAILED`, `ACK_ACCEPTED`, `ACK_ACCEPTED_WITH_WARNING`, `ACK_REJECTED`, and `RECONCILIATION_REQUIRED`.

## 11. AI and speech provenance model

### `assist_session`

Stores provider type/version, local model identifier/version, start/end timestamps, purpose, source type, and retention disposition. It must not store prompts/responses by default.

### `field_proposal`

During human review, holds the assist session, target draft/entity field, proposed typed value, confidence/uncertainty, source-span offsets/reference, schema version, and validation state. After accept/reject/cancel, raw proposal values and spans are deleted by default. Persist only minimum provenance and reviewer disposition; accepted values live in the ordinary domain revision and rejected values are not retained. A proposal never becomes clinical truth automatically.

### `speech_artifact`

Stores transient-file identifier, model/version, duration, language, creation/deletion times, and cleanup disposition without patient content. The MVP does not retain raw audio or an encrypted content reference. Patient names or transcript content never appear in filenames. Any later recording-retention feature requires a separate policy and data-model decision.

## 12. Audit model

### `audit_event`

| Field | Purpose |
|---|---|
| `audit_event_id`, sequence | Stable identity and facility/workstation ordering |
| `occurred_at`, `recorded_at` | User-event time and database time |
| actor/session/workstation/facility | Attribution |
| action code, outcome | What was attempted and whether it succeeded |
| entity type/id/revision | Target without embedding clinical values |
| changed field names | Minimal change description |
| reason/policy code | Structured accountability |
| correlation/causation IDs | Link one use case and related events |
| previous hash, event hash | Detect accidental alteration or broken sequence |
| software/schema version | Interpret old events correctly |

Audit events are part of the clinical/security data store, not ordinary application logs. They contain identifiers that may themselves be sensitive and receive the same encryption, access, retention, and backup controls as PHI.

The hash chain detects accidental damage and unsophisticated alteration; it is not proof against a privileged attacker who can rewrite the database and application state. A later threat model may justify keyed checkpoints anchored outside the database.

## 13. Operational metadata and logs

Operational logs may contain:

- Timestamp, severity, application component, event code.
- Random correlation ID.
- Duration, count, retry number, and non-sensitive error class.
- Application/version/platform metadata that does not identify a patient or user unnecessarily.

Operational logs must not contain patient/user names, dates of birth, addresses, identifiers, free text, transcript/audio, vaccine record details, consent, raw SQL parameters, registry payloads/ACKs, model prompts/responses, session tokens, passwords, or encryption keys.

## 14. Constraints and invariants

1. A finalized immunization revision cannot be updated or deleted.
2. A correction must reference the revision it supersedes and include reason, actor, and time.
3. Exactly one current non-void revision exists per logical clinical record.
4. An administered-here revision has one authenticated administering professional attestation.
5. AI/speech identities cannot be actors for administration, finalization, override, void, export authorization, or transmission authorization.
6. Applicable VIS delivery timestamps precede administration.
7. A registry artifact references only finalized immutable revisions and one profile version.
8. `REGISTRY_READY` requires a passing validation for the exact artifact inputs and profile version.
9. `ACK_ACCEPTED` requires a stored, parsed acknowledgement tied to the submission; a successful HTTP response alone is insufficient.
10. Every successful meaningful mutation has at least one audit event in the same transaction.
11. Audit records and referenced clinical revisions cannot be hard-deleted through normal application paths.
12. External transmission requires a matching authorization that enumerates the artifact hashes and destination.

## 15. Retention and deletion

Retention periods are policy/legal decisions and remain open. The model supports:

- Effective-dated retention policy versions.
- Legal/clinical holds.
- Logical retirement without silent historical deletion.
- Secure purge only through a separately approved, audited policy after all retention obligations expire.
- Backup expiration and verified secure disposal.

The MVP must not implement patient-record hard deletion until a reviewed retention policy defines authority, scope, backup handling, and evidence of destruction.

## 16. Major data architecture decisions

| ID | Decision and rationale | Alternatives considered | Primary risks | Future migration path |
|---|---|---|---|---|
| DM-001 | **Canonical clinical model separate from PREIS/HL7.** Preserves clinical meaning and isolates profile change. | Store raw HL7 only; mirror every PREIS segment as domain tables. | Mapping complexity and potential mismatch. | Add versioned adapters for new PREIS or FHIR profiles without migrating canonical history. |
| DM-002 | **Stable roots plus immutable finalized revisions.** Corrections retain original values and current state stays queryable. | In-place updates; full event sourcing. | More joins and storage; revision bugs. | Revisions become synchronization units or can project into a server database. |
| DM-003 | **Typed relational columns for core clinical facts; JSON only for bounded, versioned extension payloads.** Supports constraints and queries without designing every possible future field. | All-JSON documents; fully normalized schema for every code component. | Migration work when core fields evolve; extensions may become a dumping ground. | Promote proven extension fields into typed migrations while retaining original payload/version. |
| DM-004 | **Audit event and mutation share a transaction.** Prevents successful changes with missing audit history. | Asynchronous audit queue; application log as audit trail. | A failed audit write blocks clinical save; audit table grows. | Partition/archive in a future service while keeping atomic outbox semantics. |
| DM-005 | **Minimal PHI in audit events; values live in immutable revisions.** Enables accountability without duplicating sensitive facts. | Full before/after values in audit; no change detail. | Review requires joining revisions; identifiers are still sensitive. | Build authorized audit projections; retain encrypted event store. |
| DM-006 | **Registry artifacts and acknowledgements are immutable encrypted records.** Supports reproducibility and proof of outcome. | Regenerate on demand; keep only status flags. | Storage growth and retention complexity. | Policy-driven archival/purge; artifact hashes survive migration. |
| DM-007 | **Content/rules are versioned data with provenance.** Offline decisions remain reproducible after guidance changes. | Always fetch latest online; hard-code everything. | Stale packages and distribution/signing burden. | Signed update service or facility distribution channel without changing clinical revisions. |
| DM-008 | **Globally unique IDs and optimistic revisions from day one.** Low MVP cost preserves a multi-workstation option. | SQLite integer IDs and last-write-wins. | IDs alone do not solve distributed conflicts. | Add server reconciliation and device event ordering later. |
| DM-009 | **Keep database-key identity outside the encrypted schema as an opaque descriptor only.** SQLCipher needs key discovery before schema access without persisting the raw DEK. | Raw adjacent key; environment/config key; deterministic key derivation. | Descriptor loss makes the DB locally undiscoverable; copying it does not copy the protected credential. | Move descriptors into a signed installation manifest or device-enrollment service without changing clinical rows. |
| DM-010 | **Backups contain a complete encrypted database snapshot plus encrypted restore metadata.** This preserves migrations, audit and revisions as one consistent unit. | Logical table export; raw live file copy. | Format/schema compatibility and memory limits. | Add versioned streaming/logical migration while retaining immutable source backups. |

## 17. Migration and integrity rules

- Every schema migration has an immutable identifier, checksum, forward action, and tested rollback/recovery plan even if production rollback uses backup restore rather than a down migration.
- Start migration only after an encrypted backup and free-space check.
- Never infer or synthesize clinical facts to satisfy a new non-null column; use explicit unknown/not-applicable semantics and a reviewed backfill.
- Record software and schema versions in the authenticated non-PHI backup header; keep snapshot hashes, content references, audit data and the snapshot key in the authenticated encrypted payload.
- Run SQLite integrity and foreign-key checks after migration and restore.
- Verify the audit chain and current-revision pointers after migration.
- Use production-like encrypted databases and non-superuser access patterns in tests; mocks alone do not validate constraints.

## 18. Related documents

- [Product requirements](PRODUCT_REQUIREMENTS.md)
- [Architecture](ARCHITECTURE.md)
- [Security](SECURITY.md)
- [Roadmap](ROADMAP.md)
- [Foundation decisions](FOUNDATION_DECISIONS.md)
