# PROV-170 root cause: custom provider models guess a 200000/4096 limit

## Problem

`rust/providers/src/custom/config.rs:43-50` defined serde defaults for
`ModelDef` (`:79-84`):

- `default_context_window()` -> `200_000` (BUG-139 raised it from 128 000)
- `default_max_output_tokens()` -> `4096`

A custom provider JSON whose model omits `context_window` / `max_output_tokens`
therefore deserialized to invented numbers. The guess then reached:

1. `RhaiCustomProvider::new` (`rust/providers/src/custom/provider.rs:104-125`):
   `overrides.context_window.unwrap_or(json_context_window)` — the provider's
   `LlmProvider::context_window()` / `max_output_tokens()` (`:228-234`) always
   reported `Some(guess)`. It also ignored an explicitly declared
   `defaults.max_output_tokens`, unlike the manager path.
2. `list_providers_info` (`rust/providers/src/custom/management.rs:195-205`):
   `ProviderModelInfo.context_window/max_output_tokens: usize` (`:45-48`) copied
   the guess.
3. The session wire: `handle_impl_5.rs:61-63` (list_providers) and
   `handle_impl_8.rs:50` (refresh_models_cache) put `Some(200000)` into
   `ModelEntry.context_window` for every custom model that declared nothing, so
   the TUI showed a context window that nobody declared.

The ProviderManager / agent turn path was NOT affected: it reads the raw JSON
(`rust/providers/src/selection_metadata.rs:70-72`, context window, else model
max output, else `defaults.max_output_tokens`, all `Option`) and the prompt
preflight already refuses an unavailable limit, naming it
(`rust/cli/src/model_limits.rs` `required_limits` -> `ModelLimitUnavailable`,
"max output tokens unavailable for <model>: ..."). A custom provider without a
declared max output therefore already fails the turn explicitly; only the
listing/provider-object surfaces lied.

## Fix

- `ModelDef.context_window` / `max_output_tokens` become `Option<usize>` with
  `#[serde(default, skip_serializing_if = "Option::is_none")]`; both default
  functions are deleted.
- `RhaiCustomProvider` stores `Option<usize>` limits: script override, else the
  JSON model value, else (output only) the declared `defaults.max_output_tokens`
  — the same precedence `selection_metadata` uses. Undeclared -> `None` from
  `context_window()` / `max_output_tokens()`. `script_context_window()` /
  `script_max_output_tokens()` are unchanged in behaviour.
- `ProviderModelInfo.context_window` / `max_output_tokens` become `Option<usize>`
  (raw model declaration, as before; field order unchanged).
- `handle_impl_5` / `handle_impl_8` map `None` to `None`
  (`m.context_window.map(...)`). The existing `u32` clamp of a declared value is
  left as is (PROV-172).
- Zero limits: `ProviderConfig::validate` rejects a declared `0` for
  `models.<alias>.context_window`, `models.<alias>.max_output_tokens` and
  `defaults.max_output_tokens` with the new
  `CustomProviderError::NonPositiveLimit { provider, field, path }`.
  Evidence that 0 would otherwise be treated as a value: `list_providers_info`
  -> `handle_impl_5` would send `Some(0)` on the wire as the model's context
  window, and `RhaiCustomProvider::context_window()` would return `Some(0)`.
  The other layers already refuse 0 (`runtime_options.rs:145` "Token overrides
  must be positive", `model_limits.rs` `parse_positive_usize` for script values,
  `request_admission.rs` `validate`), so load-time rejection makes the config
  layer consistent instead of letting a 0 travel to the selector.

## Alternatives rejected

- Keep `usize` and use `0` as "unknown": a sentinel, forbidden, and exactly the
  value the request layer rejects.
- Fill an undeclared model output limit from the models.dev catalog in
  `RhaiCustomProvider`: catalog resolution belongs to `ProviderManager`, which
  already does it; duplicating it would create a second source of truth.
- Clamp in the wire mapping: out of scope (PROV-172) and no new clamps.

## API / wire changes

- `codelet_providers::custom::ModelDef`: `context_window: Option<usize>`,
  `max_output_tokens: Option<usize>` (field order unchanged: id, context_window,
  max_output_tokens, supports_tools, supports_streaming, supports_thinking,
  supports_vision). Serialized JSON omits undeclared limits.
- `codelet_providers::custom::ProviderModelInfo`: `context_window:
  Option<usize>`, `max_output_tokens: Option<usize>` (field order unchanged: id,
  context_window, max_output_tokens, supports_tools, supports_streaming,
  supports_thinking, supports_vision).
- New `CustomProviderError::NonPositiveLimit { provider, field, path }`.
- Wire `codelet_rpc_types::ModelEntry` is unchanged (`context_window:
  Option<u32>` since PROV-169); custom models that declare no window now send
  `None`.
