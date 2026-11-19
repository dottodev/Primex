# Python runtime (plan)

**Status: NOT IMPLEMENTED.** This is a design note for a planned M8 feature.
Nothing described here exists or has been run.

## Embedding approach — Chaquopy (planned)

- Python will be embedded on Android via **Chaquopy**, which provides a
  CPython interpreter packaged for Android and a Gradle plugin. This keeps the
  interpreter local and offline, consistent with the no-network guarantee.
- The Python runtime is exposed to the rest of the app **only** through the
  versioned JNI/plugin command bus (namespaces in
  [PLUGIN_API.md](PLUGIN_API.md)), never through a raw interpreter handle in the
  Kotlin shell.
- Package availability is restricted by **tiers**; there is no unrestricted
  `pip install` at runtime.

## Package tiers T1 / T2 / T3 (planned)

| Tier | Packages | Capabilities | Notes |
|---|---|---|---|
| **T1** | Pure-Python stdlib subset (no native extensions, no sockets) | Data transformations, JSON/text helpers | Default; runs in the lowest-isolation slot. |
| **T2** | Pre-vetted pure-Python wheels bundled at build time | Parsing helpers, scripting | Allow-list curated and signed; no network fetch at runtime. |
| **T3** | Wheels with native extensions (e.g. numpy-like) | Compute-heavy analysis | Highest residual risk; explicit user enable and stricter isolation; verify 16 KB page alignment of bundled `.so`. |

- Every package must be reviewed and its license recorded in
  [THIRD_PARTY.md](../THIRD_PARTY.md) before inclusion (Chaquopy's own license
  and SDK terms are "to add when wired").
- No package may open a network socket; the offline guarantee is enforced at the
  capability layer, not by convention.

## Why not debugpy

`debugpy` is **not used**. Rationale:

- It is designed for remote debugging over a network socket (DAP over TCP), which
  conflicts directly with the offline/no-server guarantee.
- It brings a large, native-extension-heavy dependency surface and would
  substantially increase the crash and license-audit burden on Android.
- PrimeX D.T's job model already provides typed progress, cancellation, and a
  persistent log — a separate debugger protocol is redundant for on-disk work,
  and live-process debugging is an explicit non-goal.

## Native crash residual risk

- CPython and any T3 native extension can segfault the process; unlike Rust
  parsers, they are not covered by the typed-error/`ByteReader` fail-closed model.
- Planned mitigations: run Python in an isolated process, marshal only typed
  envelopes across the boundary, capture crashes into the job's persistent log,
  and restart the worker without corrupting the workspace.
- This risk is **residual** and must be documented in a release's known issues;
  it is not eliminated.

## sdist typed error (planned)

Installing from a source distribution (`sdist`) at runtime is **not supported**:
it requires a compiler and network access. An attempt returns a typed
`sdist` error — mapped to `BadInput`/`CapabilityFailure` (exit codes 2/5) with a
message that only pre-built, allow-listed wheels may be loaded. There is no
silent fallback to fetching source.

## Verification

- Device check D12 in [VERIFICATION.md](VERIFICATION.md) covers isolation and
  no-network behavior. It is **UNRUN**.
- No Python package has been reviewed, bundled, or executed in this session.
