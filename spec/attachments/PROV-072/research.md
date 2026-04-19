# PROV-072 — Extend custom-provider NAPI surface

## Problem

Current NAPI exposes only five custom-provider functions:

| Function | Purpose |
|---|---|
| `listProviders()` | Returns `JsProviderInfo[]` (built-ins + discovered custom) |
| `showProvider(name)` | Single-provider lookup |
| `validateProvider(name)` | Schema validation, no network |
| `testProvider(name)` | Probes `<baseUrl>/models` |
| `initProvider(projectRoot, name, template)` | Scaffolds a single `<name>.json` in `.fspec/providers/` |

`JsProviderInfo` (`index.d.ts:1009`) carries:
```ts
{ name, displayName?, available, isCustom, facade?, baseUrl?, apiKeyEnvVar?, models: string[] }
```

This is **insufficient** for TypeScript to render the provider settings screen or model selector for custom providers because:

1. `models: string[]` is just alias keys — loses `context_window`, `max_output_tokens`, `supports_tools/streaming/thinking`.
2. No way to query `fspec_home()` or discovery paths.
3. No way to know which on-disk file backs a provider (global vs project-local).
4. No way to list the raw config files before they've loaded (e.g., to show "5 invalid configs — click to see errors").
5. No way to trigger a re-scan after an external editor mutates `~/.fspec/providers/`.
6. No way to delete a custom provider (no `deleteProvider` NAPI).
7. No way to distinguish OAuth-backed custom providers from API-key-backed ones (the `AuthConfig` variant is dropped in `JsProviderInfo`).
8. No way to read the Rhai script content for preview/edit flows.
9. No way to check if a given slug is already taken before calling `initProvider`.

## New NAPI surface

All functions live in `codelet/napi/src/session_manager.rs` (or a dedicated `codelet/napi/src/custom_providers.rs` module if it exceeds 300 lines).

### 1. Path queries

```rust
#[napi]
pub fn fspec_paths() -> JsFspecPaths;

#[napi(object)]
pub struct JsFspecPaths {
    pub home: String,               // base directory
    pub credentials_dir: String,    // <base>/credentials
    pub providers_dir: String,      // <base>/providers (global)
    pub project_providers_dir: String, // <cwd>/.fspec/providers
    pub honours_fspec_home_env: bool,
}
```

Built on top of PROV-071's shared helper.

### 2. Enriched info

Extend `JsProviderInfo` with fields already present on the Rust side:

```ts
export interface JsProviderInfo {
  name: string
  displayName?: string
  available: boolean
  isCustom: boolean
  facade?: string
  baseUrl?: string
  apiKeyEnvVar?: string
  models: JsCustomModelInfo[]   // CHANGED: was string[]

  // NEW fields (custom only; undefined for built-ins):
  authType?: 'bearer' | 'api-key-header' | 'oauth-device' | 'oauth-pkce' | 'custom'
  configPath?: string                    // absolute path of source JSON
  scriptPath?: string                    // absolute path of sibling .rhai (if any)
  origin?: 'global' | 'project'          // which dir it was loaded from
  toolStyle?: 'openai' | 'anthropic' | 'none'
  apiStyle?: 'openai_chat' | 'openai_responses' | 'anthropic_messages' | 'custom'
}

export interface JsCustomModelInfo {
  alias: string
  id: string
  contextWindow?: number
  maxOutputTokens?: number
  supportsTools?: boolean
  supportsStreaming?: boolean
  supportsThinking?: boolean
}
```

Populated by extending `ProviderInfo` in `codelet/providers/src/custom/management.rs:23-40`.

### 3. File enumeration (includes invalid configs)

```rust
#[napi]
pub fn list_provider_files() -> Vec<JsProviderFileEntry>;

#[napi(object)]
pub struct JsProviderFileEntry {
    pub path: String,               // absolute path to JSON
    pub name: Option<String>,       // parsed `name` field, None if invalid
    pub origin: String,             // "global" | "project"
    pub valid: bool,
    pub error: Option<String>,      // parse/validation error
    pub has_script: bool,           // whether sibling .rhai exists
}
```

Implemented as a new helper in `management.rs` that walks directories without invoking `ProviderConfig::from_file` hard failure — returns invalid entries so the TUI can offer "fix this" UX.

### 4. Re-discovery + cache bust

