# Phase 1 Exit Review

**Review date:** 2026-08-30

**Branch:** `feat/phase-1-foundation`

**Data classification:** `SYNTHETIC_ONLY`

**Overall result:** **NOT PASSED — CONDITIONAL FOUNDATION ONLY**

The Phase 1 foundation is implemented and verified on the current macOS ARM64 development host. Phase 1 exit is not approved because the sole production target, Windows 11 x64, has not run the native validation suite; the Windows secret-store adapter is a fail-closed boundary rather than a DPAPI/CNG implementation; and SQLCipher, Windows key protection, and encrypted backup/restore have not been tested as one lifecycle. Real PHI, production deployment, and Phase 2 remain prohibited.

## 1. Completion report

### 1. Branch used

`feat/phase-1-foundation`, a dedicated non-`main` branch. No merge or commit to `main` occurred.

### 2. Git status

The repository began with no commits and all planning documents untracked. Phase 1 files remain uncommitted pending product-owner review; the final status must be checked at handoff. Generated dependency/build directories are ignored.

### 3. Architecture implemented

```text
React/TypeScript presentation
  -> narrow typed Tauri commands
    -> Rust application services and authorization
      -> Rust domain/state machine
        -> ports
          -> SQLite, backup, clock, logging, secret-store, and fake provider adapters
```

React receives no SQL, database key, generic filesystem, shell, process, or HTTP authority. Rust owns authorization, state transitions, validation, persistence, and audit. The result is a local modular monolith with opaque identifiers and repository boundaries that can migrate later without building multi-workstation infrastructure now.

### 4. Directory structure

| Path | Purpose |
|---|---|
| `src/` | React shell, routes, reusable components, localization, UI tests |
| `src-tauri/src/domain/` | Identity, permissions, clinical time, encounter state, audit/domain types |
| `src-tauri/src/application/` | Authentication, configuration gates, commands, authorization/use cases |
| `src-tauri/src/ports/` | Repository, backup, clock, secret-store, AI/speech/registry/barcode interfaces |
| `src-tauri/src/adapters/` | SQLite, backup, clock, safe logging, secret-store boundaries, unavailable providers |
| `src-tauri/capabilities/` | Minimal Tauri capability declaration |
| `scripts/` | Synthetic/security/network policy and license metadata gates |
| `.github/workflows/` | Frontend, Rust/macOS, secret-scan, and Windows foundation lanes |
| `docs/` | Requirements, decisions, architecture, data, security, spike, roadmap, exit evidence |

### 5. Domain types created

Implemented: `User`, `Role`, `Permission`, `Facility`, `Patient`, `PatientAddress`, `ExternalIdentifier`, `ImmunizationEncounter`, `AuditEvent`, typed IDs, `ClinicalTime`, and `EncounterState`.

Prepared as non-clinical boundaries only: `VaccinationAdministration`, `ScreeningRevision`, `ConsentRevision`, `VISDelivery`, and `ImmunizationRevision`. No clinical eligibility or recommendation engine was created.

Patient names preserve given names, middle names, first surname, second surname, suffix, and preferred name. No SSN field or automatic merge exists.

### 6. Database schema and migrations

Migration 001 creates schema metadata, facilities, workstations, users, roles, permissions, role mappings, sessions, patients, addresses, external identifiers, encounters, and append-only audit events. It enables foreign keys, secure deletion, WAL plus full synchronization for file databases, explicit transactions, and optimistic revisions. The stored migration checksum is a real SHA-256 digest and checksum drift fails closed before applying an edited migration.

The development database is permanently classified `SYNTHETIC_ONLY`; an existing unlabeled database is rejected rather than reclassified. There are no generic update/delete APIs for finalized future clinical records.

### 7. Authentication work

Application-local named-account foundations use Argon2id PHC verifiers with random salts and no recoverable password. Current provisional parameters are 65,536 KiB memory, three iterations, and one lane. A debug-host measurement took approximately 1,980 ms on this macOS host; this is evidence that measurement exists, not approval for minimum Windows hardware. Production selection requires the representative Windows benchmark.

