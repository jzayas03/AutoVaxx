# SQLCipher Engineering Spike

**Status:** Conditional recommendation; Phase 1 encryption gate **NOT PASSED**

**Date:** 2026-08-30

**Scope:** Synthetic-data feasibility only. No result in this document authorizes real PHI or production deployment.

## 1. Decision

Keep `rusqlite` with a feature-gated SQLCipher build as the leading persistence implementation. The spike proves that the application can compile and exercise an encrypted SQLite-compatible database on the current macOS ARM64 development host. It does not prove the approved Windows 11 x64 production distribution.

The encryption gate remains closed until a representative Windows build proves packaging, secret protection, database lifecycle, backup/restore, corruption behavior, performance, and clean-machine installation together.

## 2. Options evaluated

| Option | Rationale / result | Primary risk | Migration path |
|---|---|---|---|
| `rusqlite` + system SQLCipher | Small Rust API change and system security updates can be managed separately. | Reproducible Windows installation and DLL provenance become deployment dependencies. | Keep repository traits; replace build/link configuration. |
| `rusqlite` `bundled-sqlcipher` | Reproducible native build without a separately installed SQLCipher library. | OpenSSL discovery and Windows toolchain behavior need proof. | Change only Cargo feature/build inputs. |
| `rusqlite` `bundled-sqlcipher-vendored-openssl` | Most self-contained spike and the option verified now. | Larger native supply chain, security-update ownership, installer size, and Community-versus-Commercial support choice. | Preserve repository API and switch to system or supported commercial artifacts later. |
| Field-level encryption over ordinary SQLite | Avoids SQLCipher distribution. | High schema/query complexity; easy to leave indexes, metadata, journals, or temporary values exposed. | Not recommended unless a later threat model requires selective defense in depth. |
| Alternative SQLite stacks (`sqlx`, `libsql`) | Could offer async or remote-oriented APIs. | Adds churn and does not remove the need to validate a supported SQLCipher distribution; remote orientation conflicts with this MVP. | Repository ports prevent the domain from depending on `rusqlite`. |

The selected spike feature is `rusqlite/bundled-sqlcipher-vendored-openssl`. It is isolated behind Cargo feature `sqlcipher`; the default development build remains explicitly `SYNTHETIC_ONLY`.

## 3. VERIFIED NOW

Environment: Darwin 25.5.0 ARM64, Rust 1.98.0, `rusqlite` 0.40.2 through the lockfile.

- The SQLCipher feature compiled from source.
- `PRAGMA cipher_version` returned a non-empty version.
- A database keyed with a random 256-bit synthetic key did not expose the ordinary `SQLite format 3` header.
- A planted synthetic marker was absent from raw database bytes.
- Reopening with the correct key returned the stored value.
- Querying with the wrong key failed.
- `cipher_integrity_check` returned an accepted result.
- The same repository migration API compiles with both ordinary bundled SQLite and the SQLCipher feature.
- The separate encrypted-backup prototype uses a consistent SQLite online snapshot, an authenticated AES-256-GCM container, an independently random content key, and Argon2id passphrase wrapping. It rejects a corrupted container and restores only to a staged path.

Verification command:

```text
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features synthetic-only,sqlcipher sqlcipher_spike
```

These tests are in `src-tauri/src/adapters/sqlite/sqlcipher_spike.rs`. They do not claim that every plaintext remnant, journal mode, temporary file, or crash condition has been tested.

## 4. REQUIRES WINDOWS VALIDATION

Run all items on representative, fully patched Windows 11 x64 hardware and then on a clean machine without developer toolchains:

