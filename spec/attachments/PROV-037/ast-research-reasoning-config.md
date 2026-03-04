# AST Research: Codex Provider Missing Reasoning Configuration

## Analysis Scope

Analyzed the following files to understand the reasoning configuration data flow:

1. `codelet/providers/src/codex/mod.rs` — CodexProvider implementation
2. `codelet/napi/src/thinking_config.rs` — get_thinking_config() NAPI binding
3. `codelet/tools/src/facade/thinking_config.rs` — ThinkingConfigFacade trait and implementations
4. `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs` — AdditionalParameters struct
5. `/tmp/codex/codex-rs/core/src/client.rs` — Reference implementation (codex-rs)
6. `/tmp/codex/codex-rs/core/models.json` — Model configuration with reasoning flags

## Key Findings

### 1. create_rig_agent — _thinking_config ignored (Bug 1)

**File:** `codelet/providers/src/codex/mod.rs`, line 263

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    _thinking_config: Option<serde_json::Value>,  // ← underscore = unused
) -> rig::agent::Agent<CodexResponsesModel> {
    // ...
    agent_builder = agent_builder.additional_params(serde_json::json!({"store": false}));
    // ← Only store:false. No reasoning, include, tool_choice, or parallel_tool_calls
}
```

### 2. get_thinking_config() — no codex/openai branch (Bug 2)

**File:** `codelet/napi/src/thinking_config.rs`, lines 112-127

```rust
let config = if is_gemini3_provider(&provider) {
    // Gemini 3
} else if is_gemini25_provider(&provider) {
    // Gemini 2.5
} else if is_claude_provider(&provider) {
    // Claude
} else {
    serde_json::json!({})  // ← codex falls through here
};
```

No `is_codex_provider()` or `is_openai_reasoning_provider()` function exists.

### 3. complete_with_tools — same missing reasoning (Bug 3)

**File:** `codelet/providers/src/codex/mod.rs`, lines 435-440

```rust
let builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
    .tools(rig_tools)
    .preamble(effective_preamble)
    .additional_params(serde_json::json!({"store": false}));
// ← Same: only store:false
```

### 4. AdditionalParameters struct — already supports all needed fields

**File:** `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs`, lines 740-777

```rust
pub struct AdditionalParameters {
    pub reasoning: Option<Reasoning>,         // ← Already exists
    pub include: Option<Vec<Include>>,        // ← Already exists
    pub parallel_tool_calls: Option<bool>,    // ← Already exists
    pub store: Option<bool>,                  // ← Already exists
    // ...
}
```

The `Reasoning` struct (line 842) has:
- `effort: Option<ReasoningEffort>` — enum with Low, Medium, High variants
- `summary: Option<ReasoningSummaryLevel>` — enum with Auto, Concise, Detailed variants

The `Include` enum (line 909) has:
- `ReasoningEncryptedContent` — serializes as `"reasoning.encrypted_content"`

### 5. codex-rs reference — how it builds the request

**File:** `/tmp/codex/codex-rs/core/src/client.rs`, lines 500-553

```rust
let reasoning = if model_info.supports_reasoning_summaries {
    Some(Reasoning {
        effort: effort.or(default_reasoning_effort),  // defaults to "medium"
        summary: Some(summary),                        // "auto" for most cases
    })
} else { None };

let include = if reasoning.is_some() {
    vec!["reasoning.encrypted_content".to_string()]
} else { Vec::new() };

let request = ResponsesApiRequest {
    tool_choice: "auto".to_string(),
    parallel_tool_calls: prompt.parallel_tool_calls,
    reasoning,
    include,
    // ...
};
```

### 6. gpt-5.3-codex model config from codex-rs

**File:** `/tmp/codex/codex-rs/core/models.json`, lines 1-47

```json
{
    "slug": "gpt-5.3-codex",
    "supports_reasoning_summaries": true,
    "default_reasoning_level": "medium",
    "supports_parallel_tool_calls": true,
    "context_window": 272000
}
```

### 7. Data flow in session_manager

**File:** `codelet/napi/src/session_manager.rs`, lines 5104-5174

The thinking_config is computed in `agent_loop()` and passed to `run_with_provider!` macro (line 4956):
```rust
provider.create_rig_agent($session.id, None, $thinking.clone())
```

For codex provider, `$thinking` comes from `get_thinking_config()` which returns `{}` (empty) because there's no codex branch — so `$thinking` evaluates to `None` (since `serde_json::from_str("{}").ok()` yields `Some({})` which isn't `None`, but the reasoning fields inside are empty).

## Summary

Three fixes needed:
1. **thinking_config.rs**: Add `is_codex_provider()` helper and codex branch to `get_thinking_config()` returning `{reasoning: {effort, summary}}` format
2. **codex/mod.rs create_rig_agent**: Remove underscore from `_thinking_config`, merge reasoning into additional_params, add default reasoning when None
3. **codex/mod.rs complete_with_tools**: Same reasoning injection into additional_params