Sessions expire after 15 minutes, recent authentication is required within five minutes for high-impact transitions, and five failures within 15 minutes trigger a 15-minute lock. Failed and successful authentication events are audited without disclosing whether a username exists. Compile guards reject production combined with `synthetic-only` or `dev-auth` and require SQLCipher for `production`.

### 8. Authorization work

Rust defines named permissions and four roles. Multiple-role permissions are combined. `FacilityAdministrator` has user/facility/backup authority but no patient or clinical authority unless separately assigned a clinical role. Application services enforce permissions independently of React, including direct forged-command denial tests. AI/provider ports have no state-transition authority.

### 9. Audit implementation

Audit events include actor/session/workstation/facility, action, entity reference/revision, outcome, UTC time, correlation ID, software/schema version, minimum metadata, previous hash, and event hash. Database triggers reject update and delete. Patient/encounter mutations append audit in the same SQLite transaction; rollback tests prove a failed mutation does not leave a partial record. Authentication attempts and authorized/denied backup attempts are audited.

Filesystem backup and restore cannot be made atomically transactional with SQLite audit. The application coordinator records success/failure without paths or PHI; a future production design must specify reconciliation when the filesystem operation succeeds but audit persistence fails.

### 10. SQLCipher spike results

The vendored SQLCipher/OpenSSL `rusqlite` feature compiled and passed encrypted-header, plaintext-marker absence, correct-key reopen, wrong-key rejection, and cipher-integrity tests on macOS ARM64. Windows packaging and integrated lifecycle evidence are absent, so the encryption gate fails. See [SQLCIPHER_SPIKE.md](SQLCIPHER_SPIKE.md).

### 11. SecretStore prototype results

`SecretStore` is a Rust-only port. The fake stores and returns a random 256-bit per-database key and fails closed when unavailable. `WindowsSecretStore` deliberately returns `SECRET_STORE_UNAVAILABLE`; DPAPI/CNG protection, ACLs, recovery scope, and clean-machine behavior require Windows implementation and validation. No secret crosses Tauri IPC.

### 12. Backup and restore results

The manual synthetic prototype takes a consistent SQLite online snapshot, creates a new versioned authenticated container, encrypts it with AES-256-GCM, wraps an independently random content key with an Argon2id passphrase-derived key, and writes with create-new semantics. Restore authenticates/decrypts to a new staging path, verifies the plaintext hash, SQLite quick-check, and `SYNTHETIC_ONLY` classification, and refuses to overwrite an existing destination. Tests cover creation, corruption rejection, staging, integrity, cutover, and no-overwrite behavior.

It is not scheduled, cloud-backed, UI-exposed, or integrated with a live SQLCipher database/Windows key store. The prototype creates a temporary plaintext snapshot and is therefore strictly synthetic-only; the production design must eliminate or explicitly protect and clean that exposure. Restore does not yet verify the complete audit chain, foreign keys, revision pointers, or content references.

### 13. Provider interfaces

Fake-first ports exist for `LocalAiProvider`, `SpeechToTextProvider`, `RegistryAdapter`, `BarcodeInput`, `BarcodeParser`, and `Clock`. Unavailable provider fakes fail predictably. Barcode work is syntax-only and performs no semantic resolution. There is no Ollama, llama.cpp, whisper.cpp, PREIS, or cloud integration.

### 14. Network controls

Phase 1 contains no external egress implementation. The source policy rejects frontend `fetch`/`XMLHttpRequest`, Rust socket/client primitives, and generic Tauri HTTP/shell/filesystem capabilities. The main window has an empty Tauri core-plugin permission list; registered application commands remain narrow Rust boundaries. The CSP allows only bundled/custom-protocol assets and the Tauri IPC origins (`ipc:` and `http://ipc.localhost`); it grants no general web origin, remote script, or remote API destination. Offline/provider-unavailable tests prove the manual foundation does not require a provider.

### 15. Localization implementation

All shell navigation, page headings, persistent synthetic warning, empty states, and shared component text use semantic English and Spanish catalogs. Locale selection is local UI state. Tests cover both languages and fallback to English. Clinical translations are not claimed or implemented.

### 16. Tests created

Rust tests cover the required patient, encounter, authorization, forged-command, role union/separation, authentication/configuration, audit immutability, safe logging, revision/rollback/restart, secret-store, backup/restore, provider, offline, clock, migration-checksum, and SQLCipher behaviors. Frontend tests cover routes, persistent synthetic mode, Spanish output, and localization fallback. All fixtures are conspicuously synthetic.

