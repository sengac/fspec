# AST research: human-action reply paths (AMGR-023, 2026-11-09)

Raw fspec astgrep output (rust/, target excluded) below, then the summary.

```
## pattern: fn pause_resume($$$ARGS) $$$RET
No matches found
## pattern: PauseKind::Continue
rust/cli/src/interactive/goal.rs:70:19:PauseKind::Continue
rust/cli/tests/stream_loop_pause_test.rs:108:23:PauseKind::Continue
rust/cli/tests/stream_loop_pause_test.rs:156:23:PauseKind::Continue
rust/cli/tests/stream_loop_pause_test.rs:221:23:PauseKind::Continue
rust/cli/tests/stream_loop_pause_test.rs:246:23:PauseKind::Continue
rust/tools/src/web_search.rs:612:15:PauseKind::Continue
rust/tools/src/tool_pause/tests/part_1.rs:37:15:PauseKind::Continue
rust/tools/src/tool_pause/tests/part_1.rs:56:23:PauseKind::Continue
rust/tools/src/tool_pause/tests/part_1.rs:92:23:PauseKind::Continue
rust/tools/src/tool_pause/tests/part_1.rs:122:23:PauseKind::Continue
rust/tools/src/tool_pause/tests/part_1.rs:166:23:PauseKind::Continue
rust/tools/tests/tool_pause_test.rs:43:23:PauseKind::Continue
rust/tools/tests/tool_pause_test.rs:83:23:PauseKind::Continue
rust/tools/tests/tool_pause_test.rs:152:23:PauseKind::Continue
rust/tools/tests/tool_pause_test.rs:204:23:PauseKind::Continue
rust/tools/tests/tool_pause_test.rs:266:23:PauseKind::Continue
rust/tools/tests/tool_pause_test.rs:373:27:PauseKind::Continue
rust/tools/tests/pause_handler_session_isolation_test.rs:93:19:PauseKind::Continue
rust/tools/tests/pause_handler_session_isolation_test.rs:107:19:PauseKind::Continue
rust/tools/tests/pause_handler_session_isolation_test.rs:133:19:PauseKind::Continue
rust/sessions/src/background_session/interactions.rs:107:9:PauseKind::Continue
rust/sessions/tests/amgr020_requests_answered_by_id.rs:286:13:PauseKind::Continue
rust/sessions/tests/bug631_pause_reply_answers_only_its_pause_kind.rs:98:9:PauseKind::Continue
rust/sessions/tests/bug631_pause_reply_answers_only_its_pause_kind.rs:179:10:PauseKind::Continue
rust/sessions/tests/bug631_pause_reply_answers_only_its_pause_kind.rs:180:10:PauseKind::Continue
rust/sessions/tests/bug631_pause_reply_answers_only_its_pause_kind.rs:183:10:PauseKind::Continue
rust/sessions/tests/bug631_pause_reply_answers_only_its_pause_kind.rs:235:18:PauseKind::Continue
rust/agent-loop/src/agent_manager_requests.rs:215:10:PauseKind::Continue
rust/agent-loop/tests/bug631_bridge_pause_response_answers_only_a_three_choice_approval.rs:73:15:PauseKind::Continue
rust/agent-loop/tests/amgr021_parent_answers_approves_or_escalates.rs:187:15:PauseKind::Continue

## pattern: PauseReply::$V
rust/agent-loop/src/bridge_control.rs:56:9:PauseReply::Triple
rust/agent-loop/src/agent_manager_requests.rs:227:56:PauseReply::Triple
rust/agent-loop/src/agent_manager_requests.rs:229:13:PauseReply::Triple
rust/agent-loop/src/agent_manager_requests.rs:231:53:PauseReply::Triple

## pattern: Action::PauseResumed { $$$F }
rust/fspec-tui/tests/paused_prompt_write_failures_bug395.rs:214:18:Action::PauseResumed {

```

## Summary

- Raisers of a human action (internal PauseKind::Continue): tools/src/web_search.rs:612 (WebSearch 'interact with the page'), cli/src/interactive/goal.rs:70 (/goal escalation). Both wait in pause_for_user; WebSearch treats PauseResponse::Interrupted as "Operation interrupted by user"; the goal escalation ignores the response (the goal stays active).
- sessions/src/conversions.rs maps Continue to the wire PauseKind::Confirm today; background_session/interactions.rs PauseReply {Resume, Confirm, Triple} answers Continue with Resume or Confirm (BUG-631).
- Reply paths: rpc service_declaration supervision_credentials.rs pause_resume (frozen index, wire_order_frozen.txt line 80), service_implementation supervision_interaction.rs, interaction_sessions.rs reply_interaction_checked, mcp_dispatch catalog/dispatch/validate/group_25, core SessionManagerHandle + stub, sessions handle_impl_3.rs, fspec-tui backend trait + embedded + websocket transports, Action::PauseResumed / handle_pause_resumed (unreachable from the prompt keys, RPC-406), agent-loop bridge_control.rs (pause_response answers Triple only), agent_manager_requests.rs (AgentManager approve rejects a human action).
- TUI prompt: views/agent/pause_prompt.rs and pause_keys.rs match the wire PauseKind {Confirm, Triple}; a human action shows as a Y/N confirm.
