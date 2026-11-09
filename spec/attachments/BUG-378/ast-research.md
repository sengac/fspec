# BUG-378 AST research

`fspec astgrep --pattern 'if let Ok($X) = std::fs::read_dir($D) { $$$ }' --lang rust --path rust/fspec-core/src/commands`
includes `update_work_unit_status.rs:557` (linked_feature_files). Its only caller is `check_prefill` (:638), called
for every forward transition except to blocked (:191).

Correction (2026-11-03, after REFAC-048): `linked_feature_files` now lives in
`update_work_unit_status/prefill.rs` and has three callers, not one: `check_prefill` (prefill.rs),
`check_scenarios_exist` and the bug linked-feature check in `check_testing_entry` (review.rs), and
`check_temporal_ordering` (temporal.rs). All three must propagate the scan error. BUG-360's typed
`FeatureScanError` (`step_docstrings/matching.rs`) already names each failing path and is reused.
