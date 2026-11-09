# AST research: reply entry points for questions and approvals (AMGR-020, 2026-11-07)

fspec astgrep call-site patterns over rust/; each line is file:line:col:match. These are every caller the checked, id-naming reply replaces.

## `$R.send_pause_response($$$A)`
```
rust/cli/tests/stream_loop_pause_test.rs:213:13:session_for_signaler.send_pause_response(PauseResponse::Resumed)
rust/sessions/examples/repro_paused_chunk_delay.rs:120:5:session.send_pause_response(PauseResponse::Denied)
rust/sessions/tests/paused_chunk_delivery_rpc409.rs:188:5:session.send_pause_response(PauseResponse::Denied)
rust/sessions/tests/paused_chunk_delivery_rpc409.rs:333:5:session.send_pause_response(PauseResponse::AllowOnce)
rust/agent-loop/src/agent_loop/bridge.rs:102:21:session_for_control.send_pause_response(pause_resp)

```

## `$R.send_pause_response_checked($$$A)`
```
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:145:25:self
rust/agent-loop/tests/support/bug265.rs:174:25:session

```

## `$R.send_hitl_response($$$A)`
```
rust/rpc/src/mcp_dispatch/group_26.rs:48:9:client
rust/sessions/tests/paused_chunk_delivery_rpc409.rs:292:5:session.send_hitl_response(HitlResponse::Cancelled { cancelled: false })
rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs:157:23:handle.send_hitl_response(
rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs:204:23:handle.send_hitl_response(
rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs:269:23:handle.send_hitl_response(
rust/sessions/tests/handle_impl/session_manager_satisfies_trait.rs:130:9:handle.send_hitl_response(
rust/fspec-tui/src/app/dispatch_pause_hitl.rs:190:45:backend.send_hitl_response(id, response)
rust/fspec-tui/tests/rpc037_cross_transport_parity/hitl_lifecycle.rs:44:5:embedded

```

## `$R.send_hitl_response_checked($$$A)`
```
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:138:25:self
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:489:5:fixture

```

## `$R.pause_triple($$$A)`
```
rust/rpc/src/mcp_dispatch/group_26.rs:21:9:client
rust/sessions/tests/handle_impl/session_manager_satisfies_trait.rs:126:9:handle.pause_triple(&sid, ApprovalChoice::Approve)
rust/fspec-tui/src/app/dispatch_pause_hitl.rs:162:13:backend.pause_triple(id, choice)
rust/fspec-tui/tests/rpc037_cross_transport_parity/debug_pause.rs:152:5:websocket

```

## `$R.pause_confirm($$$A)`
```
rust/rpc/src/mcp_dispatch/group_25.rs:73:9:client
rust/sessions/tests/handle_impl/session_manager_satisfies_trait.rs:123:47:handle.pause_confirm(&sid, true)
rust/fspec-tui/src/app/dispatch_pause_hitl.rs:149:13:backend.pause_confirm(id, accept)
rust/fspec-tui/tests/rpc037_cross_transport_parity/debug_pause.rs:119:5:embedded

```

## `$R.pause_resume($$$A)`
```
rust/rpc/src/mcp_dispatch/group_25.rs:46:9:client
rust/sessions/tests/handle_impl/session_manager_satisfies_trait.rs:122:46:handle.pause_resume(&sid)
rust/fspec-tui/src/app/dispatch_pause_hitl.rs:174:13:backend.pause_resume(id)
rust/fspec-tui/tests/rpc037_cross_transport_parity/debug_pause.rs:183:5:embedded

```

## `$R.reply_interaction_checked($$$A)`
```
rust/rpc/src/mcp_dispatch/group_40.rs:82:9:client
rust/fspec/tests/client_mcp_approval_regressions.rs:49:5:handle.reply_interaction_checked(&id, "pause_confirm", nonce, r#"{"accept":false}"#)

```

## `$R.interaction_snapshot($$$A)`
```
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:130:28:self.0.interaction_snapshot()
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:474:49:fixture.session.interaction_snapshot()
rust/agent-loop/tests/support/bug265.rs:167:26:session
rust/fspec/tests/client_mcp_approval_regressions.rs:33:56:handle.interaction_snapshot(&id)
rust/fspec/tests/client_mcp_approval_regressions.rs:45:60:handle.interaction_snapshot(&id)

```

## `$R.get_interaction_snapshot($$$A)`
```
rust/rpc/src/mcp_dispatch/group_39.rs:82:21:client

```

## `$R.get_pause_state($$$A)`
```
rust/cli/tests/stream_loop_pause_test.rs:164:21:session.get_pause_state()
rust/rpc/src/mcp_dispatch/group_26.rs:74:21:client
rust/sessions/tests/cov057_session_pause_state.rs:58:18:session_a
rust/sessions/tests/handle_impl/session_manager_satisfies_trait.rs:85:34:handle.get_pause_state(&sid)
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:236:32:session.get_pause_state()
rust/fspec-tui/examples/probe_pause.rs:29:21:backend.get_pause_state(sid.clone())
rust/fspec-tui/examples/repro_env_pause.rs:124:17:backend.get_pause_state(my_sid.clone())
rust/fspec-tui/tests/rpc037_cross_transport_parity/debug_pause.rs:106:17:embedded
rust/fspec-tui/tests/rpc037_cross_transport_parity/debug_pause.rs:110:17:websocket
rust/fspec-tui/tests/bug445_session_getters_report_lookup_failures.rs:100:17:handle.get_pause_state(&id)

```

## `$R.get_hitl_request($$$A)`
```
rust/rpc/src/mcp_dispatch/group_27.rs:20:21:client
rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs:66:16:handle
rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs:113:16:handle
rust/sessions/tests/handle_impl/session_manager_satisfies_trait.rs:86:35:handle.get_hitl_request(&sid)
rust/sessions/tests/bug605_closing_or_interrupting_cancels_pending_interactions.rs:227:32:session.get_hitl_request()
rust/fspec-tui/examples/probe_pause.rs:31:20:backend.get_hitl_request(sid)
rust/fspec-tui/tests/rpc037_cross_transport_parity/hitl_lifecycle.rs:30:18:embedded
rust/fspec-tui/tests/rpc037_cross_transport_parity/hitl_lifecycle.rs:34:18:websocket
rust/fspec-tui/tests/bug445_session_getters_report_lookup_failures.rs:101:20:handle.get_hitl_request(&id)

```
