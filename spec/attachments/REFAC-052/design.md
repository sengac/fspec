# REFAC-052 design

`rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs` (498 lines) is the one test file of
`spec/features/hitl-wire-protocol-parity.feature` (7 scenarios). BUG-397 has to change how its tests set up their
session, so it is split first, with no behaviour change:

- Shared helpers (`fresh_session`, `option_question`, `freeform_question`, `three_question_request`,
  `yes_no_request`, `spawn_blocked_waiter`, `join_waiter`) move to
  `rust/sessions/tests/rpc410_hitl_wire_protocol_parity_support/mod.rs`. Only the parity file uses them, and it uses
  every one, so no dead-code suppression is needed.
- The two scenarios that need no session ("The send path contains no option-label comparison", "New wire shapes
  round-trip through serde_json") move unchanged to the new feature
  `spec/features/hitl-wire-protocol-parity-static-checks.feature` (@RPC-410 @REFAC-052, same user story) with its own
  test file `rust/sessions/tests/rpc410_hitl_wire_protocol_static_checks.rs` (with `round_trip` and `wire_option`).
- The five session scenarios stay in `rpc410_hitl_wire_protocol_parity.rs` (275 lines).
- Coverage: test ranges in both `.coverage` files are recomputed. Two impl mappings were stale and are corrected:
  the send-path scenario pointed at `handle_impl_3.rs` 109-146 (`toggle_debug`), now `handle_impl_4.rs` 5-25
  (`send_hitl_response`) plus `hitl_mapping.rs` 42-67; the round-trip scenario pointed at `chunk_support.rs` 27-79
  (`ToolResultInfo`), now `rpc-types/src/session_state.rs` 122-173 (the wire Hitl types).

Verification: `cargo test --profile ci-test -p codelet-sessions --test rpc410_hitl_wire_protocol_parity --test
rpc410_hitl_wire_protocol_static_checks` and clippy on both targets. The parity tests still share the BUG-397 data
directory problem; BUG-397 fixes it next.
