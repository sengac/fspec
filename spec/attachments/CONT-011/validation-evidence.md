# Auto-continue audit validation evidence

This is the historical discovery baseline, not final repair validation. See
[final validation and review](validation-and-review.md) for retained causal
failures, actual final-source outcomes, fixture corrections and closure limits.

Baseline: `f0eb0759`, 2026-10-25. Production code was not changed during the audit.
The commands below were executed, not merely suggested.

## Scoped Cargo runs

From `rust/`, using `--profile release-slim` and `-j 4`:

| Package | Integration target | Result |
| --- | --- | --- |
| codelet-cli | auto_continue_engine_test | 10 passed, 1 failed |
| codelet-cli | goal_enforcement_test | 18 passed |
| codelet-cli | cont006_goal_immediate_termination | 6 passed, 1 failed |
| codelet-providers | codex_terminal_regressions | 7 passed |
| codelet-providers | prov148_codex_completed_answers | 1 passed |
| codelet-providers | prov149_codex_history | 2 passed |

Totals: 44 passed, 2 failed. Both failures are stale source-location assertions:

1. `done_is_registered_only_while_auto_continue_is_armed` reads Codex `mod.rs`,
   although registration is now in `rust/providers/src/codex/agent.rs:99`.
2. `stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants`
   searches for `clear_goal` in session `mod.rs`; it moved to `reminders.rs:15`.

Representative command:

```sh
cargo test --profile release-slim -p codelet-cli \
  --test goal_enforcement_test --test cont006_goal_immediate_termination \
  --no-fail-fast -j 4
cargo test --profile release-slim -p codelet-providers \
  --test prov148_codex_completed_answers --test prov149_codex_history \
  --test codex_terminal_regressions --no-fail-fast -j 4
```

## Independent production-library probe

A temporary Rust test executable linked to the compiled `codelet-providers`
library exercised the actual public `CodexHistory::commit` and `prepare` APIs.
Both diagnostic assertions passed: ordinary output_text permits a follow-up;
native refusal flattened to the same visible text rejects the follow-up with
an ancestry error. The full minimal reproduction is preserved on
[BUG-260](../BUG-260/codex-refusal-history.md).

This establishes a Codex history defect, not a universal failure of every
auto-continue path or the root cause of a specific unprovided user trace.

## Evidence limitations

Other findings are source-confirmed unless their focused attachment says
otherwise. No live provider credentials or user-installed Rhai scripts were
used. Passing existing Codex single-response tests does not cover the failing
refusal round trip. Raw audit logs were temporary; this durable Markdown
captures commands, exact targets, observed counts and failure classification.
