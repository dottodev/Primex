# PrimeX D.T

**PrimeX D.T** (Developer Tools) — collaboration label **`mc1h`**, package id
`dev.primex.dt` — is an **offline-first, on-device Android IDE and
reverse-engineering workbench for MLBB MODIFIED-BUILD FILE RESEARCH and
DEBUGGING ON DISK.**

It reads, analyses, edits, and re-packages **files on disk**. It is a static
workbench, not a live-game tool.

## What it is

- A local workspace for inspecting modified-build artifacts: APK/ZIP containers,
  Unity/IL2CPP data, DEX, binary XML and resources, ELF, and hex-level edits.
- A Rust core (`pdx-*`) that is host-testable with no Android dependency, behind
  a versioned JNI command bus consumed by a Kotlin + Compose + Miuix shell.
- A capability ladder (L0–L3) that degrades with typed errors when a required
  system capability is unavailable, plus a tiered, signed plugin host (T1–T4).
- Fully offline: no account, no server, no telemetry, no bundled game assets or
  keys. All project data stays in the user's workspace directory.

## What it is NOT — non-goals (hard constraints)

PrimeX D.T **NEVER attaches to, hooks, injects into, or manipulates the running
game.** Specifically, it does **not** provide:

- no aimbot, ESP, wallhack, or any live-match advantage;
- no anti-cheat detection bypass or evasion;
- no bundled game assets, decryption keys, or stolen proprietary data;
- no telemetry, analytics, crash reporting, account, login, or cloud sync;
- no network server or remote service — it works entirely on-device;
- no mechanism that reads or writes the memory of a running game process.

Anything that requires a live process attachment is out of scope by design, not
by policy-of-the-moment. The product's value is reproducible, verifiable work on
files the user already has on disk.

## Architecture at a glance

- **Rust is primary.** Kotlin + Compose + Miuix is the shell; Lua is the
  automation language; C/C++ appears only where the NDK forces it (e.g. a PTY
  shim).
- **Pinned:** tool version `0.1.0`; Miuix `0.9.4`
  (`top.yukonga.miuix.kmp:miuix-ui-android:0.9.4`; the `-android` AAR is the
  form consumed by an Android application module).
- **Platform:** `minSdk 26`, `targetSdk 35`, `compileSdk 37` (Miuix 0.9.x AARs
  declare `minCompileSdk=37`; see [docs/BUILD.md](docs/BUILD.md)),
  `arm64-v8a` required, `Apache-2.0`.
- Details: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md),
  [docs/FORMATS.md](docs/FORMATS.md), [docs/SECURITY.md](docs/SECURITY.md).

## Build and check

From the repository root:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
./gradlew assembleDebug
```

See [docs/BUILD.md](docs/BUILD.md) for exact toolchain versions, installing the
missing Android SDK platform 35, `cargo-ndk`, and reproducibility notes. See
[docs/VERIFICATION.md](docs/VERIFICATION.md) for what has and has not been run.

## Milestone status

| Milestone | Scope | Status |
|---|---|---|
| **M0** | Foundation: `pdx-core`, `pdx-apk`, `pdx-cli`, `pdx-jni`, `pdx-term` (host-testable) + Miuix shell scaffold + native bridge in APK | **PARTIAL** — host tests/clippy/build PASS; PTY→UI bridge not wired; no device run |
| M1 | Workspace shell, JNI bus wiring, Miuix navigation | NOT STARTED |
| M2 | ZIP/APK deep inspection + verification UI | NOT STARTED |
| M3 | UnityFS / IL2CPP metadata analysis | NOT STARTED |
| M4 | DEX, AXML, ARSC decoders | NOT STARTED |
| M5 | ELF disassembly + symbol/index tooling | NOT STARTED |
| M6 | Hex editor + diff + audit/triage | NOT STARTED |
| M7 | Lua automation runtime | NOT STARTED |
| M8 | Plugin host (T1–T4), Python runtime | NOT STARTED |
| M9 | Sora editor + LSP integration | NOT STARTED |
| M10 | VFS + repack/sign/install pipeline | NOT STARTED |
| M11 | logcat/tombstone + device-side utilities | NOT STARTED |

Only M0 foundation work is done. Everything from M1 onward is **planned and
not implemented**. Host checks pass (47 tests, clippy, fmt) and a debug APK
builds with the native JNI library bundled, but the terminal is a local-echo
scaffold (no PTY wired to the UI) and no device/emulator check has been run.
See [docs/VERIFICATION.md](docs/VERIFICATION.md) for the recorded results.

## License

Apache License 2.0 — see [LICENSE](LICENSE). Third-party components and their
verification status are recorded in [THIRD_PARTY.md](THIRD_PARTY.md).
