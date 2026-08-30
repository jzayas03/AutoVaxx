# Phase 1 Implementation Contract

**Status:** Reconstructed from the product-owner-approved Phase 1 gate-closure request

**Date:** 2026-08-30

## Scope

Phase 1 establishes a synthetic-only, local-first technical foundation. It may implement and test authentication, authorization, audit, encrypted persistence, local key custody, backup/restore, provider boundaries, Tauri hardening, and build/security gates.

It does not authorize Phase 2 clinical workflow features, real PHI, production deployment, PREIS transmission, clinical eligibility or recommendation logic, local model integration, cloud services, telemetry, or cloud backup.

## Required invariants

1. React never receives database or backup keys and has no generic SQL, filesystem, shell, or network authority.
2. Rust authenticates and authorizes every privileged use case.
3. Development data is conspicuously synthetic; real PHI remains blocked.
4. An existing database key is never silently replaced when recovery fails.
5. The database DEK and each backup's keys are independent.
6. Backup creation and restore staging never require a plaintext SQLite database file.
7. Restore validates container authentication, SQLCipher integrity, schema/checksum, foreign keys, audit chain, and entity revisions before cutover.
8. Restore never overwrites the active database as its first action.
9. Audit and operational logs contain no key, recovery secret, patient value, or patient-bearing path.
10. Platform claims are limited to environments where the tests actually ran.

## Accepted technical shape

```text
React -> narrow Tauri commands -> Rust application/domain -> ports -> adapters

Windows Credential Manager -> database DEK -> SQLCipher -> repositories/migrations

live SQLCipher DB -> encrypted SQLCipher snapshot -> authenticated portable envelope
                    -> validated encrypted staging -> explicit authorized cutover
```

The Windows credential implementation may change behind `SecretStore`; a future macOS implementation must not require changes in domain or application code. Portable backup recovery must not depend on a workstation-bound Windows secret.

## Exit rule

Phase 1 remains **NOT PASSED** until the full matrix in [PHASE_1_ACCEPTANCE_CHECKLIST.md](PHASE_1_ACCEPTANCE_CHECKLIST.md) is executed on representative Windows 11 x64 hardware. CI on Windows Server is valuable evidence but is not a substitute for the Windows 11 clean-machine protocol.
