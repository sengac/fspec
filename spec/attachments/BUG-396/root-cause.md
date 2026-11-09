# BUG-396 root cause

`rust/sessions/tests/rpc425_session_creation_refactor.rs::model_limits_and_thinking_level_are_set` (line ~182) asserts
that `rust/sessions/src/session_creation_helper.rs` contains `resolved.compaction_threshold`. PROV-165 (3ef07f2c5,
"unavailable model limits are None, not 0") replaced the per-field limit setters with one
`session.set_model_limits(resolved)` call (helper line ~240) and updated the helper's own inline test (which now
checks `set_model_limits(resolved)`), but not this integration test. The test has failed since that commit.

Fix: assert on `set_model_limits(resolved)` like the inline test, and keep the scenario's step text accurate.
