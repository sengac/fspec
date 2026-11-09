# REFAC-053 design

`spec/features/isolation-path-validation-callback-is-never-registered-in-the-fspec-binary-isolated-sessions-are-not-actually-pinned-to-their-worktree.feature`
(WT-004, 12 scenarios) had its tests in two `#[path]` fragments of `rust/sessions/tests/wt004_session_tool_callbacks.rs`
(`wt004_session_tool_callbacks/isolated_session_under_the.rs`, `.../work_unit_stage_lookup.rs`). fspec counts two
test files for the feature and refuses BUG-398's move to implementing. Together the tests are well over 300 lines, so
the feature is split, with no behaviour change:

- Shared helpers (`MODELS_FIXTURE`, `ENV_GUARD`, `set_dummy_credentials`, `run_git`, `fresh_git_repo`, `SHARED_DIR`,
  `shared_dir`, `manager_with_seeded_cache`, `register_tool_callbacks`, `create_plain`) move to
  `rust/sessions/tests/wt004_session_tool_callbacks_support/mod.rs`; every public one is used by both test files.
- `wt004_session_tool_callbacks.rs` keeps the seven isolation-lookup scenarios (the old `isolated_session_under_the.rs`
  body) plus `create_isolated`, used only there (284 lines).
- New feature `spec/features/wt004-tool-callbacks-wiring.feature` (@wt-004 @REFAC-053) takes "Work unit stage lookup
  returns None ...", "A blocked tool action emits ...", "build_service registers ...", "The NAPI front door registers
  ..." and "The NAPI isolation lookup falls back ..." unchanged, with test file
  `rust/sessions/tests/wt004_tool_callbacks_wiring.rs` (the old `work_unit_stage_lookup.rs` body plus
  `register_chunk_sender`, `next_matching_chunk`, `rust_root`, `strip_line_comments`, `build_service_body`; 290 lines).
- Coverage moves with the scenarios; test ranges recomputed. "The NAPI front door registers ..." had a mapping to
  lines 186-245 of a 195-line fragment with no test behind it (there is no NAPI crate any more). fspec refuses `done`
  with an uncovered scenario and it cannot be tested, so REFAC-053 deletes it (`fspec delete-scenario`).

Verification: `cargo test --profile ci-test -p codelet-sessions --test wt004_session_tool_callbacks --test
wt004_tool_callbacks_wiring` (12 tests, the NAPI-fallback one still order-dependent as at HEAD: BUG-398), clippy on
both targets.
