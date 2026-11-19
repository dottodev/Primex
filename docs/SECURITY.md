# Security model

## Offline guarantee

- PrimeX D.T has **no account, no server, no telemetry, and no network
  dependency**. Every feature operates on local files or device-provided logs.
- No component may open an outbound socket. Capability checks enforce this at
  the command layer; plugins and the Python runtime have **no network
  permission** at any tier ([PLUGIN_API.md](PLUGIN_API.md),
  [PYTHON.md](PYTHON.md)).
- No game assets, keys, or proprietary binaries are bundled.
- The product **never attaches to, hooks, injects into, or manipulates the
  running game** — there is no `ptrace`, no `/proc/<pid>/mem`, no live-process
  component. This is an architectural constraint, not a runtime setting.
- Verification: device check D2 (offline) and D9 (no attachment) are **UNRUN**.

## W^X helper rule

- Native helper memory is **W^X**: a page is never writable and executable at the
  same time. No JIT, no self-modifying code, no `mmap(PROT_WRITE | PROT_EXEC)`.
- Lua automation and any scripting use interpreters, not generated machine code.
- `pdx-core` parsing is pure bounds-checked reads via `ByteReader`; it never
  casts input bytes to executable code.
- Native libraries are loaded from the app's own `jniLibs` only, and must be
  16 KB page-aligned and `arm64-v8a`.

## Plugin sandbox limits (planned, not implemented)

- Every `.pdxplug` is **Ed25519-signed**; signature covers manifest + payload and
  is verified before any code loads. Invalid/missing signature →
  `VerificationFailed`, no execution.
- Permissions are declared in the signed manifest and enforced as allow-lists;
  a call outside them is a typed `CapabilityFailure`.
- Filesystem access is workspace-scoped through the VFS; T3/T4 run in isolated
  processes; T4 requires manual install and per-use consent.
- No network permission at any tier; no access to live-game tooling at any tier.
- Trusted-key distribution and revocation are still open design decisions.

## Typed, fail-closed handling

- All parsers fail closed: `UnsupportedFormatVersion`, `BadInput` (with offset,
  expected, actual), and `VerificationFailed` are the normal failure modes —
  never guessed values or zero-filled structures.
- Jobs are cooperatively cancellable with typed `Cancelled`; progress and a
  persistent log make every operation auditable.

## Residual risks

- **Native code outside Rust** (PTY shim, Python T3 extensions, T3/T4 plugins)
  is not covered by the typed-error model and can crash the process. Mitigation:
  process isolation; crashes logged, not hidden.
- **Unverified third-party licenses** — every entry in
  [THIRD_PARTY.md](../THIRD_PARTY.md) is "verify"/"pending"; this is a
  release-blocking audit item.
- **Unimplemented device checks** — D1–D15 are all **UNRUN**; the offline and
  no-attachment guarantees are by design until verified on hardware.
- **Platform 35 not installed** and **16 KB alignment not verified**; see
  [BUILD.md](BUILD.md).
- **Python debugpy/native-extension risk** — documented in [PYTHON.md](PYTHON.md).
- **Trusted-key revocation** for plugins is unresolved.

Security claims in this document are design claims. Do not treat them as
verified until the corresponding rows in [VERIFICATION.md](VERIFICATION.md) and
[SELFCHECK.md](SELFCHECK.md) are updated with recorded evidence.
