# MODEL-015 AST research: where per-turn Claude effort plugs in

Run 2026-11-06 with `fspec astgrep --lang rust --path rust`. The turn's level is resolved once per turn (agent_loop.rs), every request of a turn goes through RigAgent::prompt_streaming_with_history_and_hook (the driver and the restart/steer/recovery paths), the Claude body params are merged in rig's anthropic request_body.rs, the beta headers are fixed per client (build_beta_headers), the driver configures the RigAgent per turn (configure_agent), and restore splices the replayed history (session_restore.rs).

## `thinking::resolve_turn_thinking($$$A)`
```
rust/agent-loop/src/agent_loop.rs:130:43:thinking::resolve_turn_thinking(

```
## `$A.prompt_streaming_with_history_and_hook($$$B)`
```
rust/cli/src/interactive/gemini_continuation.rs:102:22:agent
rust/cli/src/interactive/stream_loop/final_completion.rs:122:23:agent
rust/cli/src/interactive/stream_loop/steer.rs:62:15:agent
rust/cli/src/interactive/stream_loop/final_thinking.rs:128:19:agent
rust/cli/src/interactive/stream_loop/recovery_network.rs:71:19:agent
rust/cli/src/interactive/stream_loop/restart.rs:105:15:agent
rust/cli/src/interactive/stream_loop/driver.rs:123:46:agent
rust/cli/src/interactive/stream_loop/recovery_truncation.rs:62:19:agent
rust/cli/src/interactive/gemini_continuation/continuation_loop.rs:190:38:agent
rust/providers/tests/cmpct082_physical_admission.rs:98:18:agent
rust/providers/tests/cmpct082_hook_cancellation.rs:50:22:agent
rust/providers/tests/cmpct069_request_settlement.rs:140:26:agent
rust/providers/tests/cmpct069_request_settlement.rs:219:18:agent
rust/providers/tests/cmpct069_runtime_settlement.rs:42:18:agent
rust/providers/tests/cmpct069_runtime_settlement.rs:96:18:agent
```
## `merge_inplace(&mut body, params)`
```
rust/patches/rig-core/src/providers/anthropic/request_body.rs:80:13:merge_inplace(&mut body, params)

```
## `build_beta_headers($$$A)`
```
rust/providers/src/claude.rs:273:34:build_beta_headers(model, auth_mode == AuthMode::OAuth)
rust/providers/src/claude.rs:378:34:build_beta_headers(model, true)
rust/providers/src/claude.rs:476:9:build_beta_headers(&self.model_name, self.auth_mode == AuthMode::OAuth)
rust/providers/tests/claude_opus_4_6_adaptive_thinking_test.rs:38:23:build_beta_headers(model, false)
rust/providers/tests/claude_opus_4_6_adaptive_thinking_test.rs:72:23:build_beta_headers(model, false)
rust/providers/tests/claude_opus_4_6_adaptive_thinking_test.rs:105:23:build_beta_headers(model, false)
rust/providers/tests/claude_opus_4_6_adaptive_thinking_test.rs:136:23:build_beta_headers(model, false)
rust/providers/tests/claude_opus_4_7_adaptive_thinking_test.rs:49:23:build_beta_headers(model, false)

```
## `$S.configure_agent($A)`
```
rust/cli/src/interactive/stream_loop/driver.rs:70:17:session
rust/agent-loop/src/agent_loop/provider_custom.rs:45:17:inner_session.configure_agent(codelet_core::RigAgent::with_default_depth(agent))
rust/agent-loop/src/agent_loop/provider_custom.rs:99:13:inner_session.configure_agent(codelet_core::RigAgent::with_default_depth(agent))
rust/agent-loop/src/agent_loop/provider_openai.rs:40:17:inner_session.configure_agent(codelet_core::RigAgent::with_default_depth(agent))

```
## `inner.messages.splice($$$A)`
```
rust/sessions/src/session_restore.rs:112:17:inner.messages.splice(prefix..end, model_replay.messages)

```
