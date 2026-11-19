# Build guide

## Pinned versions (use exactly)

| Item | Value |
|---|---|
| Workspace / tool version | `0.1.0` |
| Rust edition | `2021`, declared MSRV `1.76` (host verified on **1.99.1**) |
| Miuix | `0.9.4` — `top.yukonga.miuix.kmp:miuix-ui-android:0.9.4` |
| Kotlin | `2.4.21` (the `org.jetbrains.kotlin.android` plugin is gone in AGP 9; Kotlin is built into AGP) |
| Compose compiler plugin | `org.jetbrains.kotlin.plugin.compose:2.4.21` |
| Compose runtime | `androidx.compose.*:1.12.0` (Miuix's `-android` AARs are built on these) |
| AGP | `9.5.1` |
| Gradle wrapper | `9.6.0-bin` (minimum required by AGP 9.5.1) |
| minSdk / targetSdk | `26` / `35` |
| compileSdk | `37` (minor 0) — see deviation note below |
| ABI | `arm64-v8a` required; `x86_64` included for the emulator |
| Native lib alignment | 16 KB page-aligned (not yet enforced in the link step — open task) |
| License | Apache-2.0 |

**Deviation from the written spec — compileSdk 35 → 37.** Verified from the
published AAR metadata (`META-INF/com/android/build/gradle/aar-metadata.properties`)
of `miuix-ui-android` **0.9.0 through 0.9.4**: every version declares
`minCompileSdk=37`. AGP refuses to build with compileSdk 35. `targetSdk` remains
**35** as specified, so runtime behavior is unchanged; only the build-time API
level was raised. AGP 9.5.1, Gradle 9.6.0, and installed platform
`platforms;android-37.0` were required to make the Miuix scaffold buildable.

## Confirmed host toolchain (observed in this session)

- Rust **1.99.1**, cargo **1.99.1**
- Java **17** (`jdk-17.0.18.8-hotspot`)
- Android SDK platforms **34, 35, 36, 36.1, 37.0**; build-tools **34–37**; NDK **25.1.8937393**, **27.0.12077973**
- `cargo-ndk` **4.1.2** installed; Rust Android targets `aarch64-linux-android`, `x86_64-linux-android` installed
- Gradle wrapper 9.6.0; daemon builds succeed
- `gh` CLI authenticated as `dottodev`; target repo `dottodev/Primex` (empty, `main`)
- Graphify MCP had **no repositories enabled** for this workspace, so no indexed graph was available

## Commands — all executed in this session

```sh
# Rust: host tests and lints
cargo test --workspace                       # PASS: 47 tests, 0 failed
cargo clippy --workspace --all-targets -- -D warnings   # PASS: no warnings
cargo fmt --all -- --check                   # PASS after applying cargo fmt

# Host CLI smoke
cargo run -p pdx-cli -- info --json
cargo run -p pdx-cli -- apk list fixtures/minimal.apk
cargo run -p pdx-cli -- apk verify fixtures/minimal.apk --json

# Native bridge (Android ABIs)
$env:ANDROID_NDK_HOME = "$ANDROID_HOME\ndk\27.0.12077973"
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build --release -p pdx-jni
# -> app/src/main/jniLibs/{arm64-v8a,x86_64}/libpdx_jni.so
# Exported symbol verified with llvm-nm -D:
#   Java_dev_primex_dt_jni_CommandBus_nativeSend

# Android shell
./gradlew :app:assembleDebug
# -> app/build/outputs/apk/debug/app-debug.apk  (11,680,062 bytes)
# APK contains lib/{arm64-v8a,x86_64}/libpdx_jni.so
```

`aapt2 dump badging` on the produced APK confirms:
`package='dev.primex.dt'`, `versionCode='1'`, `versionName='0.1.0'`,
`minSdkVersion='26'`, `targetSdkVersion='35'`, `application-label='PrimeX D.T'`.

## OneDrive build-directory caveat

The checkout lives under OneDrive, and Gradle occasionally hit
`AccessDeniedException` / "Unable to delete directory" inside `app/build`
(first `aar_metadata_check`, later `mergeDebugNativeLibs`). Removing
`app/build` before the build resolved it. If it recurs, delete `app/build`
and retry; do not disable OneDrive or move the source tree.

## Install SDK platforms

```sh
sdkmanager --licenses
sdkmanager "platforms;android-35" "platforms;android-37.0" "build-tools;37.0.0"
```

## Reproducibility notes

- `Cargo.lock` is committed; build with `cargo test --locked --workspace`.
- Gradle version catalog pins every dependency; no dynamic `+` versions.
- Record exact Rust/NDK/SDK/Gradle versions beside each release artifact.
- Native libs are produced by `cargo-ndk` into `jniLibs/<abi>` and shipped in the APK.
- No signing keys are stored in the repo; release signing material is operator-supplied.
- Deterministic timestamps / fixed file order for release builds are **not implemented** yet.
