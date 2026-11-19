<div align="center">

<img src="assets/primex-logo.png" alt="PrimeX D.T logo" width="180" />

# PrimeX D.T

**Developer Tools · offline-first Android IDE & reverse-engineering workbench**

<p>
<img src="https://img.shields.io/badge/label-mc1h-7C3AED?style=for-the-badge" alt="mc1h" />
<img src="https://img.shields.io/badge/package-dev.primex.dt-232F3E?style=for-the-badge" alt="package" />
<img src="https://img.shields.io/badge/Android-minSdk%2026-3DDC84?style=for-the-badge" alt="minSdk" />
<img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="license" />
</p>

<p>
<img src="https://img.shields.io/badge/offline-always-1ABC9C?style=flat-square" alt="offline" />
<img src="https://img.shields.io/badge/root-not%20required-E67E22?style=flat-square" alt="no root" />
<img src="https://img.shields.io/badge/telemetry-none-E74C3C?style=flat-square" alt="no telemetry" />
<img src="https://img.shields.io/badge/account-none-E74C3C?style=flat-square" alt="no account" />
<img src="https://img.shields.io/badge/live--game%20access-never-8E44AD?style=flat-square" alt="no live game" />
</p>

<sub>Made for people who ship modified builds and need to know **why they break**.</sub>

</div>

---

## 🧠 What it is

PrimeX D.T is a **static workbench for files on disk**. You bring a modified
build — edited manifest, patched `.so`, swapped AssetBundle assets, rewritten
XML/JSON/Lua — and the tool tells you what changed, what fails to parse, and
where the bytes went wrong.

It is an IDE in the sense that it gives you a workspace, an editor surface, a
terminal, a hex viewer, a file tree and a command palette. It is a reverse
engineering workbench in the sense that it understands APK/ZIP, binary XML and
resources, DEX, ELF, UnityFS and IL2CPP metadata — and says **"unsupported"**
with an offset when it does not.

> 🧩 **The headline loop:** stock build vs. mod build → per-entry audit →
> ranked load-failure hypotheses, each citing bytes, offsets and parse errors.
> Confirmed means evidence-backed. Suspected means ranked, unverified. Nothing
> is guessed.

<p align="center">
<img src="assets/primex-icon.png" alt="PrimeX app icon" width="96" />
</p>

---

## 🚫 What it is not

These are not missing features. They are design boundaries.

| 🚫 Sticker | Meaning |
|---|---|
| 🎯 **No live-match advantage** | No aimbot, ESP, wallhack, map view, damage or cooldown edits. |
| 🛡️ **No anti-cheat work** | No signature-check bypass, no evasion, no unban. |
| 🔌 **No process attachment** | Never hooks, injects, reads or writes `com.mobile.legends`. |
| 📦 **No bundled game data** | No game assets, decryption keys or proprietary files ship with the app. |
| ☁️ **No service** | No account, no login, no cloud, no license server, no ads, no telemetry. |
| 🌐 **No required network** | Airplane mode is the default and the product works forever that way. |

---

## 🧭 How it fits together

| Layer | What it does |
|---|---|
| 🦀 **Rust core** | Parsers, hashing, workspace, LSP client, plugin host, Python bridge, terminal engine and the job system. Builds and unit-tests on the host with zero Android dependencies. |
| 🎨 **Kotlin + Compose + Miuix** | Lifecycle, permissions, chrome and the editor surface. Renders Rust state; never parses a format itself. |
| 🌙 **Lua** | Automation layer with a sandboxed API and per-script timeout. |
| ⚙️ **C / C++** | Only where the NDK forces it. |

Everything crosses one versioned JNI command bus: request id + command + JSON or
direct buffer, callback with status + JSON + optional buffer. Long work runs as
a cancellable job with progress and a persistent log — never on the UI thread.

### 🔑 Capability ladder

| | Level | What it unlocks |
|---|---|---|
| 🟢 | **L0** | App sandbox — read-only inspection, hashing, container listing. |
| 🔵 | **L1** | MANAGE_EXTERNAL_STORAGE — browse user-supplied files. |
| 🟠 | **L2** | Shizuku shell UID — device utilities without root. |
| 🔴 | **L3** | Root — optional capability upgrade. |

A missing prerequisite degrades with a typed message and a path to the right
settings screen. No silent elevation. Every privileged action is audit-logged.

---

## 🧪 Format coverage

| Engine | Current state |
|---|---|
| 📁 Workspace, file manager, SHA-256 | ✅ implemented |
| 🗜️ ZIP / APK / OBB container, CRC, ZIP64 | ✅ reader + verification implemented |
| 📝 Bounds-checked byte reader, typed errors | ✅ implemented |
| 🧵 PTY + VT100 parser + cell grid | ✅ core implemented, UI bridge pending |
| 📦 AXML / ARSC, DEX, ELF, UnityFS, IL2CPP metadata | 🗓️ planned |
| 🔍 Hex editor, diff, audit, triage, provenance | 🗓️ planned |
| 🐍 Python runtime, LSP, editor, plugins, VFS | 🗓️ planned |

> ⚠️ A planned format is never pretended. An unknown version returns
> `UnsupportedFormatVersion` with the detected version and the tested range.
> No zero, no guessed offset, no fabricated key.

---

## 🗺️ Milestone map

| | Milestone | Focus | Status |
|---|---|---|---|
| ✅ | **M0** | Foundation: parser core, APK reader, CLI, JNI bridge, Miuix shell, PTY core | Partial — host-verified; PTY→UI bridge pending |
| ⬜ | M1 | IDE shell: editor, explorer, palette, settings, VFS | Planned |
| ⬜ | M2 | Workspace, hashing, hex editor, snapshots | Planned |
| ⬜ | M3 | APK inspector, AXML/ARSC decode-encode, entry diff | Planned |
| ⬜ | M4 | UnityFS parse, TextAsset extract, repack | Planned |
| ⬜ | M5 | ELF disassembly, IL2CPP metadata, dump.cs | Planned |
| ⬜ | M6 | Audit, triage, native patch diagnostics, lint | Planned |
| ⬜ | M7 | Python runtime, pylsp, LSP client | Planned |
| ⬜ | M8 | Signed plugin host (Lua / WASM / server tiers) | Planned |
| ⬜ | M9 | Lua inspector, sign/align/install, patch manifests | Planned |
| ⬜ | M10 | Lua automation, CLI parity, reports, provenance | Planned |
| ⬜ | M11 | Hardening, fuzz, performance, release | Planned |

A milestone starts only after the previous one meets its acceptance criteria.

---

## 🧾 Verification snapshot

| Check | Result |
|---|---|
| Workspace unit tests | ✅ 47 passed, 0 failed |
| Clippy / format | ✅ clean |
| Debug APK build | ✅ builds, native bridge bundled |
| Device / emulator run | ⏳ **unrun** — no target available |

Full evidence, including the exact commands and every unrun device check, lives
in [`docs/VERIFICATION.md`](docs/VERIFICATION.md). Architecture is described in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), format confidence in
[`docs/FORMATS.md`](docs/FORMATS.md), and build details in
[`docs/BUILD.md`](docs/BUILD.md).

---

## 📄 License

Apache License 2.0. See [LICENSE](LICENSE) and
[THIRD_PARTY.md](THIRD_PARTY.md).

<sub>collaboration label **mc1h** · built for kindmops</sub>
