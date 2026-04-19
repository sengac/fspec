# PROV-065 AST Research — Integration Points

Conducted 2026-04-17.

## Reuse targets

- `codelet/tools/src/facade/system_prompt.rs:215-237` — `SystemPromptFacade` trait. Must implement: `provider()`, `identity_prefix()`, `transform_preamble()`, `format_for_api()`.
- `codelet/tools/src/facade/system_prompt.rs:35-41` — `prepend_fspec_guidance(preamble: &str) -> String` — used by default `transform_preamble`.
- `codelet/tools/src/fspec_workflow_guidance.rs` — `pub const FSPEC_WORKFLOW_GUIDANCE: &str`.
- `codelet/providers/src/custom/script_loader.rs` — `ScriptLoader::load`, `engine_arc()` for running Rhai functions.
- `codelet/providers/src/custom/rhai_call.rs` — `call_fn1`, `call_fn2` helpers (`pub(crate)` — PROV-063).
- `codelet/providers/src/custom/conversion.rs` — Rhai `Dynamic` ↔ `serde_json::Value`.

## New file
`codelet/providers/src/custom/system_prompt.rs` (≤ 300 lines)

Public:
- `pub struct RhaiSystemPromptFacade { ... }` with `pub fn new(provider_name: String, engine: Arc<Engine>, ast: Arc<AST>, config: rhai::Dynamic) -> Self`.
- Implements `SystemPromptFacade` (add dep on `codelet-tools` in `codelet/providers/Cargo.toml` or use existing dep).

## `'static` lifetime handling
- Leak provider name once via `Box::leak` on first call, cached in a `once_cell::sync::OnceCell<&'static str>`.
- Same approach for `identity_prefix()`.

## Sync crossing
- Facade methods are synchronous. Use `tokio::task::block_in_place` when inside an async runtime, else direct sync `engine.call_fn()` since Rhai Engine is sync. Check research doc for the chosen pattern.

## Dependencies
- `once_cell` is already present in the workspace.
- `codelet-tools` must be listed as a dep in `codelet/providers/Cargo.toml` — check; if circular dep risk exists, define the trait in a shared spot or re-export.
