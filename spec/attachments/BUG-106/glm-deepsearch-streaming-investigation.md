# GLM/ZAI DeepSearch 500 Error — Investigation

## Error

```
DeepSearch sub-agent failed: Prompt failed: CompletionError: HttpError: Invalid status code 500 Internal Server Error with message: 
{"error":{"code":"1234","message":"Network error, error id: 20260313110043f39c0c779d324bf1, please contact customer service"}}
```

## Investigation Summary

### Key Finding: Streaming vs Non-Streaming

The main ZAI session loop uses **streaming** (`prompt_streaming_with_history_and_hook`) and works fine. DeepSearch uses **non-streaming** (`rig_agent.prompt()`) for ZAI and fails with 500.

Both paths go through the same `CompletionRequest::try_from(OpenAIRequestParams { ... })` in rig. The ONLY HTTP-level difference is that streaming merges `{"stream": true, "stream_options": {"include_usage": true}}` into the request body.

This suggests Z.AI's non-streaming code path has a server-side bug or different handling for tool-calling requests.

### Secondary Finding: max_tokens Silently Dropped

Rig's OpenAI `CompletionRequest` struct does NOT have a `max_tokens` field:

```rust
// codelet/patches/rig-core/src/providers/openai/completion/mod.rs:1004-1016
pub struct CompletionRequest {
    model: String,
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
    tool_choice: Option<ToolChoice>,
    temperature: Option<f64>,
    #[serde(flatten)]
    additional_params: Option<serde_json::Value>,
    // NOTE: No max_tokens field!
}
```

In `TryFrom<OpenAIRequestParams>` (lines 1040-1048), `CoreCompletionRequest` is destructured with `..` which silently drops `max_tokens`:

```rust
let CoreCompletionRequest {
    preamble,
    chat_history,
    tools,
    temperature,
    additional_params,
    tool_choice,
    ..  // <-- max_tokens dropped here
} = req;
```

This affects BOTH streaming and non-streaming paths equally. The main loop also drops it and works fine, so this alone doesn't cause the 500. However, the Z.AI API may benefit from having it explicitly set.

### What Was Ruled Out

| Hypothesis | Status | Reason |
|---|---|---|
| Z.AI rejects non-streaming requests | **Partially correct** | Z.AI 500s on non-streaming with tools; streaming works. But it's a server-side error, not a format rejection. |
| Missing max_tokens causes 500 | **Unlikely as sole cause** | Main session loop also drops max_tokens but works fine. |
| Tool schema format mismatch | **Not the differentiator** | Both paths use the same rig tool format for the same provider. DeepSearch uses standard rig tools vs ZAI facades for the main loop, but the HTTP request tool definitions are constructed identically by rig. |
| Transient server error | **Possible but unlikely** | User reported it as a consistent bug, not intermittent. |

## Root Cause

Z.AI's non-streaming `/chat/completions` endpoint has different behavior (likely a bug) compared to streaming when handling tool-calling requests. The streaming endpoint processes the same payload successfully.

## Fix (Two Changes)

### 1. Use streaming execution for ZAI DeepSearch

```rust
// codelet/napi/src/deep_search_handler.rs
fn provider_uses_streaming_execution(provider_name: &str) -> bool {
    provider_name == "codex" || provider_name == "zai"
}
```

This aligns DeepSearch with the working main session loop behavior.

### 2. Include max_tokens in additional_params for ZAI

```rust
// codelet/napi/src/deep_search_provider_config.rs
fn zai_request_config(system_prompt: &str) -> DeepSearchRequestConfig {
    DeepSearchRequestConfig {
        preamble: system_prompt.to_string(),
        additional_params: Some(json!({
            "temperature": 1.0,
            "top_p": 0.95,
            "max_tokens": SUB_AGENT_MAX_TOKENS
        })),
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}
```

Since rig drops `max_tokens` from `CoreCompletionRequest`, we pass it via `additional_params` (which uses `#[serde(flatten)]`) so it actually reaches the Z.AI API.

## Files to Modify

1. `codelet/napi/src/deep_search_handler.rs` — Add "zai" to `provider_uses_streaming_execution()`
2. `codelet/napi/src/deep_search_provider_config.rs` — Add `max_tokens` to ZAI `additional_params`
3. `codelet/napi/src/deep_search_handler/tests.rs` — Update test for ZAI streaming

## Testing

1. Unit tests: Verify `provider_uses_streaming_execution("zai")` returns true
2. Unit tests: Verify ZAI config includes `max_tokens` in `additional_params`
3. Integration: Run DeepSearch with ZAI provider after fix
