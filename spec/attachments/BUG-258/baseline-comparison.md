# BUG-258: executed baseline versus final current code

## Scope and provenance

Finalization only: these executions were completed before this evidence update.
No tests, checks, Clippy or builds were rerun to prepare or attach this record.

The isolated baseline copy preserved earlier-card working changes and restored
only these files from pre-BUG-258 HEAD
`2699693b74124cf12b323997fd68dd491adf7e8f`:

- `rust/agent-loop/src/agent_loop.rs`
- `rust/agent-loop/src/background_recovery.rs`
- `rust/agent-loop/src/stream_journal.rs`
- `rust/agent-loop/src/terminal_overflow_recovery.rs`

The original working tree was untouched. The baseline was an executed isolated
comparison, not an inference from untouched fixtures or a pristine whole-tree
checkout. Exact source paths and baseline/current SHA-256 values are preserved
in `baseline-manifest.json`. The native attachments preserve the three source
files byte-for-byte; no baseline source tree is copied into this ticket.

## Full-suite results

| Suite | Isolated baseline | Final current code |
| --- | --- | --- |
| `bug211_compaction_budget_recovery` | 4 passed, 4 failed | 4 passed, same 4 failed |
| `cmpct054_terminal_dependencies` | 2 passed, 1 failed | 2 passed, same 1 failed |
| `bug211_explicit_retry_safety` | Not part of isolated run | 2 passed, 0 failed |

Both commands exited **101**, expected because they include the disclosed
failures. The final current-code log records **15,076,065,280 free bytes** after
the run, above its **6,442,450,944-byte** disk floor.

## Exact failure mapping

All four budget failures assert at
`agent-loop/tests/support/bug210_completion_metrics.rs:110:9` with
`assertion left == right failed`, **left: 11371**, **right: 90336**, on both
baseline and final current code:

- `scenario_streaming_synthesis_preserves_the_final_original_evidence_page`
- `scenario_exhausted_exploration_can_finish_a_summary_without_further_tools`
- `scenario_invalid_synthesized_summaries_are_never_installed`
- `scenario_provider_failures_do_not_trigger_budget_synthesis`

The fifth failure is
`scenario_terminal_recovery_invokes_the_turn_builder_before_cleanup`, at
`agent-loop/tests/cmpct054_terminal_dependencies.rs:92:5` on both versions:

```text
builder failure: terminal compaction failed: compaction builder failed: invalid work_unit_id attribution: unknown invoking context <per-run UUID>. Supply top-level work_unit_id: "an existing card in the invoking project, or NONE"; business session/project targets cannot select the invoking context
```

Context UUIDs, thread IDs, timing and execution paths vary; failure names,
assertion sites and substantive diagnostics match. All five failures therefore
predate BUG-258 in this isolated comparison. None is reclassified as passing.

## Repaired diagnostic control and limits

`scenario_repeated_builder_failures_wait_for_fresh_user_input` passes on both
the baseline and final full current-code budget suite. Its intervening failure
was a separate BUG-258 live-diagnostic regression, repaired through the seventh
acceptance scenario; it is not included among the five preexisting failures.
No introduced failures remain in these compared suites.

Existing evidence records seven green acceptance cases, 80 distinct scoped test
passes and successful scoped quality checks. Existing unrelated global
specification tag/format warnings remain disclosed. This is not a whole-workspace
all-green claim. Changes remain uncommitted; no commit or push is performed.

## Native evidence attachments

- `bug258-baseline-tests.log`: exact executed isolated-baseline output.
- `bug258-baseline-run.json`: exact command, working directory, target,
  expected exit 101 and free-space metadata.
- `baseline-manifest.json`: exact isolated restoration manifest and hashes.
- `baseline-classification-current.log`: completed final current-code comparison.
- `acdd-validation.md` and `review-findings.md`: retained ACDD and review record,
  updated with the executed classification rather than untouched-only caveats.
