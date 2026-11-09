# BUG-374 root cause

Seen in TUI-116 (2026-11-02): `fspec delete-scenario spec/features/exec-stdin-prompt.feature "<name>"` for the seven
superseded TUI scenarios updated `totalScenarios` (18 → 11) but left `stats.testFiles` listing
`rust/fspec-tui/tests/exec_stdin_prompt_p2.rs` and `stats.implFiles` listing five fspec-tui files that no remaining
scenario mapped. Once the test file was deleted, the stats pointed at a file that no longer exists. TUI-116 recomputed
that one file's lists by hand.

## Why
- `rust/fspec-core/src/commands/delete_scenario.rs` (around line 270) builds the new stats as
  `{ ...existingStats, totalScenarios, coveredScenarios, coveragePercent }`. `testFiles` and `implFiles` come from
  the spread of the old stats, so they are never recomputed.
- `rust/fspec-core/src/commands/delete_scenarios.rs` (`update_coverage_after_delete`, around line 330-376) does the
  same: it inserts the three counters into the old stats map. It also "silently no-ops on any error", which hides
  coverage write failures.
- `rust/fspec-core/src/types/coverage.rs::calculate_stats` already computes every stat (including the deduplicated
  file lists) from the scenarios array; neither command uses it.

## Fix
Both commands recompute the full stats from the remaining scenarios with `calculate_stats` (keeping unknown extra
keys), and `delete-scenarios` reports a coverage update failure instead of dropping it. Related: BUG-346 (link and
unlink stats, plus an audit check that would also catch this).
