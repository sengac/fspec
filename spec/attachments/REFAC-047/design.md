# REFAC-047 design

`rust/fspec-core/src/commands/update_work_unit_status/step_docstrings.rs` was 753 lines and BUG-360 must change
`matching_features`. Split by responsibility into child modules (no behaviour change):

- `step_docstrings.rs` (187): module docs and `validate_test_step_docstrings`.
- `step_docstrings/matching.rs` (106): `MatchedFeature`, `MatchedScenario`, `matching_features`, `is_work_unit_tag`
  and its unit test.
- `step_docstrings/steps.rs` (244): `@step` extraction, matching, adaptive threshold, missing-step reminder and
  their unit tests.
- `step_docstrings/similarity.rs` (241): the hybrid similarity port.

Items crossing module lines became `pub(super)`. The moved unit-test modules drop the old
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, which nothing in them needed. A line-multiset
comparison against the original confirmed every code line is kept. No coverage file references the old path.
