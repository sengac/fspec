# RPC-073 reopened — Cloud providers show NO models (models.dev catalog never wired)

**Date:** 2026-06-19
**Reason for reopen:** RPC-073 bug 3 ("model selector list is empty") was only
*partially* fixed. `list_providers()` was wired to
`codelet_providers::custom::list_providers_info()`, which returns the 17
canonical built-in providers **with empty model lists** (see handle_impl.rs:916
comment: "Built-in providers carry empty `models`"). So the model selector now
shows provider *rows* but every cloud provider is empty — diverging from the
TypeScript frontend, whose own RPC-073 examples [8]/[9] require each built-in to
expose "their own list of models" / "non-empty models".

## TypeScript reference (the model list source)

`src/tui/services/cloudSectionBuilder.ts`:
- `loadCloudModels()` → NAPI `modelsListAll()` → the models.dev catalog.
- `buildCloudSections(allModels)`:
  - For each models.dev provider, resolve credentials via `getProviderConfig`;
    `hasCredentials = registryEntry.requiresApiKey === false || !!apiKey`.
  - `toolCallModels = pm.models.filter(m => m.toolCall)`.
  - Keep only sections where `hasCredentials`.
  - Special cases: OpenAI cloud → synthetic "Codex (ChatGPT)" when Codex creds;
    anthropic OAuth override. (DEFERRED here — see Out of scope.)
- `mapProviderIdToInternal`/`mapModelsDevToRegistryId`: `google`↔`gemini`,
  `anthropic`↔`claude`.

## Rust building blocks that already exist (MODEL-001)

- `codelet_providers::models::ModelCache` — `new()` (=> `<data_dir>/cache/models.json`),
  `new_with_path(path)` (test), async `get()` (cache → else network fetch).
- `ModelRegistry::from_response(ModelsDevResponse)` — **pure, sync, no IO** (ideal
  for offline tests); `new(&cache).await` for production.
- `registry.list_models(provider) -> Result<Vec<&ModelInfo>>`.
- `ModelInfo { id, name, tool_call, reasoning, status, release_date, limit.context,
  modalities }`, `has_capability(Capability::Vision)`, `ModelStatus::Deprecated`.
- `codelet_sessions::credentials::resolve_credential(id, project_dir, data_dir)`
  → `Option<String>` (file → env → .env priority chain).
- NAPI parity helper `models_list_all` filters deprecated + sorts newest-first.

## Design (this card)

New module `codelet/sessions/src/cloud_models.rs` (< 300 LoC):
- `canonical_to_models_dev(id) -> &str` — `gemini`→`google`, else identity.
- `cloud_model_entries(registry, canonical_id, has_credentials) -> Vec<ModelEntry>`:
  - `has_credentials == false` → `[]` (credential-gated, mirrors TS).
  - look up `registry.list_models(canonical_to_models_dev(id))`; `Err` → `[]`
    (provider absent from models.dev, e.g. codex/github-copilot → unchanged).
  - filter `tool_call`, drop `Deprecated`, sort newest-first (NAPI parity).
  - map → `ModelEntry { id, display_name=name, context_window=limit.context,
    supports_reasoning=reasoning, supports_vision=has_capability(Vision),
    is_custom=false }`.
- `provider_has_credentials(id) -> bool` = `resolve_credential(id, None, None)
  .map(|o| o.is_some()).unwrap_or(false)`.

Wire `list_providers()` (handle_impl.rs): build the registry ONCE via
`block_in_place(|| Handle::current().block_on(ModelRegistry::new(&ModelCache::new()?)))`,
`Option` (None on any error → graceful degradation, rule [6]). For each
non-custom provider whose mapped `models` is empty, replace with
`cloud_model_entries(&registry, &p.name, provider_has_credentials(&p.name))`.
Custom providers and local-profile sections are untouched.

## Testability (offline)

- `cloud_model_entries` is pure: build `ModelRegistry::from_response` from an
  inline `ModelsDevResponse` JSON (anthropic + a deprecated model + a
  non-tool-call model + google/gemini) — assert tool-call-only, deprecated
  dropped, newest-first, `gemini`→`google` mapping, and `has_credentials=false`
  → `[]`. No network, no runtime, no globals.
- A source-shape test asserts `list_providers` references `cloud_model_entries`
  (the wiring/connection), so the helper can't rot unused.
- `list_providers()` itself uses `block_in_place` (already true via
  `build_local_profile_sections`) so it is only callable inside a tokio
  multi-thread runtime — we therefore do NOT unit-call it directly (matches the
  existing rpc073_list_providers_wiring constraint).

## Out of scope (documented deferrals — follow-ups)

- Synthetic "Codex (ChatGPT)" OpenAI section + Codex/Claude OAuth credential
  overrides (needs OAuth token NAPI parity).
- Hiding uncredentialed cloud providers entirely (TS filters them out of the
  selector); Rust keeps all canonical rows visible with empty models when
  uncredentialed — less disruptive to the existing model-selector UX.
- Cold-cache network fetch behaviour (degrades to empty models offline).
