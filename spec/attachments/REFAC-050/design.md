# REFAC-050 design

`rust/fspec-tui/tests/pause_hitl_rpc053.rs` is 298 lines and ends with
`include!("pause_hitl_rpc053/errors_and_source_shape.rs")` (132 lines), an `include!`d test fragment. BUG-394 changes
the Paused probe this feature tests, so the file is restructured first, with no behaviour change:

- helpers (`sid`, `pause_state`, `hitl_request`, `key_event`, `wait_until`, `drain_pending`,
  `fresh_app_with_session`, `seed_hitl_slot`, `hitl_slot_set`) move to `tests/pause_hitl_rpc053_support/mod.rs`,
  plus two new ones that remove repetition: `dispatch_state(app, session, state)` (the
  `ChunkReceived(SessionStateChange)` dispatch) and `pause_slot_empty(app, session)`;
- `drain_pending` stops discarding task join results (`let _ = handle.await`) and expects each task to join;
- the fragment's three tests are inlined into `pause_hitl_rpc053.rs`, which stays the feature's one test file, under
  300 lines; the fragment file and the `include!` are deleted.

Verification: `cargo test -p codelet-fspec-tui --test pause_hitl_rpc053` (8 tests) and clippy on that target.
Coverage test ranges for `pause-and-hitl-dialogs.feature` are remapped to the new lines.