1. Build `x86_64-pc-windows-msvc`, create the NSIS installer, install, launch, upgrade, uninstall, and verify WebView2/runtime prerequisites.
2. Confirm whether SQLCipher/OpenSSL are statically or dynamically linked and inventory every redistributed native artifact, notice, hash, and update owner.
3. Exercise migrations, foreign keys, WAL/checkpoint/recovery, rollback journals, temporary storage, busy handling, power/process interruption, disk-full behavior, and database corruption.
4. Verify database, `-wal`, `-shm`, journal, snapshot, restore staging, and temporary artifacts contain no plaintext synthetic marker.
5. Implement and test the Windows `SecretStore` adapter using an approved DPAPI/CNG-compatible protection scope and Windows ACL design; test missing/corrupt key material and account/machine recovery.
6. Wire the SQLCipher database key, secret store, and encrypted backup path together; test clean-workstation restore and total local-key loss without silently overwriting the active database.
7. Benchmark application startup, migration, common repository operations, backup, restore, and Argon2id on minimum supported hardware.
8. Test endpoint protection/antivirus interaction, code signing, installer signing, reproducible builds, vulnerability response, and upgrade/rekey procedures.

The Windows CI definition is useful future evidence only after it has actually run; presence of workflow YAML is not a passing test.

## 5. WAL, temporary data, backup, and corruption conclusions

- SQLCipher is expected to encrypt database pages and its own journaling modes, but this project has not yet measured every Windows artifact. The production configuration must set and verify the chosen cipher and SQLite pragmas on every connection before schema access.
- The current SQLCipher tests checkpoint before raw-file inspection; a dedicated active-WAL test is still required.
- The backup container cryptography is independent from the database encryption key. That separation is desirable for portable recovery, but the current prototype creates a temporary plaintext snapshot of a synthetic ordinary-SQLite source before encrypting the container. It is therefore prohibited for real PHI and is not evidence of an integrated encrypted production backup. The production path must avoid or tightly protect/clean any plaintext staging artifact and prove the result on Windows.
- Authenticated encryption and the plaintext snapshot hash detect container corruption after decryption. SQLite quick-check and classification are verified on staging. Audit-chain, foreign-key, migration-compatibility, schema-checksum, and application-level revision checks must be added to the integrated restore gate.

## 6. Security maintenance and licensing

SQLCipher Community is published under a permissive BSD-style license with attribution obligations; commercial editions add supported binary packages and commercial support. The open-source option is technically feasible, but no Community-versus-Commercial procurement decision is approved. See [Zetetic licensing](https://www.zetetic.net/sqlcipher/license/) and [Community Edition](https://www.zetetic.net/sqlcipher/community/).

Bundling SQLCipher and vendored OpenSSL makes AutoVaxx responsible for native dependency inventory, security advisories, timely rebuilds, notices, and installer provenance. A supported commercial distribution may reduce packaging/support risk but adds vendor cost and procurement dependency. Security and legal owners must select the distribution before real PHI.

## 7. Risks and controls

| Risk | Current control | Required closure evidence |
|---|---|---|
| macOS success mistaken for Windows readiness | Explicit failed gate and separate evidence sections | Windows test report and clean-machine installer evidence |
| Key stored beside database or exposed to React | `SecretStore` port; Rust-only random key generation; no key IPC | Working Windows adapter and access/backup recovery tests |
| Plaintext side artifacts | Production rejects plaintext real-PHI configuration | Windows artifact scan across WAL/temp/backup/crash paths |
| Native library vulnerability or license drift | Locked dependencies and dependency/license review | SBOM/notices, ownership, monitoring, rebuild procedure |
| Lost key makes clinical records unavailable | Portable backup format is independent from local key | Approved custody and clean-workstation restore drill |
| Binding lock-in | Domain/application depend on repositories, not SQLCipher types | Maintain contract tests when changing adapter |

## 8. Gate result

**FAIL / OPEN:** the technical candidate is viable, but the Phase 1 encryption exit gate does not pass on macOS evidence alone. Real PHI remains prohibited. Re-evaluate after Windows-native validation and after the SQLCipher, Windows secret store, and encrypted backup/restore paths operate as one tested lifecycle.
