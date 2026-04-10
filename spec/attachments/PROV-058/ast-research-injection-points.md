# AST Research: Copilot Prompt Caching Injection Points

## Request Body Processing Pipeline

### `classify_body()` in `refreshing_client.rs:84`
```
fn classify_body(body: &bytes::Bytes) -> RequestClassification
```
This is the existing JSON body parsing entry point. The body is already deserialized as `serde_json::Value` here. This is the ideal hook point to also inject `copilot_cache_control` — we can extend this function (or call a sibling) to mutate the parsed JSON before re-serializing.

### `inject_copilot_headers()` in `refreshing_client.rs:103`
```
fn inject_copilot_headers(mut req: Request<bytes::Bytes>, classification: &RequestClassification, access_token: &str) -> Request<bytes::Bytes>
```
Header injection function. Could be extended to also handle body mutation, or a parallel `inject_copilot_cache_control()` could be added.

## Model Family Detection

### `behavior_facade.rs` — Three family implementations
- `CopilotGptBehaviorFacade::family()` → `"gpt"` (line 65)
- `CopilotClaudeBehaviorFacade::family()` → `"claude"` (line 97)
- `CopilotGeminiBehaviorFacade::family()` → `"gemini"` (line 118)

### Model ID prefix detection pattern (from `behavior_facade.rs:148`)
```rust
if model_id.starts_with("gpt-") { ... }
else if model_id.starts_with("claude-") { ... }
else if model_id.starts_with("gemini-") { ... }
```

This same prefix check should be reused in the cache injection logic.

## Send Path (all three variants call classify_body then inject_copilot_headers)

The `send()`, `send_multipart()`, and `send_streaming()` methods all follow the same pattern:
1. `classify_body(req.body())` → `RequestClassification`
2. `inject_copilot_headers(req, &classification, &access_token)` → modified request

For prompt caching, we need to insert a step between 1 and 2 that:
1. Re-parses the body (or reuses the parsed value from classify_body)
2. Checks `body["model"]` starts with `"claude-"`
3. Injects `copilot_cache_control` on system msg, last tool, last non-user msg
4. Re-serializes the modified body back into the request

## Key Insight: Avoid Double Parse
`classify_body()` already parses JSON. The refactored flow should parse once and return both the classification AND the (possibly mutated) body bytes.
