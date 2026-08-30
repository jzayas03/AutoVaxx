# SQLCipher Integrated Engineering Evidence

**Status:** Integrated path verified on macOS ARM64 with synthetic data; Windows 11 encryption gate **NOT PASSED**

**Date:** 2026-08-30

## Decision

Retain `rusqlite` with `bundled-sqlcipher-vendored-openssl` behind the `sqlcipher` feature as the Phase 1 candidate. It now runs the database-key, repositories/migrations, WAL, backup, and restore paths rather than an isolated spike only. Real PHI remains prohibited.

## Options and migration

| Option | Rationale | Risk | Migration path |
|---|---|---|---|
| Bundled SQLCipher + vendored OpenSSL (current) | Reproducible current-host tests and one Rust repository API | Native supply chain, Windows build prerequisites, patch and installer ownership | Switch Cargo/build inputs without changing domain/repository ports |
| System or supported commercial SQLCipher | Central patches or vendor support | Installation/procurement and binary provenance | Same repository/key contracts |
| Field-level encryption | Selective protection | Query/index complexity and easy journal/temp leakage | Not recommended; would be a new persistence adapter |
| Plain SQLite plus disk encryption | Simple | Does not protect copied DB/backup from an authorized OS session | Rejected as sole PHI control |

## VERIFIED on current macOS host

- Create and reopen an encrypted database through a recovered 256-bit key.
- Migrations and migration checksum checks operate on the encrypted connection.
- Foreign keys are enabled; WAL mode and full synchronous mode are applied after the key.
- Repository transactions, expected-revision rejection, audit atomicity and restart behavior remain intact.
- Wrong keys fail safely; missing/corrupt/denied/unavailable protected keys do not generate replacements.
- Database and active WAL-directory artifacts do not reveal the SQLite header or synthetic sentinel in the exercised test.
- A consistent online backup from the live encrypted connection writes to a separately keyed SQLCipher snapshot.
- The portable backup envelope and restore staging contain no plaintext SQLite database.
- Restore checks SQLCipher/SQLite integrity, schema/checksum, foreign keys, audit-chain hashes, JSON audit metadata and entity revision minima.

Current command:

```text
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sqlcipher --all-targets
```

The integrated run passed 48 tests on the current host. This is library behavior, not Windows packaging/runtime evidence.

## PARTIALLY VERIFIED

- The Windows Rust target and Windows credential-store crates were downloaded. A macOS cross-check compiled the Windows credential crates, then stopped at native SQLite/OpenSSL cross-compilation because the macOS toolchain lacks the Windows MSVC/Perl environment. This does not count as a Windows build.
- CI now runs the complete SQLCipher and real Windows Credential Manager synthetic tests on a Windows runner. Results count only after an actual workflow run is retained; GitHub's Windows Server runner is not representative Windows 11 acceptance hardware.

## REQUIRES WINDOWS VALIDATION

Execute [WINDOWS_VALIDATION.md](WINDOWS_VALIDATION.md), especially native SQLCipher/OpenSSL linkage and notices, installer/clean launch, Credential Manager account scope, active WAL/journal/temp inspection, wrong-key/key-loss, backup/restore to a clean workstation, crash, disk-full, upgrade and performance cases.

## WAL, temp and backup conclusion

The exercised macOS file-backed connection uses encrypted WAL and produced no raw synthetic marker across files present in the database directory. Backup staging is an encrypted SQLCipher file and is deleted on failure/drop. This does not prove Windows temp, crash dump, endpoint-protection, page-file, installer or interrupted-I/O behavior.

## Risks and controls

| Risk | Current control | Closure |
|---|---|---|
| Key loss | Fail closed; separate portable recovery envelope | Approve custody and run clean-workstation restore drill |
| Plaintext side artifact | Key before schema access; encrypted snapshot/staging; sentinel tests | Windows-wide artifact scan and crash/disk-full tests |
| Native dependency drift | Lockfile, RustSec/license gates, risk register | SBOM, notices, signed installer, patch owner |
| Large in-memory backup envelope | 2 GiB cap and encrypted snapshot on disk | Introduce versioned streaming format before size demands it |
| Binding lock-in | Domain/application use ports and repositories | Maintain contract tests for replacement adapter |

## Gate result

**NOT PASSED:** integrated technical behavior is verified on macOS, but the sole production target has not completed Windows 11 x64 validation. Do not use real PHI or begin Phase 2.
