# MODEL-010 AST research (2026-11-04)

```
== get_thinking_config($$$A)
rust/agent-loop/src/agent_loop/thinking.rs:60:22:get_thinking_config(config_key.to_string(), level)
rust/agent-loop/src/thinking_config/tests.rs:44:9:get_thinking_config("codex".to_string(), JsThinkingLevel::High)
rust/agent-loop/src/thinking_config/tests.rs:49:18:get_thinking_config("gpt-5.3-codex".to_string(), JsThinkingLevel::Medium)
rust/agent-loop/src/thinking_config/tests.rs:55:9:get_thinking_config("codex".to_string(), JsThinkingLevel::Off)
rust/agent-loop/src/thinking_config/tests.rs:77:18:get_thinking_config("claude-opus-4-7".to_string(), JsThinkingLevel::High)
rust/agent-loop/src/thinking_config/tests.rs:86:18:get_thinking_config("claude-opus-4-6".to_string(), JsThinkingLevel::High)
rust/agent-loop/src/thinking_config/tests.rs:95:18:get_thinking_config("claude-opus-4-5".to_string(), JsThinkingLevel::High)
rust/agent-loop/src/thinking_config/tests.rs:105:9:get_thinking_config("claude".to_string(), JsThinkingLevel::High)
rust/agent-loop/src/thinking_config/tests.rs:116:18:get_thinking_config("unknown-provider".to_string(), JsThinkingLevel::High)
rust/agent-loop/tests/bug440_openai_compatible_thinking_level.rs:16:16:get_thinking_config(provider.to_string(), level)
rust/agent-loop/tests/bug442_gemini_thinking_for_every_gemini_id.rs:12:16:get_thinking_config(model.to_string(), level)
rust/agent-loop/tests/bug441_codex_reasoning_for_every_codex_model.rs:20:31:get_thinking_config(key.into(), JsThinkingLevel::Medium)
rust/agent-loop/tests/bug441_codex_reasoning_for_every_codex_model.rs:21:44:get_thinking_config(key.into(), JsThinkingLevel::Off)
rust/agent-loop/tests/model013_prompt_text_never_changes_the_thinking_level.rs:16:18:get_thinking_config("openai".to_string(), level)
== provider_routed_thinking_key($$$A)
rust/agent-loop/src/agent_loop/thinking.rs:25:31:provider_routed_thinking_key(current_provider)
rust/agent-loop/tests/bug441_codex_reasoning_for_every_codex_model.rs:17:15:provider_routed_thinking_key("codex")
== resolve_turn_thinking($$$A)
rust/agent-loop/tests/bug455_sessions_without_a_chosen_thinking_level.rs:43:19:resolve_turn_thinking(
rust/agent-loop/tests/model013_prompt_text_never_changes_the_thinking_level.rs:30:16:resolve_turn_thinking(&session, None, "openai", Some("gpt-5.5"), None)
rust/agent-loop/tests/model013_prompt_text_never_changes_the_thinking_level.rs:44:16:resolve_turn_thinking(&session, None, "openai", Some("gpt-5.5"), None)
== request_config_for_model($$$A)
== gemini_generation_config($$$A)
rust/providers/src/gemini.rs:252:33:gemini_generation_config(thinking_config.as_ref())
rust/agent-loop/tests/bug442_gemini_thinking_for_every_gemini_id.rs:58:22:gemini_generation_config(None)
```

Conclusion: the per-turn thinking parameters are computed once per turn by agent_loop/thinking.rs::resolve_turn_thinking, which routes by model id through thinking_config.rs::get_thinking_config and provider_routed_thinking_key. Every other caller is a test (thinking_config/tests.rs, bug440/441/442, cov057, model013, bug455). The facade request_config tables (codelet-tools) are reached only through get_thinking_config and tests; gemini_generation_config (providers/src/gemini.rs) only merges whatever thinking JSON it is given.