### 17. Commands executed

Key gates run on the current host:

```text
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm policy:check
pnpm license:check
pnpm audit --audit-level high
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features synthetic-only,sqlcipher sqlcipher_spike
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features production
cargo audit --file src-tauri/Cargo.lock
pnpm tauri build --no-bundle
gitleaks detect (scoped source/configuration directories)
```

The Windows CI commands are defined but have not run on this host.

### 18. Test results

The final macOS run passed 4 frontend tests, 31 default-feature Rust tests, and 2 SQLCipher-feature tests with zero failures. Formatting, ESLint, TypeScript, Vite, Clippy with warnings denied, production-feature compilation, policy/CSP/capability checks, license metadata review, npm/Rust dependency audits, scoped source secret scans, and the Tauri release build all passed. The release binary was produced at `src-tauri/target/release/autovaxx`. RustSec reported zero vulnerabilities and 17 allowed warnings described below. The Argon2id benchmark reported approximately 1,980 ms in the debug test profile. No Windows result is claimed.

### 19. Acceptance checklist status

See sections 2 and 3. The synthetic functional checklist passes on macOS; the Phase 1 exit gate does not.

### 20. Windows-specific items not validated

- Windows 11 x64 application launch and NSIS clean-machine install/upgrade/uninstall
- SQLCipher/OpenSSL native linkage, redistributed files, notices, signing, and update provenance
- DPAPI/CNG-backed secret storage, Windows ACLs, account/machine scope, and key-loss recovery
- Active WAL/journal/temp/crash artifact encryption and endpoint-protection interaction
- SQLCipher migration/corruption/power-loss/disk-full behavior and minimum-hardware performance
- Integrated database-key/backup/restore/rekey lifecycle and clean-workstation recovery
- WebView2/runtime prerequisites and representative pharmacy peripherals

### 21. Security findings

- `cargo audit` initially found vulnerable `quick-xml` 0.38.4 and `time` 0.3.45 versions held by a transitive Tauri/plist chain under the earlier Rust-version floor. Raising the floor to Rust 1.88 and updating the lockfile to `plist` 1.10.0, `quick-xml` 0.41.0, and `time` 0.3.55 removed known vulnerabilities.
- Current RustSec review reports no vulnerabilities but retains warnings for transitive unmaintained packages and a `glib` advisory in the cross-platform Tauri graph. Most GTK packages are Linux-target dependencies even though Windows is the production target; they remain tracked supply-chain debt rather than ignored evidence.
- `pnpm audit --audit-level high` reports no known vulnerabilities.
- The license metadata gate found no missing or prohibited license metadata across the installed npm and Cargo package inventories. `r-efi` offers permissive MIT/Apache alternatives to its LGPL option. SQLCipher attribution and Community-versus-Commercial selection remain open.
- Scoped gitleaks scans of `src`, `src-tauri/src`, `scripts`, `docs`, and `.github` found no secret. CI performs a clean-checkout full repository scan.
- Capability review found that Tauri's generated `core:default` set granted unnecessary window/menu/image/path APIs. It was replaced with an empty core-plugin permission list, and CI now fails on capability or reviewed-CSP drift.
- No HIPAA compliance, encryption-at-rest readiness, or production security claim is made.

### 22. Known limitations

- Synthetic-only shell; no production-ready startup configuration.
- Windows secret store is not implemented.
- The ordinary SQLite foundation database is plaintext by design and cannot hold real PHI.
- SQLCipher proof is macOS-only and is not wired to application startup.
- Backup proof is synthetic and not integrated with SQLCipher/key custody.
- No scheduled backups, production account administration/recovery UI, export, printing, clinical workflow, deterministic clinical rules, VIS packages, registry candidate, or real providers.
- No Windows, peripheral, accessibility, usability, incident, downtime, installer-signing, or restore-drill evidence.
- Filesystem operations and audit are coordinated but cannot share a single atomic transaction; reconciliation policy is unresolved.

### 23. Documentation updated

