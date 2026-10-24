# BUG-244 / BUG-245 completion validation

Final verification has completed successfully; see `final-acdd-validation.md`
for the current-tree 56-test result, strict quality gates and independent
26-scenario ACDD audit. The earlier attempts below remain historical evidence.
No commits or manual work-unit edits.

## Repairs and traceability

- Existing archive, delivery, TUI, resume and history-inspector implementations
  were audited against their features and historical executable RED evidence.
- The remaining functional gap was failed initial resume releasing hydration
  suspension. The existing feature scenario was strengthened first; the real
  Cargo test failed on a post-failure polling request. Only then was the response
  handler changed to release suspension on success. Its full 36-test TUI rerun
  passed, including the strengthened scenario.
- Feature/test-file splits preserve all 18 BUG-244 acceptance scenarios, with
  one test file per feature. All eight BUG-244/245 features have 100% mappings
  and successful audit-coverage results, without transition bypass flags.
- Strict Clippy's pre-existing nested-match warning was resolved by an identical
  match guard in the assigned TUI module. Obsolete comments describing the absent
  local context recompute were removed. No cumulative-counter fill fallback was
  introduced, and no unrelated production module was changed.

## Commands

All Cargo commands run from `rust/`, using the existing shared artifacts:

```sh
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_rendered_transcript --test bug244_transcript_delivery \
  --test bug244_replay_state --test bug245_history_access \
  --test session_restore_tui113 --test slash_resume_rpc049 \
  --test cmpct067_coherent_fill --test cmpct067_header_provenance \
  --test cmpct068_committed_projection --no-fail-fast
cargo test --profile release-slim -j2 -p codelet-sessions \
  --test bug245_archive_model_separation --test bug245_cold_empty_failures \
  --test bug250_cold_terminal_display --test bug250_transcript_replay --no-fail-fast
cargo test --profile release-slim -j2 -p codelet-rpc-types --test transcript_fragments
cargo check --profile release-slim -j2 \
  -p codelet-fspec-tui -p codelet-sessions -p codelet-rpc-types --lib
cargo clippy --profile release-slim -j2 \
  -p codelet-fspec-tui -p codelet-sessions -p codelet-rpc-types \
  --lib --no-deps -- -D warnings
```

## Preserved failures

The exploratory eight-target TUI run records all six BUG-244/245 targets and
the four committed-projection tests passing, but exits 101 because the additional
legacy `token_state_realtime_recompute_rpc101` target has 2 pass and 7 fail.
Those expectations conflict with the current `coherent-request-context-fill`
feature: request occupancy must not be recomputed from cumulative display spend.
HEAD's `apply_token_tracker` already lacks that obsolete recompute. The failures
were not caused by the equivalent match guard and are not hidden or relabelled
GREEN. No old feature or test was rewritten outside the assigned cards.

The current authoritative context and provenance tests verify the intended
contract directly. A separate final-attempt exit 101 was an unavailable
datafusion object during archive construction, before tests executed. That
artifact error is not behavioral RED. Both failed logs remain preserved.

Global tag validation has 526 pre-existing violations outside the owned features;
none of the eight owned features has a tag violation. Global feature validation,
scoped rustfmt and staged/unstaged diff checks pass. Newly edited Rust modules
remain below 300 lines. Other dirty/staged work is preserved.
