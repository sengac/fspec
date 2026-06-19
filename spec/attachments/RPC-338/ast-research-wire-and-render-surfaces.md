# RPC-338 — AST Research: wire-type, napi, server, and render surfaces

AST/structural analysis (AstGrep + Grep) confirming the exact code anchors
that this card must change. Performed during the specifying phase.

## 1. Wire type — `codelet/rpc-types/src/lib.rs`
AstGrep `pub struct ProviderInfo { $$$FIELDS }`:
- `ProviderInfo` at **lib.rs:362** — current fields: `key`, `display_name`,
  `models: Vec<ModelEntry>`. Derive set: `#[cfg_attr(feature = "napi",
  napi_derive::napi(object))]` + `Debug, Clone, Default, PartialEq, Eq,
  Serialize, Deserialize`.
- `ModelEntry` at lib.rs:341 already carries `is_custom: bool` (backs `[C]`,
  RPC-337 — out of scope here).
- Convention to follow for the new optional field: `ProviderCredentialInfo`
  (lib.rs:402) uses `masked_key: Option<String>` (L413) and
  `source: Option<String>` (L417) — doc comment (L392-399) states
  `napi(object)` does NOT support discriminated enums, hence `Option<String>`.

**Change:** add `profile_name: Option<String>` + `is_unreachable: bool` to
`ProviderInfo` (Default keeps `None` / `false`).

## 2. napi binding — `codelet/napi/src/models/napi_bindings.rs`
AstGrep `pub struct NapiProviderModels { $$$FIELDS }`:
- `NapiProviderModels` at **napi_bindings.rs:65** — fields `provider_id`,
  `provider_name`, `models: Vec<NapiModelInfo>`; `#[napi(object)]`.

**Change:** mirror `profile_name: Option<String>` + `is_unreachable: bool`.

## 3. Server data source
AstGrep `fn list_providers(&self) -> $RET { $$$BODY }`:
- `handle_impl.rs:930` — `fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo>`
  is the real implementation (maps internal provider info → wire `ProviderInfo`).
- Grep: `custom/management.rs:110` — `pub fn list_providers_info() -> Result<Vec<ProviderInfo>, ProviderError>`
  (internal richer type) is the upstream source.
- Reachability probe primitive: `openai.rs:300` —
  `pub async fn list_local_models_with_auth(base_url, api_key)` (L285 is the
  no-auth wrapper). This is the Rust equivalent of TS `modelsListLocalOpenai`.
- Profile config plumbing lives under `codelet/providers/src/custom/`
  (`config.rs`, `management.rs`); `manager.rs` already models
  `selected_profile_name: Option<String>` and the `"{provider}:{profile}/{model}"`
  composite (BUG-137) — so a profile concept already exists provider-side.

**Change:** in `handle_impl.rs::list_providers` default `profile_name=None`,
`is_unreachable=false` for cloud/custom; add a local-server profile branch that
enumerates `openai` profiles, builds `display_name = "openai: {profile}"`,
probes via `list_local_models_with_auth`, and sets `is_unreachable` on failure
(MODEL-004: not unreachable when the profile has custom models).

## 4. Render — `codelet/fspec-tui/src/views/model_selector/`
- `ModelSelectorRow` at **components/model_selector_dialog_rows.rs:25**
  (`pub(crate)`) — fields `label`, `badges`, `selectable`, `provider_key`,
  `model_id`. No profile/unreachable flag.
- `rows.rs:26` — `pub(crate) const LEGEND: &str = "[R] Reasoning | [V] Vision | [C] Custom";`
  (comment explicitly defers the `📁 Profile (local server)` segment to RPC-338).
- `rows.rs:45` — `build_view_rows(...)` pushes header rows
  `format!("{arrow} {} ({} models)", display_name, models.len())` (L72),
  `selectable: false`.
- `rows.rs:214` — `render_row(...)`: header branch (L221-230) paints
  `format!(" {}", row.label)` as a single span (REVERSED|BOLD when selected).

**Change:** extend `ModelSelectorRow` with `is_profile`/`is_unreachable`; set
them in `build_view_rows`; in `render_row` header branch insert a magenta `📁`
span after the arrow + a red ` (unreachable)` span after the count (both adopt
the selected highlight style when selected); append
` | 📁 Profile (local server)` to `LEGEND`.

## TS parity anchors (cross-checked, read in full)
- `src/tui/services/profileSectionBuilder.ts:106-183` — `displayName =
  \`${providerId}: ${profileName}\`` (L116); `effectiveUnreachable =
  isUnreachable && !hasCustomModels` (L159). NOTE L163-165 mutates
  `providerName` to embed "(unreachable)" — we deliberately do NOT port that
  (it would double-render against the renderer's structured marker).
- `src/tui/components/ModelSelectorView.tsx:150-187` — render order:
  arrow (L167) → 📁 magenta (L168-172) → displayName (L173) → "(N models)"
  dim (L174-177) → red "(unreachable)" (L178-183); legend at L270.
