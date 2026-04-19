# PROV-075 — Custom providers in Model Selector (`/model`)

## Problem

The Model Selector is fed by `providerSections` in the Zustand `modelStore` (`src/tui/store/modelStore.ts:36,99,162`). Sections are populated by `modelInitializationService.ts:193-196`, which combines:

1. `profileSections` — from `loadProfileSections()` (local OpenAI-compatible via `modelsListLocalOpenai`)
2. `cloudSections` — from `buildCloudSections(cloudModels)` (fed by `modelsListAll` — models.dev cache only)

Neither source knows about custom Rhai providers. `modelsListAll` inside Rust (`codelet/providers/src/models/registry.rs:129`) only iterates the models.dev registry. Therefore:

- A user who has `~/.fspec/providers/my-provider.json` with `models: { "fast": { id: "fast-v1" } }` will **not** see "my-provider / fast" anywhere in `/model`.
- Even if they manually select `my-provider/fast` via `/agent set-model my-provider/fast` (if such CLI exists), the model registry treats it as an unknown model and falls back to default limits.

## Target data flow

Rust becomes the single source of cloud+custom provider model data; models.dev remains one source among several, consumed by Rust.

### Option A (recommended): Rust-side merge

Extend `codelet/providers/src/models/registry.rs` with a new `list_providers_with_custom()` that:

1. Takes the models.dev cache as today.
2. Iterates `discover_provider_configs()`.
3. For each custom provider, synthesizes a `ProviderInfo` entry with:
   - `id = cfg.name`
   - `name = cfg.display_name`
   - `models: HashMap<String, ModelInfo>` derived from `cfg.models` where each `ModelDef` becomes a `ModelInfo { id, context_window, max_output_tokens, supports_tools, supports_streaming, supports_thinking, release_date: None, status: "current", ... }`
4. Merges into the returned list.

Expose a new NAPI:

```rust
#[napi]
pub async fn models_list_all_with_custom() -> Result<Vec<NapiProviderModels>>;
```

…or replace `modelsListAll` semantics to always include custom (safer because callers don't need to opt-in). The current `modelsListAll` name stays; behaviour changes to be superset.

### Option B: TypeScript-side merge

Call `listProviders()` in `modelInitializationService` and synthesize sections client-side. Requires duplicating the "which models are current" filter that currently lives in `is_current_model`. **Rejected** — violates "Rust is the source of truth".

## Sections returned for custom providers

```ts
{
  providerId: 'my-provider',
  providerName: 'my-provider',    // or display_name
  models: [
    {
      modelId: 'fast',
      providerId: 'my-provider',
      contextWindow: 128000,
      maxOutputTokens: 4096,
      supportsTools: true,
      supportsStreaming: true,
      releaseDate: undefined,
      status: 'current',
      isCustom: true,              // new discriminator flag on NapiModelInfo
    },
    // ...
  ],
  isUnreachable: !providerInfo.available,
  isCustom: true,                  // new flag on NapiProviderModels
}
```

The `isUnreachable` flag is already used by `modelInitializationService.ts:196` to gate empty sections; custom providers without credentials should still render (they list models; they just can't be selected until credentials are present) — so `isUnreachable` here really means "no credentials AND no models", and custom providers always pass the filter because they always declare models.

## Flat list rendering

`src/tui/utils/flat-model-list.ts` already handles arbitrary providers — no change needed other than the new `isCustom` indicator badge. Suggested rendering:

```
▼ my-provider [custom]
    fast          ctx 128000   out 4096
    default       ctx 32000    out 2048
```

Optional: suffix models with `(via <facade>)` when `provider.facade` is set, to remind users the request goes through another provider's HTTP stack.

## Auto-selection semantics

`detect_default_provider()` (`manager.rs:530-557`) explicitly excludes custom providers. This is correct — custom providers must be **explicitly selected**. No change in PROV-075.

## Model selector confirmation

When the user picks a custom-provider model, `AgentView.handleModelSelected` eventually calls `session_set_model_profile` (`session_manager.rs:6765`). That already routes through `set_model_direct` + `apply_custom_provider_env_vars` for `ProviderType::Custom(_)` (lines 6866-6876). No change — the plumbing works as soon as `/model` can name a custom-provider model.

## Test plan

- With an `FSPEC_HOME` fixture containing one custom provider JSON with two models, call `modelsListAll()` and verify the custom section is returned with both models enriched.
- UI test: Model Selector renders the custom provider section with `[custom]` badge and both models.
- Selecting a custom-provider model triggers `session_set_model_profile` which applies env vars and completes selection.

## Acceptance summary

- `modelsListAll()` (or `modelsListAllWithCustom` — decide during specifying) returns custom-provider sections.
- Model Selector displays custom providers + models with context-window / max-output metadata.
- Custom providers never auto-select but are fully selectable when credentials are present.
- Custom providers with missing credentials still list their models but are visually marked unreachable.

## Dependencies

- PROV-072 (NAPI enrichment carries `JsCustomModelInfo`; register needs full `ModelDef`)
- PROV-073 (registry cache invalidation so models refresh after `initProvider` / `deleteProvider`)

## References

- `src/tui/utils/flat-model-list.ts:17`
- `src/tui/services/modelInitializationService.ts:193-196`
- `codelet/napi/src/models/napi_bindings.rs:103`
- `codelet/providers/src/models/registry.rs:129`
- `codelet/providers/src/custom/config.rs:161-245` (ModelDef, ProviderConfig)
- `codelet/napi/src/session_manager.rs:6765,6866-6876` (set_model_direct + env var application)
- `codelet/providers/src/manager.rs:530-557` (auto-selection exclusion)
