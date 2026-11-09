# PROV-172 root cause: limits too large for the wire are clamped or truncated

## Evidence

- `rust/sessions/src/handle_impl/handle_impl_5.rs:64` (`list_providers`) and
  `handle_impl_8.rs:53` (`refresh_models_cache` for a custom provider) convert a custom model's
  declared `context_window: Option<usize>` to the wire `ModelEntry.context_window: Option<u32>`
  with `u32::try_from(window).unwrap_or(u32::MAX)`: a declared 5000000000 is published as
  4294967295, a limit nobody declared.
- Worse, `rust/sessions/src/model_resolution.rs:242-250` (`resolve_current_model_limits`)
  narrows `ProviderManager::context_window()/max_output_tokens()` (usize) and the compaction
  threshold (u64) with `as u32`, which wraps: an `OPENAI_CONTEXT_WINDOW=5000000000` override
  (accepted by PROV-171's positive-integer parser) becomes 705032704 in the session cache, the
  wire and compaction maths.

## Should the wire be widened instead?

No. Catalog limits are `u32` at their source (`models.dev` `LimitInfo { context: u32, output:
u32 }`, rust/providers/src/models/types.rs:159-164), so no catalogued model can exceed the wire.
Only a user-declared value (custom provider JSON, profile, env override) can, and 4294967295
tokens is far beyond any real model; such a value is a configuration error, not a capacity the
wire must carry. Widening `ModelEntry`/`SessionModel`/`ModelInfo` would be a wire break for every
bincode client for no real model.

## Fix (chosen)

- `codelet_providers::custom::ProviderModelInfo.context_window / max_output_tokens` become
  `Option<u32>`; `list_providers_info` converts each declared value with `u32::try_from`, and a
  value that does not fit fails the listing with the new
  `CustomProviderError::LimitExceedsWire { provider, field, value }` naming
  `models.<alias>.<limit>`. `list_providers` / `refresh_models_cache` copy the u32 through.
- `resolve_current_model_limits` returns `Result<ResolvedModelLimits, WireLimitError>`; each limit
  is converted with a checked conversion and an out-of-range one is
  `WireLimitError { limit, model, value }`. `apply_model_selection` and
  `refresh_runtime_settings_for_turn` propagate it as their existing `String` errors, so session
  creation, a model switch and a turn fail with the named error.

## Alternatives rejected

- Keep clamping: publishes a false limit (the bug).
- Reject at custom-config load: it would not cover env/profile overrides, and the wire is where
  the width constraint actually exists.
- `list_providers` still turns any listing error into an empty list (pre-existing; RPC-451).

## Wire / API changes

None on the RPC wire (`ModelEntry` is unchanged: id, display_name, context_window: Option<u32>,
supports_reasoning, supports_vision, is_custom). Rust API: `ProviderModelInfo` limits are
`Option<u32>`; `CustomProviderError::LimitExceedsWire`; `resolve_current_model_limits` returns a
`Result`.