```rust
#[napi]
pub fn rediscover_providers() -> RediscoverResult;

#[napi(object)]
pub struct RediscoverResult {
    pub count_global: u32,
    pub count_project: u32,
    pub invalid_count: u32,
    pub changed_names: Vec<String>,  // names added/removed/modified vs last snapshot
}
```

Clears the in-process `ProviderCredentials` detection snapshot and rebuilds. Must also notify `ProviderManager` so subsequent `ProviderType::from_str` calls see the new state.

### 5. Delete

```rust
#[napi]
pub fn delete_provider(name: String, scope: String) -> Result<()>;
// scope: "global" | "project" | "both"
```

Deletes both the `<name>.json` and any sibling `<name>.rhai` script from the chosen scope. Also clears any cached `ScriptLoader` AST entry.

### 6. Read script source

```rust
#[napi]
pub fn read_provider_script(name: String) -> Option<String>;
```

Returns the Rhai script text (bounded at 256 KB) for preview/edit in the TUI. Returns `None` when the provider has no script (facade-only providers).

### 7. Slug availability

```rust
#[napi]
pub fn is_provider_name_available(name: String) -> JsSlugCheck;

#[napi(object)]
pub struct JsSlugCheck {
    pub available: bool,
    pub slug_valid: bool,           // matches ^[a-z][a-z0-9-]*$
    pub conflicts_with_builtin: bool,
    pub conflicts_with_custom: Option<String>, // "global" | "project"
}
```

### 8. Init with script template

Extend `initProvider` to accept a richer scaffold:

```rust
#[napi]
pub fn init_provider(
    project_root: String,
    name: String,
    template: String,             // "openai-compatible" | "anthropic-compatible" | "rhai-full"
    scope: String,                // "global" | "project"
) -> InitProviderResult;

#[napi(object)]
pub struct InitProviderResult {
    pub config_path: String,
    pub script_path: Option<String>,  // Some(...) when template requires a .rhai
}
```

New templates:
- `openai-compatible` — existing, no Rhai script
- `anthropic-compatible` — facade="anthropic", no Rhai script
- `rhai-full` — facade omitted, generates a `<name>.rhai` with stubs for all 7 required lifecycle functions from PROV-062

## apply_custom_provider_env_vars exposure

Currently invoked internally only from `session_set_model_profile` (`session_manager.rs:6868`). Expose it:

```rust
#[napi]
pub fn apply_custom_provider_env_vars(
    provider_name: String,
    model_id: String,
    facade_override: Option<String>,
) -> Result<()>;
```

So TypeScript pre-flight flows (e.g., "Test this custom provider with model X") can prime env vars before calling `testProvider`.

## Backwards compatibility

Changing `JsProviderInfo.models: string[]` → `JsCustomModelInfo[]` is a **breaking NAPI change**. Two options:

- **Option A (recommended)**: Break. The only TS consumer today would be PROV-073, being written in the same epic. Ship the break in the same NAPI minor bump.
- Option B: Add `modelsEnriched: JsCustomModelInfo[]` alongside `models: string[]`. Cruftier but zero-risk.

Decision point for specifying phase.

## Acceptance summary

- `fspec_paths()` exposes all four path concepts with identical semantics to Rust helpers.
- `listProviders()` returns enriched models when `isCustom === true`.
- `list_provider_files()` surfaces parse-invalid entries so the TUI can render them.
- `rediscover_providers()` refreshes in-process caches and reports diff.
- `delete_provider` removes files and cached ASTs.
- `read_provider_script` returns source or None.
- `init_provider` with `"rhai-full"` template writes both `<name>.json` and `<name>.rhai` containing stubs for all 7 lifecycle functions.
- All new NAPI functions have TypeScript-side unit tests using a `FSPEC_HOME=<tmpdir>` fixture.

## References

- `codelet/napi/src/session_manager.rs:8184-8288` (existing five functions)
- `codelet/providers/src/custom/management.rs:20-100` (`ProviderInfo` + `list_providers_info`)
- `codelet/providers/src/custom/config.rs:130-245` (`ModelDef`, `ProviderConfig`)
- `codelet/providers/src/custom/discovery.rs:1-102` (discovery walk to generalize)
- `codelet/providers/src/custom/script_loader.rs:93` (AST cache that `delete_provider` must invalidate)
- `codelet/napi/index.d.ts:1009,1060,2324,2398,2471,927` (current NAPI declarations)
