# RPC-348 AST research — mid-session facade re-resolution

AST-grep findings (language: rust) used to scope the facade re-resolution fix.

## Shared resolver (fix site)
- `pub fn apply_model_selection(...)` — `codelet/sessions/src/model_resolution.rs:28`
  - Custom branch currently calls `pm.set_model_direct(registry_provider, model_part, None, None, None)`
    (5th arg `facade_override` hard-coded `None`) — model_resolution.rs:71.
  - Non-custom branch calls `pm.select_model(model)` — model_resolution.rs:74.
  - Both creation (`session_manager.rs:500`) and mid-session (`handle_impl.rs:1049`) call this helper,
    so the facade gap is SHARED across both paths.

## Facade derivation (already exists, reuse)
- `pub fn derive_facade_for_custom(name: &str) -> Option<String>` — `codelet/providers/src/custom/management.rs:440`
  - explicit config `facade` wins → `Some(f)`;
  - non-empty Rhai `script` with no explicit facade → `None`;
  - else derived from `api_style`: anthropic_messages → `"claude"`, openai_chat → `"openai"`.
  - Exported via `codelet/providers/src/custom/mod.rs:59` (`pub use ... derive_facade_for_custom ...`).
- `pub fn apply_custom_provider_env_vars(name, model_id, facade) -> Result<(), ProviderError>` —
  `codelet/providers/src/custom/management.rs:380` (sets OPENAI_BASE_URL/OPENAI_MODEL/keys for the facade).

## Provider manager facade API
- `pub fn set_facade_override(&mut self, facade: Option<String>)` — `codelet/providers/src/manager.rs:1157`.
- `pub fn facade_override(&self) -> Option<&str>` — `codelet/providers/src/manager.rs:1152` (observation point).
- `set_model_direct` stores facade (`self.facade_override = facade_override;` manager.rs:603);
  `select_model` (manager.rs:437) NEVER touches `facade_override` → stale facade leaks on registry switch.

## Gold-standard pattern to mirror (NAPI creation path)
- `codelet/napi/src/session_bindings.rs:1947-1993` (session_set_model_profile): after
  `set_model_direct_with_profile`, for `ProviderType::Custom(_)` derive facade via
  `derive_facade_for_custom`, `set_facade_override`, then `apply_custom_provider_env_vars`.

## set_model entry (no wire change)
- `fn set_model(&self, session_id, provider_id, model_id) -> Result<(), String>` —
  `codelet/sessions/src/handle_impl.rs:1008` (3-arg signature stays; locked source-shape contract test).

## custom provider registration check
- `pub fn custom_provider_registered(slug: &str) -> bool` — `codelet/providers/src/manager.rs:131`
  (consults `discover_provider_configs()` → reads `.fspec/providers/*.json` under CWD + FSPEC_HOME).

## Test fixture references
- `codelet/sessions/tests/rpc343_mid_session_model_reresolution.rs` (SessionManager + dummy creds setup).
- `codelet/providers/tests/custom_provider_manager_integration_test.rs:94-165`
  (DiscoveryFixture redirecting HOME/FSPEC_HOME/CWD + write_project_custom_provider helper).
