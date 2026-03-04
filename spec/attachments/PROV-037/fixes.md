# Codex Provider: Missing Reasoning Configuration — Root Cause Analysis

## Summary

GPT-5.3 Codex stops after producing a brief text response (33 output tokens, 0 reasoning tokens) without ever calling tools. The root cause is that **our Codex provider does not send the `reasoning` parameter** in Responses API requests, which prevents the model from performing multi-step agentic reasoning and tool use.

---

## Evidence from Debug Log

**Session:** `/Users/rquast/.fspec/debug/session-2026-03-04T01-56-34.jsonl`

```
sequence 33 — api.response.end:
  outputTokens: 33        ← Model produced only 33 tokens of commentary
  reasoningTokens: 0      ← Zero reasoning happened
  inputTokens: 10305
  duration: 5971ms
```

The model said *"I'll quickly scan only source code files (no Markdown/JSON) to infer what this repo does, then summarize the architecture and core behavior."* — it **planned** to use tools but then the response **completed immediately** without any `function_call` output items. The SSE stream went: chunks → `api.response.end` → `session.end` (user exit). No tool calls were ever emitted.

---

## Comparison: codex-rs (Official) vs Our Provider

### codex-rs — How it builds a Responses API request

**File:** `/tmp/codex/codex-rs/core/src/client.rs` (lines 500–553)

```rust
let reasoning = if model_info.supports_reasoning_summaries {
    Some(Reasoning {
        effort: effort.or(default_reasoning_effort),
        summary: if summary == ReasoningSummaryConfig::None {
            None
        } else {
            Some(summary)
        },
    })
} else {
    None
};

let include = if reasoning.is_some() {
    vec!["reasoning.encrypted_content".to_string()]
} else {
    Vec::new()
};

let request = ResponsesApiRequest {
    model: model_info.slug.clone(),
    instructions: instructions.clone(),
    input,
    tools,
    tool_choice: "auto".to_string(),            // ← Always explicit
    parallel_tool_calls: prompt.parallel_tool_calls,  // ← From model config
    reasoning,                                    // ← CRITICAL: enables reasoning
    store: provider.is_azure_responses_endpoint(),
    stream: true,
    include,                                      // ← Required for reasoning
    service_tier: ...,
    prompt_cache_key: Some(...),
    text,
};
```

For GPT-5.3 Codex, the wire request includes:

```json
{
  "model": "gpt-5.3-codex",
  "instructions": "...",
  "input": [...],
  "tools": [...],
  "tool_choice": "auto",
  "parallel_tool_calls": true,
  "reasoning": {
    "effort": "high",
    "summary": "auto"
  },
  "include": ["reasoning.encrypted_content"],
  "store": false,
  "stream": true
}
```

### Our provider — What it actually sends

**File:** `codelet/providers/src/codex/mod.rs` (lines 259–336)

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    _thinking_config: Option<serde_json::Value>,  // ← UNUSED (underscore prefix!)
) -> rig::agent::Agent<CodexResponsesModel> {
    // ...
    agent_builder = agent_builder.additional_params(serde_json::json!({"store": false}));
    // ← Only store:false! No reasoning, no include, no tool_choice!

    agent_builder.build()
}
```

And in `complete_with_tools` (lines 417–450):

```rust
let builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
    .tools(rig_tools)
    .preamble(effective_preamble)
    .additional_params(serde_json::json!({"store": false}));
    // ← Same problem: no reasoning config
