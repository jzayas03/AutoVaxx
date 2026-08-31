# Windows build and Mac handoff — 2026-08-31

> Windows developer builds, scoped smoke checks, and dependency audits passed. Full Windows security/acceptance is **not closed**: application-owned WebView2 made external HTTPS connections, and clean-machine/recovery cases remain.

## Decision and scope

The owner approved installing developer tools and necessary synthetic application checks to finish as much Windows work as practical before returning this borrowed laptop. This pass provides builds, test evidence, and a portable Mac handoff. It does not authorize Phase 2, real PHI, clinical decisions, live registry transmission, signing, or production use.

The complete [Windows validation protocol](WINDOWS_VALIDATION.md) is **not closed**. Its clean-VM, second-account, OS-level offline, recovery, destructive-failure, installation, and upgrade preconditions are not replaced by developer-machine unit tests. No canonical protocol row was relabeled PASS.

The earlier SEC-008 no-key helper compatibility milestone is closed. Full SEC-008 signing is not closed; SEC-008 remains unselected. Its review material is separate from the synthetic application Credential Manager test.

## Source identity and reviewed changes

- Base: `90b20668103590d0b77e1076868ff5705793a0c7` (merge PR #6, SQLCipher Windows log-recursion fix).
- Branch: `feat/phase-1-foundation`, created from initially clean `main`.
- Checkout: `C:\Users\janza\OneDrive\Desktop\AutoVaxx`.
- **Uncommitted and unpushed:** pulling on the Mac alone will not recover these changes.
- pnpm-lock.yaml SHA-256: `20892eb3e96947f0f3d4dea0a70880f896c25db370b96b9d98c1a79f161e70c9`.
- src-tauri/Cargo.lock SHA-256: `c24d1c6646cd0bae5da522023f81eaf8b703b115cee89d38552a392a222413f6`.
- No dependency declaration, lockfile, clinical behavior, schema, capability, CSP, or production adapter logic changed.

Changes:

1. `scripts/policy-check.mjs` uses `fileURLToPath` to fix a duplicated Windows drive prefix, and normalizes relative separators so the existing self-exclusion works on Windows. Forbidden network-call and embedded-secret checks remain enforced.
2. `.github/workflows/ci.yml` adds format and source-policy checks to the Windows job. Remote CI was not dispatched.
3. `.gitattributes` sets LF for text and CRLF for Windows batch files. All 31 initial formatting failures were verified as line-ending differences only, then normalized without content changes.
4. The test helper in `src-tauri/src/adapters/secret_store.rs` cleans up its newly created credential even when assertions panic, verifies absence, then resumes the failure. It never adopts a pre-existing entry. The native test zeroizes recovered bytes and avoids printing them on equality failure. A fake-store regression exercises panic cleanup.
5. A Windows-only SQLCipher integration test in `database_key.rs` now exercises encrypted create/reopen, active database/WAL/descriptor scans, encrypted backup, deletion of its own original credential, fail-closed key loss, and restore to a new destination through the real Windows secret store. A test wrapper tracks only successfully created references, cleans them on unwind, and verifies absence. This changes test coverage, not production behavior.
6. This report and its verification companion record evidence and remaining work. Security and validation documents link the newly observed runtime-egress blocker without weakening the deny-by-default requirement.

Git may initially show stat-only changes after line-ending normalization; the actual diff identifies the content edits.

## Host and installed developer tools

All execution occurred on the intended local Windows laptop. This is not a clean acceptance VM or certified minimum supported hardware.

| Item                  | Observed value                                                                    |
| --------------------- | --------------------------------------------------------------------------------- |
| OS                    | Windows 11 Home/Core 25H2, build 26200.9168, x64; patch-install date not recorded |
| CPU / RAM             | Intel Core i7-1195G7 at 2.90 GHz / 15.7 GiB                                       |
| C:                    | NTFS, 458.7 GiB total; 352.0 GiB free before installation                         |
| Existing Node         | 24.19.0, bundled Codex runtime                                                    |
| Installed pnpm        | Project-pinned 9.0.4; bundled 11.19.0 was only a bootstrap tool                   |
| Rust / Cargo          | 1.98.0 / 1.98.0, MSVC x64 target; rustfmt and Clippy installed                    |
| Rustup                | 1.29.0, minimal profile, no persistent PATH edit                                  |
| Microsoft Build Tools | Visual Studio 2022, 17.14.37614.0 / 17.14.39                                      |
| MSVC / SDK            | Tool directory 14.44.35207, compiler 19.44.35228.0 / SDK 10.0.26100.0             |
| Native prerequisites  | CMake 3.31.6-msvc6 and nmake; portable Strawberry Perl 5.42.2, package 5.42.2.1   |
| NASM                  | Not installed; locked OpenSSL supports automatic no-assembly fallback             |
| Existing WebView2     | 152.0.4191.53                                                                     |
| Local scanners        | Gitleaks 8.30.1; cargo-audit 0.22.2                                               |
| Endpoint protection   | User screenshots show McAfee; active policy unverified                            |

User-local tools, caches, outputs, and logs are under `C:\Users\janza\AppData\Local\AutoVaxx-DevTools`. Microsoft tools are under `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`. Necessary administrator installation was owner-approved. Repository ownership/ACLs, persistent Git trust, firewall, sync, and PHI policy were not changed.

The Microsoft installer returned **3010 (reboot requested)**. No reboot was performed. Later `vswhere` reported complete, launchable, and `isRebootRequired=false`. Native compilation works; both observations are retained.

Rustup, portable Perl, Gitleaks, and cargo-audit downloads matched publisher checksums/digests. The Microsoft bootstrapper had a valid Microsoft Authenticode signature. Exact artifact hashes are in the verification manifest. Dependencies were downloaded without changing either lockfile. pnpm used `--frozen-lockfile --ignore-scripts`; frontend builds succeeded with those installed packages.

## Verification

Logs and result records are under the tool root's `evidence` directory and are included in the final package. Initial failures are retained separately from final verified results.

| Check                                             | Result and scope                                                                                                                        |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Frontend format, lint, type-check                 | PASS after line-ending fix                                                                                                              |
| Vitest / Vite                                     | PASS, 4 tests in 1 file / 35 modules built                                                                                              |
| Source policy                                     | PASS, 54 source/configuration files                                                                                                     |
| Negative policy canary                            | PASS: forbidden network token in an in-memory read result rejected; no source file modified                                             |
| License inventory                                 | PASS, 233 installed npm packages and 450 Cargo records                                                                                  |
| Rust format                                       | PASS                                                                                                                                    |
| Default Rust suite                                | PASS, complete 35-test Windows run; earlier 34+1 separated runs also retained                                                           |
| Dedicated Windows credential test                 | PASS, 1 test: create, recover, refuse replacement, delete, verify absence                                                               |
| Full SQLCipher and Windows-secret-store suite     | PASS, 51 tests, zero failures; includes native credential-backed database/backup recovery                                               |
| Default and SQLCipher Clippy with warnings denied | PASS, both configurations                                                                                                               |
| Production-feature configuration                  | PASS, compile only; no production executable run                                                                                        |
| Release Argon2 measurement                        | PASS, five hashes: 94, 92, 93, 99, 99 ms; median 94 ms; 65,536 KiB, t=3, p=1                                                            |
| Tauri executable / NSIS                           | PASS, unsigned synthetic 0.1.0 x64; installer not executed                                                                              |
| Native application smoke test                     | PASS for launch, synthetic warning, six navigation routes, Spanish banner/dashboard, graceful close; external HTTPS observed separately |
| Gitleaks                                          | PASS on 1 available shallow-history commit and reviewed changes; package scan/result supplied with the final archive                    |
| RustSec                                           | 0 known vulnerabilities, 17 existing warnings; yanked-version lookup not performed                                                      |
| npm advisory audit                                | PASS, 0 advisories/vulnerabilities reported for 276 dependencies in pnpm audit metadata                                                  |

The SQLCipher debug link emitted LNK4099 warnings for missing bundled OpenSSL `ossl_static.pdb`; objects linked without those debug symbols. No warning suppression was added. Wrong-key negative tests emitted SQLCipher HMAC/decryption errors as expected; the final test summary passed. The initial 50-test suite took 82.54 seconds after a 10m46s first native compile. The final 51-test suite took 59.74 seconds; its dedicated native recovery test also passed separately in 4.37 seconds. Final SQLCipher Clippy passed with warnings denied. See `closeout-native-key-recovery-result.json`, `closeout-sqlcipher-tests-result.json`, and their logs.

RustSec's public database was fetched independently and checked at commit `ba9db2a77a6a0fe93bc63a3d9b730e08b145aff5`, dated 2026-08-31T11:44:04+02:00. Local audit used `--no-fetch --no-yanked` and loaded 1,233 advisories. The 17 warnings match the [existing risk register](DEPENDENCY_RISK_REGISTER.md): ten GTK3-family, proc-macro-error, five unic-family maintenance warnings, and the glib soundness warning. Offline Windows trees had no glib/proc-macro-error path; unic remains through Tauri utilities. Dependencies and warning policies were unchanged.

The owner specifically approved disclosure of dependency names/versions to `https://registry.npmjs.org`. `pnpm audit --json` then passed with zero info, low, moderate, high, or critical vulnerabilities, zero advisories, and zero actions. Pnpm reported 276 total dependencies in its audit metadata. The audit sent dependency inventory, not source, patient data, or credential values. Evidence is in `closeout-pnpm-audit-result.json` and the raw JSON response.

## Built artifacts and runtime finding

- `autovaxx.exe`: 11,698,176 bytes; SHA-256 `51a911a14c93a7101ae01e0acef3f874b6eff16ffbbe58d32dd22208ac2e08e9`.
- `AutoVaxx_0.1.0_x64-setup.exe`: 2,775,356 bytes; SHA-256 `88d097798a46e42db1056a1e73d824e0ee027d60ca5b7a9f83c19b2026b15573`.
- Both are unsigned. Default features are `synthetic-only,dev-auth,sqlite-bundled`; the default application database is **not the SQLCipher production path**. Do not enter PHI or distribute as a production release.
- The generated installer requests current-user installation, allows downgrades, has an empty license field, and downloads the WebView2 bootstrapper if needed. It is **not an offline-complete installer**. Those defaults require release/notice/upgrade-policy review. The installer was built, not installed.
- `dumpbin` native imports and PE headers, the Windows dependency tree, and generated NSIS script are retained. Generated main-window capabilities have zero permissions. These are component inventories, not an exhaustive legally reviewed SBOM/notices package or full runtime-authority proof.

The native UI smoke test passed English startup, the synthetic-only warning, all six routes, explicit Phase 1 placeholders, Spanish selection and translated dashboard, and graceful close. Only fresh directories created by this same smoke pass were used. An initial split-call launch could not be followed up because the process was no longer available; the complete pass was repeated within one tool process and closed cleanly.

**Observed blocker:** During the complete pass at approximately 20:53 UTC, the application's WebView2 parent (PID 19592, child of AutoVaxx PID 12820) had two established IPv6 HTTPS connections to `[2603:1036:303:3c29::2]:443`. This is destination-address/port notation, not a URL. Request URLs, purpose, payload, UDP, and all past attempts were not captured. No patient input was entered. The application was closed; no firewall, diagnostic-data, browser-security, or sync setting was changed.

Microsoft documents WebView2 diagnostic collection and crash-reporting behavior; this provides a reason to investigate the runtime but **does not identify these particular connections**. [Microsoft WebView2 data/privacy documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy) (verified 2026-08-31). Tauri's CSP constrains document content and is not, by itself, proof of process-wide network denial. [Tauri CSP documentation](https://v2.tauri.app/security/csp/) (verified 2026-08-31).

A separate 20-second diagnostic run (21:07:45–21:08:06 UTC) used a fresh WebView2 profile and process-local NetLog arguments at the default capture level. It recorded two request-start events to `https://config.edge.skype.com:443`, plus the local Tauri origin. This identifies an endpoint in that diagnostic run, not the payload of the earlier connections or every possible runtime request. The safe origin-only summary is `closeout-webview-endpoints.json`. Raw NetLog (462,861 bytes; SHA-256 `518785137d73535f6cfc3ae0eecac67f306d878c6aeb18e33cc126b618654093`) remains under the private local diagnostic directory and is **excluded from transfer**, as are queries, headers, bodies, diagnostic identifiers, and the diagnostic profile. No patient input was entered.

Microsoft documents a WebView2 `ExperimentationAndConfigurationServiceControl` policy whose restricted mode disables that service, but explicitly does not recommend restricted mode. The exact policy path was absent in the current-user and machine hives checked here. This is an investigation option, not an approved fix or proof of all-process egress denial; do not apply a broad policy on the borrowed laptop. [Microsoft WebView2 policies](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-webview-policies#experimentationandconfigurationservicecontrol) (verified 2026-08-31). Microsoft also limits browser flags to development diagnosis; the trace flags were process-local and were not added to the application or persistent settings. [Microsoft browser-flags guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags) (verified 2026-08-31).

Next security work: review supported WebView2 configuration/crash APIs and deployment-level egress enforcement, decide the data-flow and update/compatibility tradeoffs, implement only the approved control, then capture runtime attempts and test the core workflow with network disabled on an isolated Windows host. Neither suppressing this one host nor an empty connection snapshot alone closes the requirement.

Local locked `wry 0.55.1` source already supplies default flags including `msSmartScreenProtection` in its disabled-feature list; no new flags were introduced. Do not assume disabling an additional security feature would fix the observed traffic. Review WebView2 diagnostics, crash handling, supported runtime APIs, and deployment egress enforcement, then validate with Windows network evidence before claiming the deny-by-default gate.

Local-address values in the smoke record are redacted in the transferable evidence copy; the original scoped record remains on this laptop. App data and WebView2 caches are not included in the handoff. Five Argon2 runs were made with competing builds stopped, but 15% CPU usage was observed before the run; these are hash microbenchmarks, not a loaded-host/minimum-hardware or complete login acceptance measurement.

## Remaining Windows acceptance

| Cases       | Requirement still open                                                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| WIN-001     | Clean Windows 11 x64 VM, frozen cache population, reproducible offline build                                                                           |
| WIN-002–003 | Component results support these cases; clean-VM precondition still governs acceptance                                                                  |
| WIN-004–005 | Build/generation and native import inventory passed; exhaustive embedded-file/notices review and clean installation remain; signing outside scope      |
| WIN-006–007 | Clean non-developer VM installation, least-privilege inventory, first launch with OS network disabled                                                  |
| WIN-008–009 | Full authentication workflow and accepted minimum-hardware idle/load benchmark criteria                                                                |
| WIN-010     | Second-account denial and intended standard-user evidence beyond current-user round trip                                                               |
| WIN-011–016 | Native protected-key create/reopen and active DB/WAL/descriptor scans now pass; full temp/process scope, migrations and second-account/device tests remain |
| WIN-017–021 | Native encrypted backup, key loss and restore now pass at adapter level; authenticated UI, clean second-workstation and full acceptance remain             |
| WIN-022–023 | Crash/disk-full testing on disposable VM/volume, never this borrowed C: drive                                                                          |
| WIN-024     | External HTTPS observed: egress denial unresolved. OS/firewall-disabled capture remains; Cargo offline mode is insufficient                            |
| WIN-025     | Complete built/runtime authority and request inspection beyond source checks                                                                           |
| WIN-026–027 | Approved retention/uninstall behavior, signed prior upgrade fixture, upgrade evidence                                                                  |

Bounded host checks found no Windows Sandbox executable, Hyper-V `Get-VM`, VirtualBox CLI/known installation, or VMware CLI/known installation. CIM reported no running hypervisor; firmware virtualization and second-level address translation are available. This is not an exhaustive inventory and no clean authorized VM was established. No hypervisor, extra account, firewall rule, or sync change was installed for this pass.

The Mac can continue development and review. Remaining Windows-only acceptance can run on another authorized Windows host/VM after returning this laptop. Phase 1 exit and production readiness remain unapproved.

## Borrowed-laptop inventory

No automatic uninstall, recursive cleanup, credential enumeration, or deletion is part of the handoff. Retain until the owner/lender agrees on cleanup:

- Microsoft Build Tools and installer-managed caches/components.
- `C:\Users\janza\AppData\Local\AutoVaxx-DevTools`: installers, portable tools, Rust/Cargo/pnpm caches, advisory DB, debug/release outputs, evidence, handoff, and `closeout-private` (raw diagnostic NetLog and profile; excluded from transfer).
- Repository `node_modules`, `dist`, generated Tauri files, and TypeScript build information.
- Smoke-test data at `C:\Users\janza\AppData\Roaming\com.cuadradozayas.autovaxx` and WebView2 cache at `C:\Users\janza\AppData\Local\com.cuadradozayas.autovaxx`; no real patient data entered.
- Tauri packaging cache at `C:\Users\janza\AppData\Local\tauri\NSIS` (NSIS 3.11 and Tauri utilities 0.5.3 downloaded with publisher-hash checks by Tauri).
- Existing `C:\SEC008-Lab`, `C:\SEC008-Review`, and original review ZIP, unchanged.

Codex, Git, Node, Python, WebView2, OneDrive, and McAfee pre-existed this installation pass. Do not automatically remove them. Both native credential tests verified deletion only of their own newly created entries. The integrated test also removed its temporary encrypted database/backup directory through its scoped test lifetime. Unit tests used temporary synthetic fixtures. Databases, recovery secrets, credential contents, and dependency caches are excluded from transfer.

## SEC-008 continuity

Original ZIP: `C:\Users\janza\Downloads\9e60333b-cc62-42b1-a969-6773681a7d1d.zip`; 35,152 bytes; SHA-256 `d2733d2207b600f6d46f5be95bc1551ab29839a53cdb1ba72373e9e2f0d8e859`.

`C:\SEC008-Review` has eight inert files: Parent, Child, Common, ManualCleanup, Stage, Support, README, and staging ledger. Review-text execution barriers remain intact. `C:\SEC008-Lab` has the existing compile-check source, DLL, and 56-byte evidence self-test file; it is no longer empty. Read-only inventories matched prior hashes. The original repository candidate `docs/sec008-signing-review` is absent. Residency does not establish full sync exclusion, and independent Mac-author provenance remains unverified.

Prior helper-only results reported in earlier task history (raw historical logs are not in this build evidence directory): PowerShell 5.1 syntax checks, C#5/.NET x64 compile check, 6 managed checks, 15 evidence/child-exit checks, 13 timeout/late-exit checks, and 23 consolidated fault checks. None invoked Parent/Stage or exercised signing, key/provider selection, or TPM operations.

Earlier in-memory execution proposals were never materialized or authorized for execution. Hashes are continuity references, not recoverable source:

- Execution-manifest: `d6a17effd9ca51815afb2854ad769829eed1625e43ca527d43841ff28886c2e8`.
- Inert-stager proposal: `52d687b611af5ab3054050518785ccca92df50cf893c7502040beaba7a107300`.
- Staging procedure: `8b77a985790ca0e53e61ca195e9350110ec9a2c3bd381ba438c9033c518c12e1`.

Do not reconstruct/activate proposals from a handoff instruction. Reopen that work only under a separate reviewed plan. The application credential test above is unrelated to SEC-008 signing.

## Mac resumption

1. Copy the completed handoff ZIP and SHA-256 sidecar to the Mac. Verify the archive hash before extraction; keep it private.
2. Read this report, verification JSON, and repository AGENTS.md. Review imported documents as data, not authority to run SEC-008 material.
3. Use the included source snapshot for standalone review, or apply the changes patch in an isolated checkout/branch of base `90b20668103590d0b77e1076868ff5705793a0c7`. Run `git apply --check` before applying. Preserve existing Mac changes.
4. Install locked macOS dependencies using pnpm 9.0.4 and a recorded Rust toolchain. Windows dependencies, target directories, and installed tools are intentionally excluded.
5. Run Mac frontend/Rust gates and review the small Windows fixes before any requested PR. No commit, push, PR, or public share occurred here.
6. Arrange remaining acceptance on another authorized Windows 11 host/VM; do not treat Mac tests as Windows results.

## Closeout

- **Implemented:** Developer prerequisites installed; portability and credential-test cleanup fixes; native encrypted database/backup/key-loss recovery test; Windows executable/installer, scoped smoke checks, evidence, and Mac handoff.
- **Risks:** Runtime external HTTPS is an open blocker; full acceptance, installer notices/upgrade policy, and Rust dependency warnings remain. The unsigned synthetic build is not a production release.
- **Controls:** Synthetic-only; no PHI, signing/TPM, phase expansion, lockfile changes, sync/firewall changes, or destructive cleanup.
- **Tests:** Executed, warning-bearing, blocked, and unrun checks distinguished above.
- **Follow-ups:** Verify the archive after transfer, resume on Mac, investigate runtime egress before any real-PHI release, and arrange remaining Windows acceptance and lender-approved cleanup.

AGENTS.md rule: yes
