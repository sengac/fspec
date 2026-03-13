# GLM/ZAI DeepSearch Streaming Investigation

## Error

```
DeepSearch sub-agent failed: Prompt failed: CompletionError: HttpError: Invalid status code 500 Internal Server Error with message: 
{"error":{"code":"1234","message":"Network error, error id: 20260313110043f39c0c779d324bf1, please contact customer service"}}
```

## Root Cause (Suspected)

The GLM/ZAI provider likely requires streaming execution like Codex, but `provider_uses_streaming_execution()` only returns `true` for "codex":

```rust
// codelet/napi/src/deep_search_handler.rs:137-139
fn provider_uses_streaming_execution(provider_name: &str) -> bool {
    provider_name == "codex"
}
```

## How Codex Was Fixed (BUG-104)

1. Added `provider_uses_streaming_execution()` function that returns `true` only for "codex"
2. Modified `build_and_run!` macro to check this function:
   - If streaming: call `rig_agent.prompt_streaming($query)` and collect final response
   - If non-streaming: call `rig_agent.prompt($query)`
3. Codex requires streaming because it uses the Responses API

## Current State

- ZAI provider: Uses OpenAI-compatible completions API via rig
- ZAI DeepSearch config: Sets `temperature: 1.0` and `top_p: 0.95`
- ZAI `supports_streaming()`: Returns `true` (line 328-330 in zai.rs)

## Hypothesis

GLM models may require streaming execution similar to Codex. The 500 error could indicate the API rejects non-streaming requests for certain operations.

## Proposed Fix

Add "zai" to the streaming providers list:

```rust
fn provider_uses_streaming_execution(provider_name: &str) -> bool {
    provider_name == "codex" || provider_name == "zai"
}
```

## Files to Modify

1. `codelet/napi/src/deep_search_handler.rs` - Update `provider_uses_streaming_execution()`
2. `codelet/napi/src/deep_search_handler/tests.rs` - Add test for zai streaming

## Testing

Run DeepSearch with zai/glm provider after the fix.