`AGENTS.md`, product requirements, architecture, data model, security baseline, roadmap, and foundation decisions now record the approved Decisions 1, 2, and 6, Phase 1 authorization, and the Phase 2 boundary. This spike report and exit review were added. Approval wording was not extended to unresolved legal, clinical, PREIS, encryption-distribution, backup-custody, or real-PHI decisions.

### 24. Recommended Phase 2 scope

**Do not begin Phase 2 yet.** First close or explicitly disposition the failed Phase 1 exit gates on Windows. After a new product-owner authorization, the recommended Phase 2 is the narrow documentation-only vertical slice already described in the roadmap: patient search/create, resumable encounter draft, screening response capture without interpretation, consent/VIS evidence, product/lot documentation, explicit professional administration confirmation, finalization/correction/void history, and an inspectable registry candidate only against a verified profile. Keep Ollama and whisper.cpp optional and local; do not add clinical eligibility, recommendations, a general clinical DSL, or PREIS transport.

## 2. Required test checklist

| Required behavior | macOS synthetic result | Evidence boundary |
|---|---|---|
| Patient creation and retrieval | PASS | Rust command/repository tests |
| Encounter creation | PASS | Rust command/service tests |
| Valid and invalid transitions | PASS | Domain table tests |
| Authorization allow/deny | PASS | Rust service tests |
| Forged Tauri-command denial | PASS | Direct command implementation test |
| Multiple-role union | PASS | Permission tests |
| Administrator lacks clinical authority | PASS | Permission and command denial tests |
| Audit append and API/database immutability | PASS | Repository tests and update/delete triggers |
| Operational-log PHI protection | PASS | Typed fields reject synthetic PHI markers |
| Stale revision rejection | PASS | Repository transaction test |
| Transaction rollback | PASS | Mutation/audit rollback test |
| Restart persistence | PASS | File database close/reopen test |
| Migration checksum drift | PASS | Tampered checksum rejection test |
| Key-store unavailable | PASS for fake boundary | Real Windows behavior not tested |
| Backup creation | PASS for synthetic ordinary SQLite | Integrated SQLCipher path not tested |
| Corrupted backup rejection | PASS | AEAD corruption test |
| Staged restore and integrity | PASS for prototype | Full application integrity checks incomplete |
| Provider unavailable | PASS | Fake-first ports |
| Complete offline operation | PASS for Phase 1 foundation | No external adapter exists |
| Localization fallback | PASS | React tests |
| Production rejects DEV_ONLY auth | PASS | Compile/runtime configuration tests |
| Production rejects plaintext real-PHI | PASS | Compile/runtime configuration tests |
| SQLCipher correct/wrong key and raw marker | PASS on macOS | Windows not tested |

## 3. Phase 1 exit gates

| Exit gate | Status | Reason |
|---|---|---|
| Dedicated non-main branch and synthetic-only scope | PASS | Branch and compile/runtime/data-classification guards verified |
| Architectural trust boundaries | PASS | Narrow commands, Rust authority, ports/adapters, minimal capabilities |
| Functional foundation and automated tests | PASS on macOS | Required synthetic behaviors pass locally |
| No external egress required | PASS for Phase 1 | CSP, capabilities, source policy, unavailable providers |
| Dependency, license, and secret review | PASS with tracked warnings | No known vulnerability/prohibited license/secret; RustSec warnings remain |
| Windows 11 x64 installer and runtime | **FAIL** | Workflow is defined but unexecuted |
| Production encrypted database lifecycle | **FAIL** | SQLCipher verified only as an isolated macOS spike |
| Windows SecretStore lifecycle | **FAIL** | Adapter is intentionally unavailable |
| Integrated encrypted backup and clean-workstation restore | **FAIL** | Prototype uses synthetic ordinary SQLite and lacks full integrity checks |
| Representative-hardware Argon2/performance selection | **FAIL** | Only macOS debug-host timing exists |
| Real-PHI authorization | **FAIL / NOT REQUESTED** | Legal, clinical, deployment, recovery, and security gates remain open |

## 4. Exit decision

The code is a useful, synthetic-only Phase 1 foundation, but **Phase 1 exit does not pass**. Keep the branch unmerged, do not use real patient data, do not deploy to production, and do not start Phase 2 until the product owner reviews this evidence and authorizes the next bounded plan.
