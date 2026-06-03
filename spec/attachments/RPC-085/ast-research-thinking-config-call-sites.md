# RPC-085 — Thinking Config Wiring AST Research

## Goal

Lock the contract that every provider dispatch arm in the agent loop
threads an `Option<serde_json::Value>` thinking config through
`create_rig_agent` per turn.

## Wiring Map

```
PromptInput.thinking_config: Option<String>
   ↓ (input_rx.recv() → InputWithImages.thinking_config: Option<String>)
   codelet/agent-loop/src/dispatch.rs:137
   ↓ (per-turn computation block)
   codelet/agent-loop/src/agent_loop.rs:372-470
   ↓ thinking_config_value: Option<serde_json::Value>
   ↓ Three dispatch paths:
   ├── run_with_provider!(... thinking_config_value)
   │   for claude / gemini / zai / codex / copilot
   │   codelet/agent-loop/src/agent_loop.rs:868, 915-917
   │   ↓
   │   dispatch.rs:58-62 → provider.create_rig_agent(id, preamble, $thinking.clone())
   ├── OpenAI inlined arm (because get_openai takes session.id)
   │   codelet/agent-loop/src/agent_loop.rs:879
   │   ↓
   │   provider.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value.clone())
   └── Custom-provider fallthrough (`_ =>`)
       codelet/agent-loop/src/agent_loop.rs:951, 972
       ↓
       custom_provider.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value.clone())
```

## Per-Turn Computation (agent_loop.rs:372-470)

Three branches with strict priority (PROV-005 fix):

1. **Adaptive thinking models** (`is_adaptive_thinking_model(routing_model) == true`)
   → ALWAYS use model-aware config from `get_thinking_config(routing_model, effective_level)`
   → Overrides any TS-passed `PromptInput.thinking_config` to avoid `max_tokens` errors on Opus 4.6/Sonnet 4.6
2. **Non-adaptive + PromptInput.thinking_config set** → `serde_json::from_str(config_str).ok()`
3. **Unified detection fallback** → detect from message text + session base level
   → `compute_effective_thinking_level(base, detected, force_off)`
   → If Off → None; otherwise `get_thinking_config(routing_model_or_provider, level)`

`force_off` from `has_disable_keywords(input)` (e.g. "quickly", "briefly")
returns the effective_level to Off regardless of base/detected.

## Provider create_rig_agent Signature

All 7 providers expose the same shape:

```rust
fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> RigAgentHandle;
```

This is verified at compile time by a no-op closure that names the
signature for each provider type.

## Verification Plan

1. **Structural source-string assertions:**
   - `dispatch.rs` contains `pub(crate) thinking_config: Option<String>` field on `InputWithImages`
   - `dispatch.rs` macro body passes `$thinking.clone()` as 3rd positional arg to `provider.create_rig_agent`
   - `agent_loop.rs` has `thinking_config_value` computation block referencing `compute_effective_thinking_level`, `is_adaptive_thinking_model`, `get_thinking_config`
   - All five `run_with_provider!` invocations end with `thinking_config_value` as the 7th positional macro arg
   - OpenAI inlined arm and custom-provider fallthrough invoke `create_rig_agent(...id..., ...preamble..., thinking_config_value.clone())`
2. **Compile-time closure signatures** for all 7 providers.