```

Our wire request is missing critical fields:

```json
{
  "model": "gpt-5.3-codex",
  "instructions": "...",
  "input": [...],
  "tools": [...],
  "store": false,
  "stream": true
}
```

**Missing:**
- `"reasoning"` — effort + summary
- `"tool_choice"` — defaults to undefined instead of `"auto"`
- `"include"` — `["reasoning.encrypted_content"]`
- `"parallel_tool_calls"` — model capability flag

---

## The Three Bugs

### Bug 1: `_thinking_config` is completely ignored

**File:** `codelet/providers/src/codex/mod.rs`, line 263

```rust
_thinking_config: Option<serde_json::Value>,
```

The underscore prefix means Rust suppresses the "unused variable" warning. The thinking config is passed in from the NAPI layer but never consumed. It should be used to populate the `reasoning` field in additional_params.

### Bug 2: `get_thinking_config()` doesn't handle codex/openai providers

**File:** `codelet/napi/src/thinking_config.rs`, lines 112–127

```rust
pub fn get_thinking_config(provider: String, level: JsThinkingLevel) -> napi::Result<String> {
    let level: ThinkingLevel = level.into();

    let config = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.request_config(level)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.request_config(level)
    } else if is_claude_provider(&provider) {
        // Claude models...
        match ClaudeThinkingFacade.request_config_for_model(&provider, level) { ... }
    } else {
        // Unknown provider - return empty config (no thinking)
        serde_json::json!({})  // ← codex falls through to here!
    };
    // ...
}
```

When called with `"codex"` or `"gpt-5.3-codex"`, it returns empty `{}`. There is no branch for OpenAI/Codex reasoning models.

### Bug 3: Missing critical Responses API request fields

Even if thinking_config were populated, `create_rig_agent` and `complete_with_tools` only set `store: false` in additional_params. The following fields required by the Codex backend API for reasoning models are never set:

| Field | codex-rs sends | Our provider sends |
|-------|---------------|-------------------|
| `reasoning.effort` | `"high"` (from model config) | ❌ absent |
| `reasoning.summary` | `"auto"` | ❌ absent |
| `tool_choice` | `"auto"` (always explicit) | ❌ absent (rig default unclear) |
| `include` | `["reasoning.encrypted_content"]` | ❌ absent |
| `parallel_tool_calls` | `true` (from model config) | ❌ absent |

---

## Why This Causes "No Tool Calls"

GPT-5.3 Codex is a **reasoning model**. The `reasoning` parameter controls whether the model performs chain-of-thought reasoning before producing output. When `reasoning` is absent from the request:

1. The Codex backend API puts the model in a **non-reasoning completion mode**
2. The model generates brief commentary text (33 tokens) — it "plans" but cannot execute
3. The response completes immediately with `response.completed`
4. No `function_call` output items are ever produced in the SSE stream
5. The agent loop sees a completed response with no tool calls → session ends
6. The user sees one line of text and a prompt, as shown in the screenshot

This is confirmed by the debug log: `reasoningTokens: 0` despite the [T:High] thinking level being configured in the UI.

---

## Required Fixes

### Fix 1: Add Codex/OpenAI reasoning config to `get_thinking_config`

**File:** `codelet/napi/src/thinking_config.rs`

Add a branch for OpenAI/Codex reasoning models that returns Responses API format:

```rust
} else if is_codex_provider(&provider) || is_openai_reasoning_provider(&provider) {
    // OpenAI Responses API reasoning format
    match level {
        ThinkingLevel::Off => serde_json::json!({}),
        ThinkingLevel::Low => serde_json::json!({
            "reasoning": { "effort": "low", "summary": "auto" }
        }),
        ThinkingLevel::Medium => serde_json::json!({
            "reasoning": { "effort": "medium", "summary": "auto" }
        }),
        ThinkingLevel::High => serde_json::json!({
            "reasoning": { "effort": "high", "summary": "auto" }
        }),
    }
}
```

### Fix 2: Use `thinking_config` in `CodexProvider::create_rig_agent`

**File:** `codelet/providers/src/codex/mod.rs`

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,  // ← Remove underscore!
) -> rig::agent::Agent<CodexResponsesModel> {
    // ...

    // Build additional_params with all required fields
    let mut additional = serde_json::json!({
        "store": false,
        "include": ["reasoning.encrypted_content"],
    });

    // Merge reasoning config from thinking_config
    if let Some(config) = thinking_config {
        if let Some(reasoning) = config.get("reasoning") {
            additional["reasoning"] = reasoning.clone();
        }
    }

    // Default reasoning if none provided (Codex models need this)
    if additional.get("reasoning").is_none() {
        additional["reasoning"] = serde_json::json!({
            "effort": "high",
            "summary": "auto"
        });
    }

    agent_builder = agent_builder.additional_params(additional);
    agent_builder.build()
}
```

### Fix 3: Same fix for `complete_with_tools`

**File:** `codelet/providers/src/codex/mod.rs`

Apply the same reasoning injection to the `complete_with_tools` method's `additional_params`.

### Fix 4: Verify rig additional_params propagation

**File:** `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs`

Verify that `AdditionalParameters` (the `#[serde(flatten)]` field on `CompletionRequest`) correctly serializes `reasoning`, `include`, and other fields when they come through `additional_params`. The struct already has:

```rust
pub struct AdditionalParameters {
    pub reasoning: Option<Reasoning>,     // ✓ exists
    pub include: Option<Vec<Include>>,    // ✓ exists
    pub parallel_tool_calls: Option<bool>, // ✓ exists
    // ...
}
```

So the fix is to pass these through `additional_params` JSON and let serde flatten them into the request. This should work because `AdditionalParameters` is deserialized from the `additional_params` JSON value in `CompletionRequest::try_from` (line 633-634):

```rust
let additional_parameters = if let Some(map) = req.additional_params {
    serde_json::from_value::<AdditionalParameters>(map).expect(...)
} else {
    AdditionalParameters::default()
};
```

---

## Verification

After the fix, the debug log should show:

```
api.response.end:
  outputTokens: > 100     ← Model produces tool calls
  reasoningTokens: > 0    ← Reasoning tokens consumed
```

And the SSE stream should contain `response.output_item.done` events with `type: "function_call"` items before `response.completed`.

---

## Relationship to RIG-011

RIG-011 tracks reasoning token *visibility* in debug capture and token tracking. This fix is a **prerequisite**: without sending the `reasoning` parameter, there are never any reasoning tokens to track. The two issues are complementary:

1. **This fix**: Makes reasoning work at all (sends `reasoning` in the request)
2. **RIG-011**: Makes reasoning tokens visible in debug output (propagates `reasoning_tokens` through Usage structs)
