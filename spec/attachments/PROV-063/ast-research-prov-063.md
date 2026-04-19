# PROV-063 AST Research — Existing Integration Points

Conducted via AstGrep on codelet/providers/src/ on 2026-04-17.

## Reuse targets

- `codelet/providers/src/lib.rs:90-118` — `LlmProvider` trait + `async_trait`. `RhaiCustomProvider` must implement this.
- `codelet/providers/src/lib.rs:64-82` — `CompletionResponse`, `MessageContent`, `StopReason` (EndTurn/ToolUse/MaxTokens). Response bridge maps Rhai → these types.
- `codelet/providers/src/error.rs` — `ProviderError` enum (Auth, RateLimit, Api, Network, Config, etc.). map_error's Rhai return values must resolve to one of these variants.
- `codelet/providers/src/custom/config.rs` (PROV-062) — `ProviderConfig`, `AuthConfig`, `ModelDef`.
- `codelet/providers/src/custom/script_loader.rs` (PROV-062) — `ScriptLoader::load`, `validate_required_functions`, `engine`. RhaiCustomProvider holds an `Arc<ScriptLoader>`.
- `codelet/providers/src/oauth/engine.rs` — `build_sandboxed_engine` already registered. Reuse via ScriptLoader.
- `codelet_common::Message` — message type used across providers.

## New files in `codelet/providers/src/custom/`

- `provider.rs` — `RhaiCustomProvider` struct + `LlmProvider` impl (async) + `ModelLimitsResolver` impl.
- `request_bridge.rs` — `fn messages_to_rhai(messages: &[Message]) -> rhai::Array`.
- `response_bridge.rs` — `fn rhai_to_completion_response(value: rhai::Dynamic) -> Result<CompletionResponse, CustomProviderError>`.
- `http_client.rs` — thin wrapper around reqwest::Client used by the provider for POSTs.

All files ≤ 300 lines. Rhai calls run via `tokio::task::spawn_blocking`.
