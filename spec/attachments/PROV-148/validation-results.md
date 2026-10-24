# PROV-148 validation

## Test-first evidence

The initial local-server integration run compiled and produced nine failures and
one passing fallback control. Failures demonstrated hanging after completion,
silent EOF success, lost errors and incomplete usage, duplicate usage, missing
continuation, and malformed terminals becoming success.

A subsequent backend-error case containing `aborted` failed because the shared
rig stream wrapper suppresses ProviderError values containing that text. Codex
terminal failures now use ResponseError, preserving those messages without
changing shared cancellation behavior.

## Passing checks

- `cargo test -p codelet-providers --test prov148_codex_lifecycle --test prov147_codex_transport -j 2`: 10 lifecycle scenarios and 3 existing boundary controls pass.
- `cargo check -p codelet-providers -j 2`: passes.
- `CARGO_INCREMENTAL=0 cargo clippy -p codelet-providers --test prov148_codex_lifecycle -j 2 --no-deps`: passes with existing tools and unused shared-fixture warnings.
- Feature syntax validation and coverage audit pass; all ten scenarios map to integration tests and wired implementation.
- All owned Rust files remain below 300 lines.

## Policy and integration

CodexResponsesModel.stream calls the adapter-owned streaming implementation.
It closes and drops its SSE source before yielding terminal usage or tools.
Text deltas remain visible during interruption; incomplete and failed responses
never emit executable ToolCall events or FinalResponse. Completed tool calls
return to the existing generic loop. Absent end_turn preserves that fallback.
Explicit false text-only completion and interrupted incomplete text permit at
most three follow-up requests, replaying only assistant message output.
Transport failures never reconnect or automatically replay side effects.

PROV-152 owns request-option normalization; model.rs integration ownership was
offered to that worker after lifecycle wiring passed the boundary controls.

## Repository-wide limitations

Global validate-tags reports 529 unrelated feature violations; this feature has
registered component and feature-group tags. Global diff checking finds trailing
whitespace in unrelated rlcd-blocklist-security.feature. Neither was modified.
Disk exhaustion interrupted a patch and a later clippy run; the test file was
restored and tests rerun. Disposable Cargo incremental caches were cleared to
recover space, then clippy passed with incremental compilation disabled.
