# Architecture

Status key: **[implemented]** = source present + host-testable (tests are still
*to run* unless recorded otherwise); **[planned]** = designed, not implemented;
**[unrun]** = no verification has been executed.

## 1. Crate graph

```
                     Kotlin / Compose + Miuix shell (single Activity)
                                    │  JNI (versioned command bus)
                                    ▼
                                pdx-jni  [in progress]
                              (bus dispatcher / envelopes)
                                    │
       ┌────────────────────────────┼──────────────────────────────┐
       ▼            ▼               ▼               ▼              ▼
   pdx-core     pdx-apk        pdx-term        pdx-cli     (future engines)
  [implemented]  [in prog]     [in prog]      [in prog]     [planned]
   typed errors  ZIP central    PTY + VT       info/apk      UnityFS, IL2CPP,
   ByteReader    dir + CRC      parser +       list/verify   DEX, AXML, ARSC,
   SHA-256       verify         cell grid      --json        ELF, hex, diff...
   workspace                                  typed exits
   job/cancel
```

Rules:

- Every parser crate depends on `pdx-core` **only** for the error type, hashing,
  byte reader, and job model. `pdx-core` has **zero Android dependencies** and
  builds/tests on host x86_64.
- `pdx-cli` is a host-facing thin client over the same crates: `info`,
  `apk list`, `apk verify`, `--json`, typed exit codes. It never links JNI.
- `pdx-jni` is the **only** bridge the Android shell uses; no other crate speaks
  JNI. Planned engines attach as new `pdx-*` crates that implement the same
  command interface — the bus, not direct Kotlin calls, is the seam.
- C/C++ exists only where the NDK forces it (the PTY shim inside `pdx-term`).

## 2. JNI command bus [in progress / planned]

One versioned, asynchronous envelope in both directions. The `v` field is
checked before dispatch; an unknown version yields a typed
`UnsupportedFormatVersion` (never a guessed interpretation).

```jsonc
// Kotlin -> Rust request
{ "v": 1, "id": 42, "cmd": "workspace.open", "args": { "path": "/storage/.../x.pdxproj" } }

// Rust -> Kotlin response (success)
{ "v": 1, "id": 42, "ok": true, "result": { /* typed payload */ } }

// Rust -> Kotlin response (failure)
{ "v": 1, "id": 42, "ok": false,
  "error": { "kind": "bad_input", "message": "...", "offset": 64,
             "expected": "4 bytes", "actual": "2 bytes" } }

// Out-of-band progress / log event (same id)
{ "v": 1, "id": 42, "event": "progress",
  "progress": { "job": "job-7", "done": 3, "total": 10, "message": "reading entries" } }
```

Properties:

- `cmd` is namespaced (`workspace.*`, `documents.*`, `diagnostics.*`,
  `statusbar.*`, `settings.*`, `terminal.*`, `primex.*`) — the same namespaces
  the plugin host exposes (see §6 and [PLUGIN_API.md](PLUGIN_API.md)).
- Errors are the `PdxError` JSON shape from `pdx-core`, so CLI, JNI, and plugins
  report identical typed failures. No command returns an untyped `null` sentinel.
- Long operations return a job id immediately; progress/log events stream on the
  same `id`. Native objects are never passed across JNI as raw pointers.

## 3. Capability ladder L0–L3 [planned]

Each rung adds power; a missing prerequisite **degrades with a typed
`CapabilityFailure`** (exit code 5) instead of crashing or silently faking data.

| Level | Name | What it permits | Degradation when unavailable |
|---|---|---|---|
| **L0** | Discovery | Offline inspection of files on the host/workspace: hashing, ZIP central-dir listing, CRC verify, schema-version detection. Read-only. | Host-only; always available on a supported build. |
| **L1** | Analysis | Read-only static decoding on-device: UnityFS, IL2CPP metadata, DEX, AXML/ARSC, ELF disassembly, indexes. Never touches a live process. | Typed `CapabilityFailure` if arm64-v8a / 16 KB page-aligned native libs are unavailable or engine not built. |
| **L2** | Authoring | Writes to the workspace: hex edits, diff application, repack/sign. Produces artifacts on disk only. | Typed `CapabilityFailure` if workspace is read-only or signing material absent. |
| **L3** | Device | Device-facing utilities: install built APK, logcat/tombstone capture. **Still no attach/hook/inject into the running game.** | Typed `CapabilityFailure` if adb/root/permission unavailable. |

The ladder is stored per project as `capability_level: u8` (validated `0..=3` in
`pdx-core`). UI features are gated on the current level and show the typed reason
for a downgrade. L3 is additive, not a superset that unlocks live-game tricks.

## 4. Job model [implemented in pdx-core]

- `JobSequencer` hands out monotonic `JobId`s (`job-N`).
- `CancelToken` is a cloneable `Arc<AtomicBool>`; `check()` returns a typed
  `Cancelled` error. Workers call it at defined checkpoints.
- `Progress { job, done, total, message }` carries `fraction()` (clamped) to the
  bus for UI progress.
- `trait Job { id(); description(); run(&self, cancel: &CancelToken, report: &dyn Fn(Progress)) -> Result<()> }`.
- **Persistent log** is planned: each job appends structured entries to a
  workspace log so a run can be audited/replayed. Not implemented yet.

Cancellation is cooperative: a cancelled job finishes at its next checkpoint
with `ErrorKind::Cancelled` (exit code 6), never a half-written artifact without
a recorded reason.

## 5. Miuix navigation choice [planned]

- **Single Activity**, Jetpack Compose for the shell; Miuix supplies widgets and
  navigation so the UI stays consistent with the Android modding community look.
- **No Material 3 outside a theme bridge.** Any component that requires M3
  (e.g. an embedded editor surface) is wrapped by a small bridge that translates
  tokens; M3 is never allowed to become a second, competing design system.
- Rationale: one design language, fewer theme conflicts, smaller dependency
  surface, and a clear seam when swapping component sets.
- Miuix is pinned to `0.9.4` (`top.yukonga.miuix.kmp:miuix-ui-android:0.9.4`) and the
  version is recorded in each project document (`miuix_version`).

## 6. Plugin host design [planned]

Four trust tiers, independent of the capability ladder. Every `.pdxplug` is
**Ed25519-signed**; the host verifies the signature over manifest + payload
against its trusted key *before* loading. An invalid/missing signature is a
typed `VerificationFailed` (exit code 4) and the plugin is rejected — no code
executes.

| Tier | Form | Capability | Trust requirement |
|---|---|---|---|
| **T1** | Data-only (JSON/descriptors) | Declarative contributions; no executable code | Signature + declared permissions |
| **T2** | Lua script | Calls an allow-listed command subset | Signature + permission review |
| **T3** | WASM (WASI, preopened workspace only) | Engine-agnostic plugins, sandboxed memory | Signature + explicit user enable |
| **T4** | External server/tool (arm64, app UID) | LSP / JSON-RPC helper processes | Signature + per-use consent with binary hash |

Sandbox limits (planned): allow-listed commands per tier, workspace-scoped file
access through the VFS, no raw socket/network (offline guarantee), isolated
process for T3/T4, and no access to the running-game tooling. See
[PLUGIN_API.md](PLUGIN_API.md) for the draft surface and
[SECURITY.md](SECURITY.md) for residual risks.
