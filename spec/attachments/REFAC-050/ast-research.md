# REFAC-050 AST research

- `grep -n 'include!' rust/fspec-tui/tests/pause_hitl_rpc053.rs` → line 298 includes
  `pause_hitl_rpc053/errors_and_source_shape.rs` (tests `get_hitl_request_error_sets_no_slot`,
  `send_hitl_response_error_still_clears_slot_and_emits_no_notice`, `dispatch_pause_hitl_hosts_the_pause_hitl_helpers`).
- Helper fns at lines 30-121 of `pause_hitl_rpc053.rs`; five `StreamChunk::SessionStateChange` dispatch blocks and
  four `pause_state_for(..).is_none()` assertions repeat across tests.
- Coverage: `spec/features/pause-and-hitl-dialogs.feature.coverage` maps both files as test files.
