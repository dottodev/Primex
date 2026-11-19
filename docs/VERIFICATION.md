# Verification ledger

This is the single place that records what has actually been run. Date: **2026-11-19**.
"PASS" means the command was executed in this session and the observed result is
noted; "UNRUN" means it was not executed. A missing future-milestone check is
expected and is not a failure of the current milestone.

## Host checks — results

| # | Check | Exact command | Status / observed |
|---|---|---|---|
| H1 | All workspace unit tests | `cargo test --workspace` | **PASS** — 47 passed, 0 failed (8 pdx-apk, 18 pdx-core, 10 pdx-jni, 11 pdx-term) |
| H2 | `pdx-core` tests only | `cargo test -p pdx-core --lib` | **PASS** — 18/18 (after fixing the test vector, not the reader) |
| H3 | Clippy, warnings as errors | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** — no warnings |
| H4 | Format check | `cargo fmt --all -- --check` | **PASS** — after applying `cargo fmt --all` |
| H5 | `pdx-cli` JSON + list + verify | `cargo run -p pdx-cli -- info --json`; `… apk list fixtures/minimal.apk`; `… apk verify fixtures/minimal.apk --json` | **PASS** — `{"verified_entries":5,"status":"ok"}`; 5 entries listed; tool version 0.1.0 |
| H6 | Typed exit code on CRC mismatch | unit test `mutated_crc_is_detected` | **PASS** — returns `ErrorKind::VerificationFailed` (exit 4) with offset |
| H7 | Unknown schema version | `workspace::tests::future_version_is_typed` | **PASS** — `UnsupportedFormatVersion` (exit 3) |
| H8 | Truncated read error | `reader::tests::truncated_read_is_typed` | **PASS** — offset 0x1000, expected 4 bytes, actual 2 bytes |
| H9 | SHA-256 known vector | `hash::tests::known_vector` | **PASS** — `ba7816bf…20015ad` |
| H10 | Cancel token stops a job | `job::tests::cancel_token_stops_job` | **PASS** |
| H11 | Duplicate / case-colliding ZIP names | `pdx-apk` fixture tests | **PASS** — both rejected with typed `BadInput` + offset |
| H12 | Gradle debug build | `./gradlew :app:assembleDebug` | **PASS** — `app-debug.apk`, 11,680,062 bytes, `BUILD SUCCESSFUL in 1m 21s` |
| H13 | Android NDK native bridge | `cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build --release -p pdx-jni` | **PASS** — both `.so` produced; `llvm-nm -D` shows `Java_dev_primex_dt_jni_CommandBus_nativeSend` |
| H14 | APK manifest + packaged libs | `aapt2 dump badging` + Python zip listing | **PASS** — `dev.primex.dt`, minSdk 26, targetSdk 35, label `PrimeX D.T`, `lib/{arm64-v8a,x86_64}/libpdx_jni.so` present |
| H15 | Locked workspace tests | `cargo test --locked --workspace` | **PASS** — all 9 test binaries report `test result: ok` |
| H16 | Unix PTY compiles for Android | `cargo ndk -t arm64-v8a check -p pdx-term` (NDK 27) | **PASS** — after declaring bionic `environ` explicitly (the `libc` crate omits it on Android). Not run on a device. |
| H17 | No Material 3 in UI code | source search for `androidx.compose.material3` under `app/src` | **PASS** — zero matches |

Note on H12: the checkout is under OneDrive; Gradle once failed with
`AccessDeniedException` inside `app/build`. Removing `app/build` and rebuilding
succeeded. See [BUILD.md](BUILD.md).

## Device / emulator checks — ALL UNRUN

No device or emulator was available in this session. Prerequisites: an
`arm64-v8a` device/emulator, `adb devices` showing one target, airplane mode for
offline checks.

| # | Check | Exact steps | Status |
|---|---|---|---|
| D1 | Install + launch debug APK | `adb install -r app/build/outputs/apk/debug/app-debug.apk`; launch `dev.primex.dt`; confirm UI renders | UNRUN |
| D2 | Offline guarantee | enable airplane mode; use all implemented flows; `dumpsys package dev.primex.dt \| grep -i INTERNET` (expect none) | UNRUN |
| D3 | Native bridge on device | confirm `apiVersion` in the status bar is `1`, not `unavailable`; confirm JNI round trip | UNRUN |
| D4 | Terminal PTY on device | type `echo hi`, `ls`, `pwd`; rotate; type ~10k lines; confirm 60 fps scrollback | UNRUN — **PTY not yet wired to the UI** |
| D5 | 16 KB page alignment | `readelf -l` on `libpdx_jni.so` for 16 KB `LOAD` alignment | UNRUN — not yet enforced in the link step |
| D6 | Capability degradation L0→L3 | withhold prerequisites; confirm typed `CapabilityFailure` and graceful UI | UNRUN — ladder not implemented |
| D7 | Workspace persistence | create `.pdxproj/project.json`, close/reopen app, confirm state survives | UNRUN — UI not implemented |
| D8 | APK inspection on device | load fixture APK; list entries; verify CRC; compare to host `pdx-cli` | UNRUN — UI not implemented |
| D9 | Read-only guarantee | confirm no `ptrace`/`/proc/<pid>/mem` access, no running-game process touched | UNRUN |
| D10 | No bundled game data | `unzip -l app-debug.apk`; confirm no game assets, keys or proprietary files | UNRUN |
| D11–D15 | Plugins, Python, repack/sign/install, logcat, Sora/LSP | as listed in the original specification | UNRUN — features not implemented |

## M0 acceptance criteria not yet met

- AC1 "reaches a usable IDE in ≤1.5 s": APK builds; cold start on device **unrun**.
- AC2 "terminal runs sh/echo/ls/pwd, 10k lines at 60 fps": the Rust PTY/VT/grid
  core is host-tested, but the Kotlin terminal screen is a local-echo scaffold;
  the PTY bridge to the UI is **not implemented**.
- AC3–AC33: not implemented; see [SELFCHECK.md](SELFCHECK.md).

## Rules

- One row = one recorded result. Paste command output, device model/Android
  version, and date when updating.
- Never mark a device check PASS without running it on a real target.
