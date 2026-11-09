# BUG-330 root cause: Codex model-limit tests expect a 272k default Codex no longer has

## Reproduction

`cargo test --profile ci-test -p codelet-providers --lib` fails:

- `model_limits::provider_resolver_tests::codex_resolver_returns_correct_defaults_and_suppresses_max_output_tokens`
  — `assert_eq!(resolver.default_context_window(), 272_000)` (left 0).
- `model_limits::integration_all_providers::codex_falls_back_to_272k_default`
  — `assert_eq!(resolver.default_context_window(), 272_000)` (left 0).

## Evidence

1. Commit 2699693b changed `CodexProvider`'s `ModelLimitsResolver` defaults from
   `CONTEXT_WINDOW` / `MAX_OUTPUT_TOKENS` to `0`
   (`rust/providers/src/codex/mod.rs:128-135`), and `ProviderManager`'s Codex
   resolver stub to `default_ctx: 0, default_out: 0`
   (`rust/providers/src/manager.rs:1100-1105`).
2. That is the deliberate policy of `spec/features/codex-catalog-limit-resolution.feature`
   ("Missing metadata must stay missing rather than introducing guessed limits";
   an uncatalogued Codex selection reports limits 0/0,
   `rust/sessions/tests/codex_catalog_limit_resolution.rs:93-119`) and of BUG-309's
   rule "Codex has no static default limits; without catalog or user values its
   context window and output limit are unavailable (0)". BUG-309 updated
   `manager::tests` and `integration_all_providers::provider_manager_full_chain_all_providers`
   (`model_limits.rs:931-942`) but missed the two resolver tests above, which still
   assert the old guessed 272k / 4096 default.
3. The guessed limits still exist as live code: `codex::CONTEXT_WINDOW = 272_000` and
   `codex::MAX_OUTPUT_TOKENS = 4096` (`codex/mod.rs:59-63`) are returned by
   `impl LlmProvider for CodexProvider` (`codex/completion.rs:62-67`), so anything
   asking the provider object for its window silently gets a guessed 272k — the
   fallback the no-default policy forbids. `manager.rs:2154-2155` asserts the
   constant still equals 272000 (MODEL-005's "constants remain unchanged").

So the product's resolution chain is right; the two tests (and their scenarios) are
stale, and the leftover guessed constants are a remaining silent default.

## Fix

- Codex declares no static limits anywhere: remove `codex::CONTEXT_WINDOW` and
  `codex::MAX_OUTPUT_TOKENS`; `CodexProvider`'s `LlmProvider::context_window` /
  `max_output_tokens` report 0 — the established "unavailable" encoding shared with
  its `ModelLimitsResolver` and `ProviderManager` (BUG-309) — instead of a guessed
  272k / 4096. The real limits come only from the model catalog or a user value
  through `ProviderManager`.
- Replace the two stale tests and scenarios with a dedicated feature,
  `codex-declares-no-static-model-limits.feature`, tested in one integration test file:
  the resolver declares no default and suppresses `max_output_tokens`; resolution
  without registry or user values yields 0/0; registry values pass through unclamped;
  the provider object reports no guessed window.
- Remove the stale Codex scenarios from `provider-model-limits-resolution.feature`
  and `integration-tests-all-provider-model-combinations.feature`, the
  `codex::CONTEXT_WINDOW` step from MODEL-005's constants scenario, and correct the
  architecture comments that still claim a 272k Codex default.
- `providers/src/manager.rs` (2886 lines) and `providers/src/model_limits.rs`
  (1136 lines) must be touched, so they are split under 300 lines first by a separate
  refactor card.

## Alternatives rejected

- Restoring the 272k default in the resolver to make the tests pass: reintroduces the
  guessed limit that codex-catalog-limit-resolution and BUG-309 removed (an ordinary
  uncatalogued model would silently get a window it may not have).
- Making `ModelLimitsResolver::default_*` and `ProviderManager::context_window()`
  return `Option`/`Result` so "unavailable" is a type instead of 0: the right long-term
  shape, but it changes every provider resolver and ~180 call sites of
  `context_window()` / `max_output_tokens()` across six crates (and the u32 wire fields
  that carry 0 for unavailable). Out of scope for a stale-test bug; recorded as a
  follow-up.
- Keeping the constants but not using them: dead guessed values documented as the
  Codex window would keep misleading readers.
