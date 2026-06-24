# PROV-120 — AST Research (codebase analysis for first-available startup model init)

AST/grep analysis confirming the integration surfaces for restoring TS-parity
first-available model initialization. Tool: AstGrep + ripgrep over `codelet/`.

## Session decline + default-model accessors (the regression site)
- `codelet/sessions/src/handle_impl.rs:82` — `fn create_session(role) -> SessionId`;
  decline log at `:91` `"create_session declined: no default model set (PROV-101: no anthropic fallback)"`.
- `codelet/sessions/src/handle_impl.rs:823` — `create_isolated_session` decline (Err) variant.
- `codelet/sessions/src/handle_impl.rs:995` — `fn set_default_model(&self, model: &str)` (handle delegate).
- `codelet/sessions/src/session_manager.rs:227` — `pub fn set_default_model(&self, model)`.
- `codelet/sessions/src/session_manager.rs:245` — `pub fn get_default_model(&self) -> Option<String>`.

## Section assembly (ordered sections feeding the selector)
- `codelet/sessions/src/handle_impl.rs:903` — `fn list_providers() -> Vec<ProviderInfo>`;
  cloud/custom built-ins first, then `build_local_profile_sections()` appended (~:979).
  ⚠️ ORDER: Rust emits cloud/custom → profiles; TS is profiles → custom → cloud.
- `codelet/sessions/src/cloud_models.rs:46` — `pub fn cloud_model_entries(...)` (credential-gated).
- `codelet/sessions/src/profile_sections.rs:443` — `pub fn build_local_profile_sections() -> Vec<ProviderInfo>`.
- `codelet/sessions/src/profile_sections.rs:44` — `pub fn build_profile_provider_info(...)`
  (MODEL-004 `is_unreachable = probe_failed && !has_custom_models`).

## Wire types (pure resolver input — unit-testable, no network)
- `codelet/rpc-types/src/lib.rs:416` — `pub struct ProviderInfo { key, display_name, models, profile_name: Option<String>, is_unreachable }`.
- `codelet/rpc-types/src/lib.rs:341` — `pub struct ModelEntry { id, display_name, ... }`.
- No `has_credentials` field → proxy = "section has >=1 model".

## Persistence surfaces
- `codelet/sessions/src/profile_sections.rs:183` — `fspec_user_dir()` (FSPEC_USER_DIR or $HOME/.fspec).
- `codelet/sessions/src/profile_sections.rs:377` — `read_config_value(config_path) -> Option<serde_json::Value>` (reuse for `tui.lastUsedModel`).
- `codelet/sessions/src/default_model_persistence.rs` — legacy `default-model.json:model` (PROV-119; back-compat read source).
- grep: zero hits for `lastUsedModel` / `last_used_model` in `codelet/` → reader must be added.

## Insertion point (Rust analogue of TS AgentView mount)
- `codelet/fspec/src/combined.rs:59` — `app.bootstrap().await`.
- `codelet/fspec-tui/src/app/bootstrap.rs:26` — `pub async fn bootstrap(&mut self)` (insert init here).
- Lazy first `create_session` at `codelet/fspec-tui/src/app/dispatch.rs:58` and `:98`
  (`Action::EnterWorkUnit`/`OpenAgentView`) → init must `set_default_model` before these.
- `FspecBackend` trait already exposes `list_providers()` and `set_default_model()` on
  embedded + websocket transports (`codelet/fspec-tui/src/transport/mod.rs`).

## PROV-101 code to KEEP untouched
- `codelet/providers/src/provider_resolution.rs:25` — `resolve_unambiguous_provider(...)`
  (ambiguous multi-cred → Err; no silent Claude pick).
- `handle_impl.rs` decline paths stay; the decline simply becomes the genuine
  zero-reachable-models edge case once startup proactively sets a default.
