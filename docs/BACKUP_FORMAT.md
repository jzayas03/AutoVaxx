# AutoVaxx Backup Format

**Format:** `AVXBAK02`, version 2

**Status:** Implemented and verified with synthetic data on macOS; recovery custody and Windows 11 validation remain open.

## Decision

The portable backup is a versioned authenticated envelope around a transactionally consistent, independently keyed SQLCipher snapshot. It never requires a plaintext SQLite staging file and does not depend on Windows Credential Manager for portable recovery.

```text
live SQLCipher database
  -> SQLite online backup into fresh-key SQLCipher snapshot
  -> snapshot key + encrypted snapshot + restore metadata
  -> AES-256-GCM authenticated encrypted payload
  -> content key wrapped by Argon2id-derived key + AES-256-GCM
  -> atomic create-new container
```

## Binary layout

| Offset | Length | Meaning |
|---|---:|---|
| 0 | 8 | ASCII magic `AVXBAK02` |
| 8 | 2 | Big-endian format version (`2`) |
| 10 | 4 | Big-endian JSON header length |
| 14 | variable | UTF-8 JSON unencrypted authenticated header |
| next | remainder | AES-256-GCM ciphertext and authentication tag |

Header length is nonzero and capped at 16 KiB. Container size is capped at 2 GiB in Phase 1. Unsupported versions fail closed; version 1 is deliberately not supported because it came from the plaintext-staging prototype. There is therefore no older supported production format yet.

## Unencrypted authenticated header

The header contains only non-PHI compatibility and cryptographic metadata:

- Format and format version.
- Software and schema versions.
- Opaque random backup ID and UTC creation time.
- Payload cipher, database cipher, key-wrap and KDF identifiers/parameters.
- Random salt and nonces.
- Wrapped content key, never the raw content key.
- Platform-neutral compatibility marker and synthetic-only classification.

The exact serialized header bytes are AES-GCM associated data for the encrypted payload. Any modification is detected. The backup ID, magic, version, and salt are also associated data when unwrapping the content key.

## Authenticated encrypted payload

The decrypted envelope payload contains:

1. Length-prefixed JSON metadata with schema version, encrypted snapshot length and SHA-256, audit-chain inclusion flag, and content-package references.
2. A fresh random 256-bit SQLCipher snapshot key.
3. The encrypted SQLCipher database snapshot, including migrations, application metadata, audit events, and clinical records.

The snapshot key is encrypted inside the outer envelope and is never in the unencrypted header. The live database DEK is not copied into the backup. A fresh random outer content key and fresh random SQLCipher snapshot key are generated for every backup.

## Recovery envelope

Phase 1 implements one recovery-envelope provider: Argon2id 1.3 (`64 MiB`, three iterations, one lane) derives a 256-bit wrapping key from the provided recovery secret and random salt; AES-256-GCM wraps the random outer content key. The interface is intentionally conceptually separable so an approved enterprise escrow/HSM/recovery-key provider can replace it without changing the payload format.

The permanent custody, rotation, multi-person control, and lost-secret process is not approved. A recovery secret must never be placed in the container, command line, environment variable, logs, audit metadata, or React state.

## Restore validation and cutover

Restore rejects malformed/missing metadata, wrong secrets, header/tag/ciphertext changes, truncation, future versions, snapshot hash/length mismatch, SQLCipher failure, schema/checksum mismatch, foreign-key errors, audit-chain mismatch, or invalid revision minima. Validation occurs on a separate encrypted staging file. An authorized cutover creates a new workstation-local key reference and moves to a nonexisting destination; it never overwrites the active database first.

## Major decision record

| Item | Decision |
|---|---|
| Rationale | Preserve offline portability while keeping workstation-local database custody separate and eliminating plaintext SQLite staging. |
| Alternatives | Raw SQLCipher file copy (not transactionally safe under active WAL); plaintext export then encrypt (prohibited exposure); DPAPI-only backup (not portable); proprietary crypto (prohibited). |
| Risks | Current implementation buffers a maximum 2 GiB envelope in memory; recovery-secret custody is unresolved; native SQLCipher behavior needs Windows testing; filesystem and audit cannot share one transaction. |
| Future migration | Add a streaming AEAD format as a new version, add recovery-envelope providers, include approved content-package manifests, and retain an explicit versioned decoder/migration policy. |
