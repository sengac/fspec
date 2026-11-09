# AST research: request routing, await_idle and AgentManager actions (AMGR-021, 2026-11-08)

fspec astgrep call-site patterns over rust/ (file:line:col:match). They locate where a request is raised, where a holder can be interrupted or closed, what await_idle reports, and where the spawner chain is read.

## `$R.set_hitl_request($$$A)`
```
rust/agent-loop/src/agent_loop/handlers.rs:27:13:session_for_hitl.set_hitl_request(Some(request))
```

## `$R.set_pause_state($$$A)`
```
rust/sessions/examples/repro_paused_chunk_delay.rs:87:9:session_for_pause.set_pause_state(Some(state))
rust/agent-loop/src/agent_loop/handlers.rs:11:9:session_for_pause.set_pause_state(Some(state))
rust/fspec-tui/src/app/dispatch_pause_route.rs:32:9:self.agent_view_store
```

## `$R.interrupt()`
```
rust/tools/src/unified_exec/tool/handlers.rs:75:14:self.interrupt()
rust/sessions/src/session_manager/manager_impl_6.rs:75:13:session.interrupt()
rust/sessions/src/background_session/input_modes.rs:52:17:self.interrupt()
rust/agent-loop/src/bridge_control.rs:14:17:session.interrupt()
```

## `$R.destroy_session($$$A)`
```
rust/rpc/src/mcp_dispatch/group_29.rs:20:9:client
rust/agent-loop/src/agent_manager_handler.rs:200:17:session_manager.destroy_session(&subordinate_id.to_string())
rust/agent-loop/src/agent_manager_handler.rs:422:11:session_manager.destroy_session(target_session_id)
rust/fspec-tui/src/app/merge_worktree_success.rs:55:25:backend.destroy_session(session_id)
rust/fspec-tui/src/app/dispatch_agent_exit.rs:74:29:backend.destroy_session(sub.id.clone())
rust/fspec-tui/src/app/dispatch_agent_exit.rs:84:25:backend.destroy_session(facts.session.clone())
```

## `AwaitOutcome::$V`
```
rust/tools/src/agent_manager/test_support.rs:193:74:AwaitOutcome::TimedOut
rust/tools/src/agent_manager/await_idle_cancel_tests.rs:34:37:AwaitOutcome::Destroyed
rust/tools/src/agent_manager/await_idle_cancel_tests.rs:74:35:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_cancel_tests.rs:75:35:AwaitOutcome::Interrupted
rust/tools/src/agent_manager/await_idle_cancel_tests.rs:76:35:AwaitOutcome::Interrupted
rust/tools/src/agent_manager/await_idle_cancel_tests.rs:173:37:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:39:35:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:40:35:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:41:35:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:78:37:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:113:37:AwaitOutcome::TimedOut
rust/tools/src/agent_manager/await_idle_tests.rs:152:35:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:153:35:AwaitOutcome::Idle
rust/tools/src/agent_manager/await_idle_tests.rs:154:35:AwaitOutcome::TimedOut
rust/agent-loop/src/agent_manager_handler.rs:890:25:AwaitOutcome::Idle
rust/agent-loop/src/agent_manager_handler.rs:949:33:AwaitOutcome::TimedOut
rust/agent-loop/src/agent_manager_handler.rs:1045:21:AwaitOutcome::Interrupted
rust/agent-loop/src/agent_manager_wait.rs:26:26:AwaitOutcome::Idle
rust/agent-loop/src/agent_manager_wait.rs:28:46:AwaitOutcome::Destroyed
rust/agent-loop/src/agent_manager_wait.rs:32:42:AwaitOutcome::Idle
rust/agent-loop/src/agent_manager_wait.rs:33:36:AwaitOutcome::Destroyed
```

## `$R.get_supervisors($$$A)`
```
rust/rpc/src/mcp_dispatch/group_22.rs:20:21:client
rust/sessions/src/session_manager/manager_impl_6.rs:180:9:self.chain_of_command.get_supervisors(subordinate_id)
rust/sessions/src/background_session/interactions.rs:128:39:manager.get_supervisors(current)
rust/agent-loop/src/agent_manager_handler.rs:284:27:session_manager.get_supervisors(current)
rust/agent-loop/src/agent_manager_handler.rs:328:26:session_manager
rust/agent-loop/src/agent_manager_handler.rs:370:22:session_manager
rust/agent-loop/src/agent_manager_handler.rs:414:23:session_manager.get_supervisors(target_uuid)
rust/fspec-tui/src/app/dispatch_supervisor_links.rs:83:38:backend.get_supervisors(session_id.clone())
rust/fspec-tui/src/app/dispatch_agent_close.rs:75:34:backend
```

## `$R.receive_agent_message($$$A)`
```
rust/agent-loop/src/agent_manager_handler.rs:525:11:target_session.receive_agent_message(incoming)
```

## `$R.receive_incoming_message($$$A)`
```
rust/rpc/src/mcp_dispatch/group_23.rs:72:9:client
rust/agent-loop/src/agent_loop/bridge.rs:64:19:session_for_injector.receive_incoming_message(supervisor_input)
rust/fspec-tui/src/app/dispatch_supervisor_links.rs:56:29:backend
```

## `ORCHESTRATOR_ROLE`
```
rust/sessions/src/bot.rs:14:11:ORCHESTRATOR_ROLE
rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:228:45:ORCHESTRATOR_ROLE
rust/fspec-tui/src/store/agent_view/role_state.rs:51:41:ORCHESTRATOR_ROLE
```
