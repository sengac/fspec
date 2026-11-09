# BUG-357 root cause (draft)

`update-work-unit-status` (fspec-core/src/commands/update_work_unit_status/step_docstrings.rs, VAL-005) refuses any feature whose coverage `testMappings` name more than one distinct file. The check only runs for the features tagged with the work unit that is moving, so done features that break 1:1 go unnoticed until a later card tags them. BUG-355 hit this when BUG-351 tagged four of them.

There are 157 such features (multi-file-features.txt), mostly from older include!/directory test splits and from scenarios tested across crates. Nothing project-wide reports them: `audit-coverage` checks only that the files exist.

Fix direction:
1. Add an audit-coverage check that lists multi-file features.
2. Fix each feature: consolidate its tests into one test file with tests/<name>_support/ modules, or split the feature.

## Decision (2026-11-04, product owner)

"One test file maps to one feature." The rule constrains test files, not features: a test
file must not map to more than one feature. A feature may be covered by several test files.
The VAL-005 gate currently checks the opposite direction (refuses a feature with several
test files); it must instead refuse a test file that is mapped to more than one feature.
