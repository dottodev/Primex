# Third-party components

This file records declared dependencies and the **license-verification status**
of each. "Verify" means: confirm the exact license and version-pinned terms from
the crate/JAR metadata before a public release. The repository's own code is
Apache-2.0 ([LICENSE](LICENSE)).

## Rust crates

All of the following are well-known crates commonly distributed under **MIT and/
or Apache-2.0** terms. The workspace declares `license = "Apache-2.0"` for
*PrimeX D.T's own code*; the dependency licenses below are **to be verified**
against the resolved versions in `Cargo.lock` and each crate's bundled `LICENSE`
file. Do not assume a single license for a dual-licensed crate.

| Crate | Pinned requirement | Declared use | License status |
|---|---|---|---|
| `serde` | `1.0` (`derive`) | typed schema (de)serialization | Well-known dual MIT/Apache-2.0 — **verify** pinned version |
| `serde_json` | `1.0` | JSON command bus / workspace documents | Well-known dual MIT/Apache-2.0 — **verify** |
| `sha2` | `0.10` | SHA-256 hashing / digest verification | Well-known dual MIT/Apache-2.0 — **verify** |
| `clap` | `4.5` (`derive`) | CLI argument parsing (`pdx-cli`) | Well-known dual MIT/Apache-2.0 — **verify** |
| `crc32fast` | `1.4` | ZIP entry CRC-32 verification | Well-known dual MIT/Apache-2.0 — **verify** |
| `flate2` | `1.0` | DEFLATE decompression for ZIP entries | Well-known dual MIT/Apache-2.0 — **verify** |
| `libc` | `0.2` | low-level POSIX/NDK FFI (PTY shim) | Well-known dual MIT/Apache-2.0 — **verify** |
| `jni` | to pin (JNI bridge) | JNI command bus bindings (`pdx-jni`) | Well-known dual MIT/Apache-2.0 — **verify** exact pinned version |

No dependency license has been audited in this session. Treat every "verify"
entry as an open release-blocking task.

## Android / UI

| Component | Version | License status |
|---|---|---|
| Miuix (`top.yukonga.miuix.kmp:miuix-ui-android`) | `0.9.4` (pinned) | Apache-2.0 per upstream POM (`maven2/.../miuix-ui-android-0.9.4.pom`) — **re-verify** on each Miuix bump |
| Sora Editor | decision pending | **Decision pending license review** — not yet a dependency. Do not vendor or bundle until its license is reviewed and recorded here. |
| Chaquopy | to add when wired | **To add when wired** — Python runtime is a plan only (see [docs/PYTHON.md](docs/PYTHON.md)); record its license and SDK terms at integration time. |

## Policy for additions

- Every new dependency must be entered here with: exact pinned version, declared
  use, and a license field that is either verified (with source) or marked
  "verify"/"pending".
- No game assets, game decryption keys, or proprietary third-party binaries may
  be added under any license.
- Copy each dependency's license text into the release `NOTICE`/`licenses`
  bundle as required by its terms. This has **not** been done yet.
