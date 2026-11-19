# Plugin API (draft)

**Status: NOT IMPLEMENTED.** Everything below is a design proposal. No plugin
host, loader, or API exists yet. The examples are **illustrative skeletons** and
are not runnable against any current build. The host is planned to arrive in M8.

## 1. Command namespaces (proposed surface)

Plugins and the Kotlin shell share the same namespaced command names as the JNI
bus (see [ARCHITECTURE.md](ARCHITECTURE.md) §2). All calls are asynchronous and
return the same typed `{ v, id, ok, result | error }` envelope.

| Namespace | Representative commands (planned) | Purpose |
|---|---|---|
| `ide.commands` | `list`, `register`, `describe` | Discover and invoke registered commands |
| `workspace` | `open`, `close`, `list`, `roots` | Open/manage `.pdxproj` workspaces (paths from `profile.json`) |
| `documents` | `open`, `read`, `edit`, `save` | Text/hex document operations in the workspace |
| `diagnostics` | `publish`, `clear`, `list` | Emit/read diagnostics; feed triage/audit |
| `statusbar` | `set`, `clear`, `progress` | Status-bar text and job progress |
| `settings` | `get`, `set`, `subscribe` | Scoped plugin settings |
| `terminal` | `open`, `write`, `cancel` | PTY terminal (via `pdx-term`) |
| `primex` | `version`, `capabilities`, `format` | Tool identity, capability ladder level, format-engine queries |

Every command input/output is schema-versioned. An unknown namespace, verb, or
version returns a typed error — never an untyped exception.

## 2. `.pdxplug` layout (proposed)

A `.pdxplug` is a ZIP container:

```
my-plugin.pdxplug
├── manifest.json          # required: id, version, tier, permissions, api_version
├── signature.ed25519      # required: Ed25519 signature over manifest + payload digest
├── payload/
│   ├── descriptor.json    # T1: declarative contributions
│   ├── main.lua           # T2: Lua script entry
│   └── libplugin.so       # T3/T4: native cdylib (arm64-v8a, 16 KB aligned)
└── resources/             # optional icons/strings (no game assets)
```

`manifest.json` example:

```json
{
  "id": "dev.example.hexhighlight",
  "version": "0.1.0",
  "tier": 2,
  "api_version": 1,
  "permissions": ["documents.read", "statusbar.set"],
  "entry": "payload/main.lua"
}
```

The host verifies `signature.ed25519` against its trusted public key and checks
that the signed digest covers both `manifest.json` and `payload/**` **before**
any file is read as executable. Failure → `VerificationFailed` (exit code 4);
the plugin is rejected and no code runs.

## 3. Tiers and permissions (planned)

| Tier | Form | Allowed permissions | Forbidden (all tiers) |
|---|---|---|---|
| **T1** | Data-only descriptor | `diagnostics.publish`, `statusbar.set`, `settings.get` | network, process attach, raw filesystem |
| **T2** | Lua script | T1 + `documents.read`, `workspace.roots`, `primex.*` | network, `documents.edit`, native code |
| **T3** | Native cdylib, isolated process | T2 + `documents.edit`, `terminal.*` | network, live-game tooling, unrestricted filesystem |
| **T4** | Privileged, manually installed + per-use consent | T3 + `workspace.open`, full `primex.*` | network, live-game tooling; filesystem only through VFS |

- Permissions are **allow-listed and declared in the signed manifest**; a
  runtime call outside the declared set is a typed `CapabilityFailure`.
- File access is always workspace-scoped through the VFS (planned).
- There is no network permission at any tier (offline guarantee).

## 4. Illustrative skeletons — NOT YET IMPLEMENTED

These show the intended shape only. There is no host to load them.

### T1 — declarative descriptor (illustrative, not implemented)

```json
{
  "id": "dev.example.hexhighlight",
  "tier": 1,
  "contributes": {
    "diagnostics": [{ "code": "EXAMPLE001", "severity": "info" }],
    "statusbar": [{ "command": "primex.version" }]
  }
}
```

### T2 — Lua script (illustrative, not implemented)

```lua
-- T2: allow-listed read-only commands only.
local ok, res = pdx.call("documents.read", { uri = "workspace:/a.txt" })
if not ok then
  pdx.call("diagnostics.publish", { code = "EXAMPLE001", message = res.message })
  return
end
pdx.call("statusbar.set", { text = "bytes: " .. #(res.result.data or "") })
```

### T3 — native cdylib (illustrative, not implemented)

```rust
// T3: isolated-process native plugin. Exposes the planned host ABI.
// extern "C" fn pdx_plugin_init(api: *const PdxApi) -> i32;
#[no_mangle]
pub extern "C" fn pdx_plugin_init(_api: *const std::ffi::c_void) -> i32 {
    // Register a compute-heavy parser via the allow-listed command set.
    0 // 0 = success; nonzero = typed capability failure
}
```

### T4 — privileged plugin (illustrative, not implemented)

```rust
// T4: requires manual install, signed manifest, per-use consent.
#[no_mangle]
pub extern "C" fn pdx_plugin_init(_api: *const std::ffi::c_void) -> i32 {
    // May call workspace.open / full primex.* through the VFS,
    // but never network or running-game tooling.
    0
}
```

## 5. Open decisions

- Exact ABI symbol names and `PdxApi` vtable layout for T3/T4.
- Trusted-key distribution and revocation.
- Whether T2 Lua uses the same runtime as the M7 Lua automation engine.
- How `api_version` maps to capability/permission changes over time.
