# BUG-403 AST research

- `grep -n "serde(default)" rust/rpc-types/src/session_state.rs` around lines 122-173: `HitlQuestion.options`
  (line 139-140) and `HitlResponse.answers` (line 170-171).
- `fspec astgrep --lang rust --pattern '$A.unwrap_or($B.len())' --path rust/sessions/tests/rpc410_hitl_wire_protocol_static_checks.rs`
  finds the send-path body end fallback (inside `send_path_contains_no_option_label_comparison`).
- `send_hitl_response` and `get_pause_state` are adjacent in `rust/sessions/src/handle_impl/handle_impl_4.rs`
  (lines 5 and 26), so the marker is currently found.
