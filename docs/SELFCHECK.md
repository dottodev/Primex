# Self-check

Each question below is answered against **current evidence only**. "NOT YET —
unrun" is the correct answer for anything that requires a build, a test run, or a
device. The status reflects the M0-in-progress state; it is not a promise that a
feature works.

| # | Question | Answer | Evidence / status |
|---|---|---|---|
| S1 | Does PrimeX D.T attach to, hook, inject into, or manipulate a running game? | **NO — by design** | Stated non-goal; architecture has no live-process component. Device confirmation D9 is **UNRUN**. |
| S2 | Is there any aimbot/ESP/live-match advantage or anti-cheat bypass? | **NO — by design** | Explicit non-goal in README and ARCHITECTURE. No code path or asset implements one. |
| S3 | Are game assets, keys, or proprietary binaries bundled? | **NO — by design (unverified in artifact)** | No such files in repo. Artifact check D10 is **UNRUN**. |
| S4 | Is the app fully offline (no account, server, telemetry)? | **Designed offline — UNRUN** | No network code in present sources. Device check D2 **UNRUN**. |
| S5 | Do parsers fail closed with typed errors and byte offsets? | **YES for implemented crates — host verified** | `PdxError` + `ByteReader` + typed CLI exits; tests H6/H7/H8 PASS; ZIP CRC mismatch returns `VerificationFailed` with offset. |
| S6 | Are workspace documents schema-versioned with tested migration? | **YES — host verified** | `ProjectJson`/`ProfileJson`, v0→v1 migration; `workspace::tests::v0_document_migrates` and future-version test PASS. |
| S7 | Is `capability_level` bounded to 0..=3? | **PARTIAL — source present** | `ProjectJson::validate` rejects >3 with `BadInput`; runtime degradation UI **NOT IMPLEMENTED**. |
| S8 | Does the job model support cancel + progress + persistent log? | **PARTIAL** | `CancelToken`, `Progress`, `Job`, `JobSequencer` implemented in `pdx-core`; **persistent log NOT IMPLEMENTED**; device cancel D6 **UNRUN**. |
| S9 | Is the JNI command bus versioned and typed end-to-end? | **PARTIAL — host verified, device unrun** | `pdx-jni` bus: 10 host tests PASS; export `Java_..._nativeSend` present in both shipped `.so` files (llvm-nm). Kotlin→Rust call not exercised on a device (D3 UNRUN). |
| S10 | Is Miuix navigation single-Activity with no Material 3 outside a theme bridge? | **YES for the M0 scaffold — source and build verified** | Single `MainActivity`, `MiuixTheme` from `top.yukonga.miuix.kmp.theme`, zero `material3` imports; `assembleDebug` PASS. Full IDE navigation not implemented. |
| S11 | Are format engines verified against real fixtures? | **PARTIAL — ZIP/APK verified on host** | `fixtures/minimal.apk`, empty/duplicate/case-collision fixtures; 8 pdx-apk tests PASS. All other format engines not implemented. |
| S12 | Are all M1–M11 features implemented? | **NOT YET — not started** | Milestone table in README; every M1–M11 item is planned. |
| S13 | Are native libs arm64-v8a and 16 KB page-aligned? | **NOT YET — unrun** | Host NDK 25/27 present; device check D3 **UNRUN**; link flags **to be verified**. |
| S14 | Do `cargo test --workspace` / `cargo clippy --workspace -- -D warnings` pass? | **YES — host verified 2026-11-19** | 47 tests PASS, 0 failed; clippy clean with `-D warnings`; fmt check PASS. |
| S15 | Does `./gradlew assembleDebug` succeed? | **YES — host verified 2026-11-19** | `BUILD SUCCESSFUL in 1m 21s`; APK contains `libpdx_jni.so` for arm64-v8a and x86_64; `aapt2` confirms label/package/SDKs. Note: compileSdk 37 required by Miuix 0.9.4 (deviation documented in BUILD.md). |
| S16 | Are plugin trust (Ed25519) and tier sandbox limits enforced? | **NOT YET — not implemented** | Design in PLUGIN_API.md/SECURITY.md; rejection test D11 **UNRUN**. |
| S17 | Is the Python runtime sandboxed with package tiers? | **NOT YET — not implemented** | Plan in PYTHON.md; D12 **UNRUN**. |
| S18 | Are all third-party licenses verified? | **NOT YET** | THIRD_PARTY.md lists everything as "verify"/"pending"; no audit done. |

## How to close a check

1. Run the exact command/steps referenced in [VERIFICATION.md](VERIFICATION.md).
2. Paste the output and the date into that file.
3. Update the corresponding row here from "NOT YET" to the recorded result.
4. Never mark a row PASS on the basis of source reading alone.
