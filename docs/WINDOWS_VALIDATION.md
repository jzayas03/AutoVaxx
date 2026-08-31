# Windows 11 x64 Validation Protocol

**Status:** Unexecuted protocol. No row below is pre-marked pass.

Developer-host component evidence is recorded in [the 2026-08-31 Windows handoff](WINDOWS_BUILD_HANDOFF.md). It does not satisfy all protocol preconditions or close these rows. That smoke test observed external HTTPS connections in an application-owned WebView2 process; runtime egress denial remains an open acceptance blocker. A later diagnostic run identified `config.edge.skype.com` requests. Native credential-backed encrypted database, backup, key-loss and restore integration now passes as component evidence; it does not replace clean-workstation, second-account, authenticated workflow or OS-offline preconditions.

**Safety:** Use only conspicuously synthetic patients and sentinel `SYNTHETIC-PHI-EQUIVALENT-SENTINEL`. Never use real PHI. Preserve command output, screenshots, hashes, and artifact inventories in an access-controlled evidence package that contains no secrets.

## Environment record

Complete before testing:

| Field | Actual value |
|---|---|
| Windows edition/version/build and patch date | NOT RECORDED |
| Architecture | NOT RECORDED; required `x86_64` |
| CPU | NOT RECORDED |
| RAM | NOT RECORDED |
| System/database/backup filesystem and free space | NOT RECORDED |
| Local/domain/Microsoft account model; standard/admin rights | NOT RECORDED |
| WebView2 runtime version/source | NOT RECORDED |
| Rust toolchain and target | NOT RECORDED |
| Node and pnpm versions | NOT RECORDED |
| Visual Studio Build Tools, Windows SDK, CMake/Perl prerequisites | NOT RECORDED |
| Endpoint protection and relevant policy | NOT RECORDED |
| Git commit and lockfile hashes | NOT RECORDED |

## Test cases

For every row, replace `NOT RUN` only after execution. Evidence must identify the commit, host record, time, operator, command or action, and artifact/log location.

