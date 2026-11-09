# PROV-172 AST research

## fspec astgrep --pattern 'u32::try_from($A).unwrap_or(u32::MAX)' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern '$A as u32' --lang rust --path rust/sessions/src/model_resolution.rs
rust/sessions/src/model_resolution.rs:242:53:tokens as u32
rust/sessions/src/model_resolution.rs:243:59:tokens as u32
rust/sessions/src/model_resolution.rs:250:23:tokens as u32


## fspec astgrep --pattern 'ProviderModelInfo { $$$F }' --lang rust --path rust/providers/src/custom
rust/providers/src/custom/management.rs:198:33:ProviderModelInfo {


## fspec astgrep --pattern 'resolve_current_model_limits($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/runtime_settings.rs:17:31:resolve_current_model_limits(manager)
rust/sessions/src/model_resolution.rs:232:8:resolve_current_model_limits(pm)


## fspec astgrep --pattern 'u32::try_from($A).unwrap_or($B)' --lang rust --path rust/sessions/src/handle_impl
No matches found

## fspec astgrep --pattern 'apply_model_selection($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/model_resolution.rs:287:18:apply_model_selection(&mut provider_manager, model)


## Findings

- The two clamps live in macro_rules fragments (`compose_handle_impl_5!` / `_8!`), which
  ast-grep does not parse; grep finds them at handle_impl_5.rs:64 and handle_impl_8.rs:53
  (`.map(|window| u32::try_from(window).unwrap_or(u32::MAX))`).
- `ProviderModelInfo` is built in one place (management.rs:198) — the conversion belongs there.
- `resolve_current_model_limits` truncates with `as u32` (model_resolution.rs:242/243/250) and
  has two callers: `apply_model_selection` (model_resolution.rs:232; reached from session
  creation, session_creation_helper.rs:171, `build_provider_manager` :287 and the mid-session
  switch) and `refresh_runtime_settings_for_turn` (runtime_settings.rs:17). Both already return
  `Result<_, String>`.
