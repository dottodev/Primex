# On-disk format engines

This table is a **plan**. "Verified" records what the code can actually check;
"uncertain" records assumptions that must be confirmed against real modified
builds before a decoder is trusted. **Only ZIP central-directory inspection is
partially implemented; no format has been validated on a real sample in this
session.** All indexes use `ByteReader` so a truncated field is a typed
`BadInput` with an absolute offset — never a guessed value.

| Engine | On-disk input | Verified (checkable) | Uncertain / assumptions | Status |
|---|---|---|---|---|
| ZIP / APK central directory | `.apk`, `.zip`, expansion files | EOCD + central-directory signature, entry counts, local-header offsets, CRC-32 per entry, DEFLATE via `flate2` | Zip64, data descriptors, UTF-8 flag subtleties, alignment/padding conventions of modified builds | **PARTIALLY IMPLEMENTED** (`pdx-apk`), unrun |
| ZIP payload decompression | stored/deflated entries | Decompressed size + CRC match | Encryption, uncommon compression methods | PLANNED |
| UnityFS | `*.bundle`, `assets/*.resources` | Magic/version header fields present in the container | Unity version layout variants, compressed block tables, LZ4/Oodle blocks, catalog offsets | NOT IMPLEMENTED |
| IL2CPP metadata | `global-metadata.dat`, `libil2cpp.so` | Header magic + version field sanity | String/type/method table layout per Unity/IL2CPP version, class registration | NOT IMPLEMENTED |
| DEX | `classes*.dex`, multi-dex sets | Magic + header size + checksum/signature sanity | Artifacts from nonstandard toolchains, compact DEX, multi-dex ordering | NOT IMPLEMENTED |
| AXML (binary XML) | `AndroidManifest.xml` binary form | Chunk types/headers | String-pool encoding, attribute extension layout, resource-id maps | NOT IMPLEMENTED |
| ARSC (resource tables) | `resources.arsc` | Table chunk signature + size | Package/key/typespec variants, sparse entries, overlay layouts | NOT IMPLEMENTED |
| ELF disassembly | `*.so`, executables | ELF header + section/program headers, architecture (`arm64-v8a`) | Symbol stripping, dynamic relocation interpretation, instruction set decode coverage | NOT IMPLEMENTED |
| Hex editor / byte patch | arbitrary file | Bounds-checked reads, diff of byte ranges | Undo/rollback policy, patch format, safe-write atomicity | NOT IMPLEMENTED |
| Diff / audit / triage | two workspace snapshots | Byte-level structural diff, hash per artifact | Semantic diff rules, triage classification, report schema | NOT IMPLEMENTED |
| Repack / sign / install | APK output artifacts | ZIP integrity + signature digest before install | Signing scheme versions, v2/v3 blocks, install/adb permissions (L3) | NOT IMPLEMENTED |
| logcat / tombstone | device logs (L3) | Structured parse of captured text | Format variance across Android versions; **no live-game attachment** | NOT IMPLEMENTED |

## Rules for every engine

1. **Fail closed.** Unknown or future format versions return
   `UnsupportedFormatVersion`; corruption returns `BadInput` with offset,
   expected, and actual. No engine emits zero-filled or guessed structures.
2. **Named layout roots only.** MLBB path layouts live in `profile.json`
   `path_roots` (see `pdx-core`); no parser hardcodes an on-device path.
3. **Offline.** Engines read local bytes only; none may open a network socket.
4. **Test evidence required.** A decoder is not "verified" until a host unit test
   against a fixed fixture has been *run* and recorded in
   [VERIFICATION.md](VERIFICATION.md).
