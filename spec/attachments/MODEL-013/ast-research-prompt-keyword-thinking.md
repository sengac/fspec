# MODEL-013 AST research (2026-11-04)

```
rust/agent-loop/src/thinking_level_detection.rs:170:1:pub fn effective_turn_thinking_level(

rust/agent-loop/src/agent_loop/thinking.rs:20:27:effective_turn_thinking_level(base_level, input)
rust/agent-loop/tests/bug455_sessions_without_a_chosen_thinking_level.rs:39:22:effective_turn_thinking_level(base, input)

rust/agent-loop/src/thinking_level_detection.rs:96:1:pub fn detect_thinking_level(prompt: &str) -> JsThinkingLevel {

```

Conclusion: keyword detection enters the turn only through effective_turn_thinking_level (rust/agent-loop/src/thinking_level_detection.rs), called once by agent_loop/thinking.rs::resolve_turn_thinking. Removing it leaves the base level (base_thinking_level_from_u8) as the turn's level. The keyword features (dynamic-thinking-level-detection-via-keywords, the thinking-level-dialog keyword scenarios) are mapped only to retired TypeScript tests.
