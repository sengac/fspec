# BUG-403 root cause

1. `rust/rpc-types/src/session_state.rs`: `HitlQuestion.options` (line 140) and `HitlResponse.answers` (line 171) are
   `#[serde(default)]`. A wire payload that omits the field deserializes as an empty list, so a malformed frontend
   message turns into "pure freeform question" or "submit with no answers" instead of an error. This is a silent
   default on input (no-fallbacks rule). Removing the attribute changes the wire contract, so check every producer
   (fspec-tui transports, websocket, napi) always sends the field, and run `refac019_contracts`,
   `board024_additive_api_contract` and the wire-order fixture test.
2. `rust/sessions/tests/rpc410_hitl_wire_protocol_static_checks.rs`, `send_path_contains_no_option_label_comparison`:
   `.find("fn get_pause_state(").map(..).unwrap_or(handle_src.len())` falls back to the rest of the composed source
   when the end marker is missing, so the check silently inspects a different body. It should fail with an
   `expect` naming the missing marker.

Found during REFAC-052 / BUG-397.
