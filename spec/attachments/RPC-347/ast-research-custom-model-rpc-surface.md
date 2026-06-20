# RPC-347 — AST research: custom-model RPC + NAPI surface

Goal: expose the RPC-346 persistence functions (`save_custom_model` /
`delete_custom_model`) over the full wire surface, mirroring the existing
`set_session_model` / `list_providers` plumbing. Below are the concrete anchor
points found via AstGrep / grep for each layer.

## RPC-346 building blocks (codelet/sessions/src/profile_sections.rs)
- `pub fn save_custom_model(provider_id: &str, profile_name: &str, definition: &CustomModelDef, original_model_id: Option<&str>) -> std::io::Result<()>` (line ~188)
  - add = `original_model_id = None` (append); update = `Some(old_id)` (replace in place)
- `pub fn delete_custom_model(provider_id: &str, profile_name: &str, model_id: &str) -> std::io::Result<()>` (line ~212)
- Path-injectable cores `save_custom_model_at` / `delete_custom_model_at` (lines ~244 / ~285) — used for OFFLINE tests against temp config files.
- `pub struct CustomModelDef` (line ~94): `id` (required) + optional `display_name`, `facade`, `context_window`, `max_output_tokens`, `compaction_threshold: Option<CompactionThreshold>`, `reasoning`, `has_vision`. `#[serde(rename_all="camelCase")]`, skip-None.
- Guards: only `provider_id == "openai"`; missing profile → no-op `Ok`.

## Layer mirror pattern (set_session_model)
AstGrep `fn set_model(&self, $$$ARGS) -> Result<(), String> { $$$BODY }`:
- `codelet/core/src/session_manager_handle.rs:148` — trait default no-op (`Ok(())`).
- `codelet/sessions/src/handle_impl.rs:1008` — concrete override (real work).

AstGrep `async fn set_session_model(&self, $$$ARGS) -> Result<()> { $$$BODY }`:
- `codelet/fspec-tui/src/transport/embedded.rs:198`
- `codelet/fspec-tui/src/transport/websocket.rs:393`

Other layers:
- RPC trait `FspecService` — `codelet/rpc/src/lib.rs:175` (`async fn set_session_model(...)`); `FspecServiceImpl` delegation block from line 747, pattern `match self.inner.session_manager() { Some(handle) => ..., None => <safe default> }`.
- `FspecBackend` trait — `codelet/fspec-tui/src/transport/mod.rs:179` (`set_session_model`), `:173` (`list_providers`).
- NAPI — `codelet/napi/src/models/napi_bindings.rs` (`#[napi] pub async fn models_list_all`, uses `registry.list_providers()`); session-level model NAPI in `codelet/napi/src/session_bindings.rs:1766` (`session_set_model`). The custom-model write bindings will live alongside the model listing bindings.
- TUI `Action` enum — `codelet/fspec-tui/src/components/mod.rs:107`. Existing model/provider variants (e.g. `OpenModelSelectorView`, `SaveProviderCredentials { provider_id, api_key }`, `RefreshProviderModels(String)`) show the struct-variant payload convention. New: `AddCustomModel`, `EditCustomModel`, `DeleteCustomModel` (inert here; RPC-344 wires a/e/d).

## Plan (3 RPC methods)
- `add_custom_model(profile, definition)` → handle → `save_custom_model(.., None)`
- `update_custom_model(profile, original_id, definition)` → `save_custom_model(.., Some(original_id))`
- `delete_custom_model(profile, model_id)` → `delete_custom_model(..)`
- New wire type `CustomModelDefinition` in `codelet/rpc-types/src/lib.rs` (alongside `ModelEntry` line ~341), camelCase, maps 1:1 to `CustomModelDef`.

## Offline test strategy
Exercise the real persistence via the path-injectable `*_at` cores + temp
`fspec-config.json`; no env mutation, no network. Cross-transport parity test
follows the existing `rpcNNN_cross_transport_parity.rs` pattern in
`codelet/fspec-tui/tests/`.
