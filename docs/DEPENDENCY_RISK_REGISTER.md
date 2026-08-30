# Dependency Risk Register

**Review date:** 2026-08-30

`cargo audit` scanned 452 locked Rust packages: zero vulnerabilities and 17 allowed warnings. The license gate reviewed 450 Cargo package records and 234 installed npm packages. `pnpm audit --audit-level high` reports no known vulnerabilities. Warnings are tracked, not suppressed.

| Advisory / packages | Classification | Runtime relevance | Decision and control |
|---|---|---|---|
| RUSTSEC-2024-0411 through 0420: `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros` 0.18.2 | Accepted transitive warning; replaceable only through upstream Tauri/wry Linux backend change | Linux-target GUI graph; not the Windows MVP runtime | Keep Tauri current; inspect `cargo tree --target all`; reevaluate if Linux becomes supported or Tauri removes GTK3 |
| RUSTSEC-2024-0429: `glib` 0.18.5 unsound iterator implementation | Accepted transitive warning; runtime concern on affected Linux GTK path | Linux target, not Windows MVP runtime; no direct AutoVaxx use of affected iterator | Do not suppress; track upstream Tauri/wry migration and reassess any Linux scope |
| RUSTSEC-2024-0370: `proc-macro-error` 1.0.4 | Accepted transitive warning; build-only concern | Procedural macro/build graph; not shipped runtime logic | Track upstream replacement; no direct dependency or runtime exposure |
| RUSTSEC-2025-0075, 0080, 0081, 0098, 0100: `unic-char-range`, `unic-common`, `unic-char-property`, `unic-ucd-version`, `unic-ucd-ident` 0.9.0 | Accepted transitive warning; removable by upstream dependency update | Pulled by `urlpattern` through `tauri-utils`; cross-platform parsing/runtime/build graph | Keep Tauri/urlpattern current; test upgrade when upstream replaces `unic-*` |

## New Phase 1 dependencies

| Dependency | Purpose | Risk/decision | Migration path |
|---|---|---|---|
| `keyring-core` 1.0.0 / `windows-native-keyring-store` 1.1.0 | Windows Credential Manager adapter | Direct Windows-only provider avoids unrelated platform stores and explicitly selects `Local` persistence; account behavior still requires Windows 11 testing | Replace adapter with an approved direct DPAPI/CNG or enterprise custody implementation |
| SQLCipher 4.14/OpenSSL through `rusqlite` | Encrypted SQLite pages | Native build, patch ownership, installer provenance and commercial-support choice remain release risks; SQLCipher [fixed a Windows `VirtualLock` logging recursion](https://github.com/sqlcipher/sqlcipher/commit/afbb132d60d421fd7b20d073e4448af3dcb5c61d) in 4.18 | Set SQLCipher logging to `ERROR` before enhanced memory security, fail closed if rejected, exercise the encrypted Windows suite, and upgrade to 4.18+ when the binding supplies it; switch bundled/system/commercial SQLCipher behind the repository adapter if needed |
| `aes-gcm`, `argon2`, `zeroize` | Standard backup AEAD, recovery KDF, key-memory cleanup | One-shot envelope currently buffers large backups; secrets cannot be guaranteed absent from all OS memory artifacts | Versioned streaming envelope and/or approved recovery provider |

## Review cadence

Run RustSec, npm audit, license metadata, secret scan, lockfile diff, Tauri capability review, and CSP policy on every dependency change and release candidate. A warning may be accepted only with target/runtime reachability, owner, control, and migration path documented here.
