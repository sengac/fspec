# BUG-378 root cause

`rust/fspec-core/src/commands/update_work_unit_status.rs:553` `linked_feature_files` (used by `check_prefill`, the
prefill gate that blocks every forward transition):

- `if let Ok(entries) = std::fs::read_dir(&dir)`: an unreadable `spec/features` (or one that is a file) yields no
  features, so the gate passes.
- `entries.flatten()` drops directory entry errors.
- `if let Ok(content) = std::fs::read_to_string(&path)`: an unreadable or non-UTF-8 feature file is skipped, so a
  prefilled feature that cannot be read never blocks.

Same pattern BUG-360 fixed in `step_docstrings/matching.rs`. Fix direction: return
`Result<Vec<(PathBuf, String)>, _>` with errors naming the path (NotFound on `spec/features` = no features, as
decided in BUG-360), and propagate from `check_prefill`. `update_work_unit_status.rs` is 1445 lines, so split it
under a REFAC card first.

## Fix (2026-11-03)
After REFAC-048 split the command, `linked_feature_files` (`update_work_unit_status/prefill.rs`) returns
`Result<_, FeatureScanError>` (BUG-360's typed error, re-exported from `step_docstrings`): NotFound on
`spec/features` is "no feature files", every other read_dir, entry and file read error names the path. Its callers
(`check_prefill`, `check_scenarios_exist` and the bug linked-feature check in review.rs, `check_temporal_ordering`)
fail the status move with it. Red: both scenarios moved BUG-1 to specifying; green with the BUG-360, COV-056,
status-command and CLI regressions. Filed BUG-384 (only the first linked feature is prefill-checked) and BUG-386
(temporal gate skips unreadable test files and mtimes).
