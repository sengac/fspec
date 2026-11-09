# BUG-406 AST research

`grep -rln "let _ = codelet_common::set_data_directory\|let _ = set_data_directory" rust` (20 files after BUG-397/BUG-398 fixed theirs):

```
agent-loop/tests/prov145_profile_config_applied.rs
agent-loop/tests/rig015_loop_abort_behavioral.rs
agent-loop/tests/rpc072_work_agent_roundtrip.rs
agent-loop/tests/rpc082_role_injection.rs
agent-loop/tests/rpc086_token_tracking.rs
agent-loop/tests/wt002_footer_poller_parity.rs
fspec-tui/tests/cont008_goal_back_sync_test.rs
fspec-tui/tests/session_intent/fixture.rs
fspec/tests/client_mcp_approval_regressions.rs
providers/tests/prov161/support.rs
sessions/tests/bug231_support/mod.rs
sessions/tests/exec_stdin_prompt_p2.rs
sessions/tests/exec_stdin_push_chunk_bug171.rs
sessions/tests/fixtures/prov161_runtime.rs
sessions/tests/fixtures/rpc348_facade.rs
sessions/tests/iso001_support/mod.rs
sessions/tests/paused_chunk_delivery_rpc409.rs
sessions/tests/rpc081_restore_session_messages.rs
sessions/tests/rpc418_compact_session/compact_populated_clears_conversation.rs
sessions/tests/sess003_work_unit_focus.rs
```
