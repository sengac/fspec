# PROV-171 root cause: OpenAI limit env overrides fall back to guessed constants

## Problem (evidence at f167f1f9, after REFAC-031)

- `rust/providers/src/openai.rs:23,27` defined `DEFAULT_CONTEXT_WINDOW = 128_000` and
  `DEFAULT_MAX_OUTPUT_TOKENS = 4096`, re-exported as `pub const CONTEXT_WINDOW` /
  `MAX_OUTPUT_TOKENS` (`:66,69`). Both are guesses: an OpenAI-compatible endpoint can be
  any local server (vLLM, Ollama, LM Studio) whose model has an unrelated limit.
- `rust/providers/src/openai/construction.rs:100-108` (`from_api_key_with_options`):
  `std::env::var("OPENAI_CONTEXT_WINDOW").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(DEFAULT_CONTEXT_WINDOW)`
  and the same for `OPENAI_MAX_OUTPUT_TOKENS`. An unset variable became the guess, and a
  set but invalid value (`"not_a_number"`, `"32k"`, `""`) silently became the guess too.
  `"0"` parsed and became a 0-token limit.
- `rust/providers/src/manager/impl_4.rs:153-169` (`provider_limits_resolver`, OpenAI arm)
  repeated the same parse with `.unwrap_or(openai::CONTEXT_WINDOW)` /
  `.unwrap_or(openai::MAX_OUTPUT_TOKENS)`, so `ProviderManager::context_window()` /
  `max_output_tokens()` reported `Some(128000)` / `Some(4096)` for every OpenAI selection
  with no catalog or user value — compaction, admission and the request `max_tokens` were
  budgeted against invented numbers.
- `rust/providers/src/openai/llm_provider.rs:33-41,87-93`: the provider's resolver and
  `LlmProvider` limits returned `Some(self.context_window)` / `Some(self.max_output_tokens)`.
- Tests pinned the fallback: `rust/providers/tests/stop_reason_propagation_test.rs:272-286`
  asserted `"not_a_number"` -> 4096; `prov164_profile_output_limit.rs:40-52` asserted the
  4096 default; `manager/tests_2.rs:187`, `tests_3.rs:61,80`, `tests_4.rs:39` asserted the
  constants.

Who sets the variables: `rust/sessions/src/profile_environment.rs:27-29` exports a
profile's `contextWindow` as `OPENAI_CONTEXT_WINDOW` before
`set_model_direct_with_profile` (`rust/sessions/src/profile_model_limits.rs:40-43`);
otherwise they come from the user's shell. `OPENAI_MAX_OUTPUT_TOKENS` is only ever
user-set.

## Fix

- New `rust/providers/src/openai/env_limits.rs`: `OpenAIEnvLimits { context_window:
  Option<usize>, max_output_tokens: Option<usize> }` with `from_env()`. Unset -> `None`
  (not overridden). Set -> must parse as `usize` and be > 0, otherwise
  `ProviderError::InvalidLimitOverride { provider: "openai", variable, value }`
  ("[openai] Invalid OPENAI_MAX_OUTPUT_TOKENS: \"not_a_number\" is not a positive integer").
  One parser shared by the provider and the manager.
- `DEFAULT_*`, `pub const CONTEXT_WINDOW` and `pub const MAX_OUTPUT_TOKENS` are deleted.
- `OpenAIProvider.context_window` / `max_output_tokens` become `Option<usize>`; the
  constructors return the error; the resolver and `LlmProvider` report the `Option`.
  `create_rig_agent` / `complete_with_tools` send `max_tokens` only when a limit is known.
  Production always builds the provider through `ProviderManager::get_openai`, which
  already refuses construction when max output tokens are unavailable (PROV-165) and sets
  the resolved value with `with_max_output_tokens`.
- `ProviderManager` gains `openai_env_limits: OpenAIEnvLimits`, captured (fallibly) for an
  OpenAI selection in `new`, `with_provider` (so `with_provider_and_model`),
  `select_model`, and `refresh_runtime_options` (`set_model_direct*`, `switch_provider`) —
  the same points that snapshot the catalog limits. The OpenAI arm of
  `provider_limits_resolver` uses the snapshot as its defaults, no constants. For any
  other provider the snapshot is empty and the variables are not read.
- Error surface, with evidence: session creation and mid-session model switch both go
  through `codelet_sessions::model_resolution::apply_model_selection`, which calls
  `select_model` / `set_model_direct_with_profile`; their error is returned as
  "Failed to set profile model: [openai] Invalid OPENAI_MAX_OUTPUT_TOKENS: ..." and the
  session creation / `set_model` fails. A turn builds the provider through `get_openai`,
  whose constructor re-validates the variables.
- Unset everywhere and no catalog/profile/user limit: the limits are unavailable (`None`),
  `get_openai` fails "max output tokens unavailable for openai/<model>" and the prompt
  preflight reports the unavailable limit (PROV-165..170 contract kept).

## Alternatives rejected

- Live env read in the manager resolver (old behaviour): `context_window()` /
  `max_output_tokens()` are infallible with 23 production callers, so an invalid value
  would have to become `None` or a default there — a silent fallback. Snapshotting at
  selection gives one fallible point, matching how catalog limits are captured.
- Making `ProviderManager::context_window()` return `Result`: ripples through 23
  production callers for a value that can be validated once at selection.
- Warn-and-ignore an invalid value: forbidden (no fallbacks).
- Making `create_rig_agent` fallible when no output limit is known: changes the shared
  dispatch macro contract for every provider; the only production construction path
  (`get_openai`) already refuses an unavailable output limit.

## API changes

- `ProviderError::InvalidLimitOverride { provider: String, variable: &'static str,
  value: String }` (new variant, not retryable).
- `codelet_providers::openai::OpenAIEnvLimits` (new); `openai::CONTEXT_WINDOW` /
  `openai::MAX_OUTPUT_TOKENS` removed.
- `ProviderManager::for_testing` captures no env overrides (tests that need them select
  a model or use `with_provider`). No RPC/wire type changes.

Related: BUG-341 (a profile without contextWindow inherits the previous profile's
`OPENAI_CONTEXT_WINDOW`).

## Test changes outside the linked feature

- `rust/providers/tests/session_affinity_integration_test.rs` (303 lines) is split: the
  cached-token tests move verbatim to `session_affinity_integration/cached_tokens.rs`
  (`#[path]` submodule). Its two `get_openai` tests relied on the 4096 guess and now
  declare an output limit (`Some(8_192)`) through `set_model_direct`.
- Pre-existing failures found while running regressions (reproduced with this change
  stashed): BUG-342 (prov147_codex_transport), BUG-343 (prov141 session creation).