| Test ID | Precondition | Steps | Expected result | Actual result | Pass / Fail | Evidence |
|---|---|---|---|---|---|---|
| WIN-001 Fresh clone/build | Clean Windows 11 x64 VM; prerequisites recorded | Clone exact commit; verify hashes; run frozen install and debug builds offline after dependency cache is populated | Reproducible build; no undeclared downloads after cache; no secret/PHI files | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-002 Frontend tests | WIN-001 complete | Run format, lint, type-check, unit tests, Vite build, policy and license gates | All pass with zero warnings/errors | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-003 Rust tests | WIN-001 complete | Run default tests and full `synthetic-only,sqlcipher,windows-secret-store` suite with Clippy warnings denied | All pass; Windows secret test uses only random synthetic key and cleans it | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-004 Tauri release build | Frozen dependencies | Build release with recorded toolchain | Executable builds; provenance/native dependency inventory captured | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-005 Installer generation | WIN-004 complete | Generate NSIS; hash installer; inventory embedded/native files and notices | Installer generated without undeclared runtime dependency | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-006 Clean installation | Separate clean non-developer VM | Install as intended standard user; record prompts, files, ACLs, services/tasks | Install succeeds with least privilege; no unexpected service, startup item, or network use | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-007 First launch | WIN-006 complete; network disabled | Launch and inspect UI, data directory, process tree, arguments and environment | App launches offline in synthetic-only mode; no key/secret/PHI-equivalent in args/environment | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-008 Local authentication | Synthetic admin and clinical accounts | Exercise success, failure, lockout, expiry and role separation | Argon2 verification, safe errors, lockout and audit behave as specified | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-009 Argon2 benchmark | Minimum supported hardware candidate | Run release-mode benchmark repeatedly under idle/load conditions | Latency and memory are acceptable; selected parameters documented | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-010 WindowsSecretStore | Standard user; empty test credential name | Create/load/refuse replacement/delete synthetic 256-bit key; inspect access from second account | Credential Manager protects current-user secret; other account denied; typed failures; cleanup verified | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-011 Protected DB-key lifecycle | WIN-010 pass | Create encrypted DB; close/restart; recover key; inspect sidecar | Restart succeeds; sidecar has opaque ID only; raw DEK absent from files/logs/UI | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-012 SQLCipher create/open/restart | WIN-011 pass | Insert synthetic sentinel and records; close/reopen repeatedly | Data and migrations persist only with correct protected key | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-013 Migrations | Encrypted prior-schema fixtures and backups | Apply migration; interrupt controlled copy; reopen and validate checksum | Forward migration succeeds atomically; drift/interruption fails safely | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-014 WAL/journal/temp inspection | Active writes with sentinel | Inspect database directory, `-wal`, `-shm`, journal, `%TEMP%`, backup staging while files are active | No SQLite header or plaintext sentinel in data-bearing artifacts; modes/pragmas recorded | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-015 Invalid DB key | Copy DB; replace protected value with wrong 32-byte synthetic key in isolated test account | Launch/open; inspect files and audit/operational output | Safe `DATABASE_KEY_INVALID`; no replacement key or destructive rewrite | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-016 Database-copy theft | Copy DB and opaque descriptor to another account/machine without credential | Attempt open | Cannot recover key or read database; original remains usable | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-017 Encrypted backup | Valid encrypted source and recovery test secret | Create two backups; scan files/staging/process data; compare envelope metadata/keys | Consistent SQLCipher snapshots; distinct backup IDs/ciphertexts; no plaintext sentinel or raw keys | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-018 Backup corruption | WIN-017 backups | Corrupt header, tag, ciphertext; truncate; change version; remove metadata | Every case is rejected before cutover with safe error and no plaintext staging | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-019 Restore same workstation | Valid backup; authorized admin recently reauthenticated | Stage, review summary, explicitly cut over to a new destination, reopen | Schema/FK/audit/revision checks pass; new local DB key protected; source active DB untouched | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-020 Restore clean authorized workstation | Clean VM; approved recovery material available out of band | Transfer backup only; authenticate authorized admin; restore and reopen offline | Portable recovery succeeds without source DPAPI/Credential Manager state | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-021 Key-loss scenario | Tested backup exists | Remove isolated test credential; restart; attempt local DB; restore backup separately | Local open fails closed without key regeneration; approved backup recovery remains possible | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-022 Crash/restart | Disposable VM snapshots | Terminate during write, backup packaging, restore staging, and cutover boundaries | Active DB remains recoverable; partial files rejected/cleaned; no silent audit/schema repair | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-023 Disk-full behavior | Disposable volume with controlled low space | Exhaust space during DB write, backup, staging and cutover | Typed failure; active DB usable; incomplete output not accepted; no audit success claim | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-024 Offline operation | Network disabled at OS/firewall | Launch, authenticate, exercise foundation, backup and restore; capture connections | Core path works; no cloud/remote attempt; only intended local IPC | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-025 Tauri capability/CSP | Built artifact and source policy | Inspect generated capabilities, CSP, commands and runtime requests | Empty core-plugin permissions; no generic shell/fs/http authority; reviewed CSP unchanged | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-026 Uninstall/reinstall | Valid encrypted synthetic DB and backup | Uninstall; inventory retained/deleted files and credential; reinstall | Behavior matches approved retention/key policy; no accidental orphan or destructive deletion | NOT RUN | NOT RUN | NOT CAPTURED |
| WIN-027 Upgrade behavior | Signed prior test build and current build | Upgrade with encrypted fixture; verify schema, key reference, rollback/backup behavior | Data/key remain usable; migration is checked; downgrade fails safely | NOT RUN | NOT RUN | NOT CAPTURED |

## Mandatory artifact scan

During WIN-007, WIN-014, WIN-017, WIN-022, and WIN-023 inspect the database directory, WAL/journal files, Windows temp directories, backup staging, operational logs, crash output/dumps where policy permits, process arguments, process environment, installer extraction paths, and uninstall remnants. Search only for the synthetic sentinel. Record tool/version, path scope, timestamps, and results; do not store recovery secrets or Credential Manager contents in evidence.

## Acceptance rule

All production-critical cases must pass on representative Windows 11 x64 hardware and a clean authorized workstation. A Windows Server CI result, cross-compilation result, mocked secret store, or unexecuted row cannot close this gate.
