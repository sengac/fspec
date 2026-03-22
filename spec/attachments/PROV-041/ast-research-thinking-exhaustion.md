# AST Research: Thinking Token Exhaustion Recovery (PROV-041)

## Key Functions in stream_loop.rs

### Detection & Recovery Functions (PROV-040 pattern to follow)
```
stream_loop.rs:78  - pub fn is_prompt_too_long_error(error_str: &str) -> bool
stream_loop.rs:102 - pub fn is_image_content_error(error_str: &str) -> bool
stream_loop.rs:125 - pub fn sanitize_image_content(messages: &mut [Message]) -> bool
stream_loop.rs:235 - pub fn is_truncated_tool_call_error(error_str: &str) -> bool
stream_loop.rs:245 - pub fn build_truncation_recovery_message(error_str: &str) -> String
stream_loop.rs:277 - pub fn build_truncation_budget_exhausted_message(max_retries: u32) -> String
stream_loop.rs:226 - pub const MAX_TRUNCATION_RETRIES: u32 = 2
stream_loop.rs:289 - fn is_compaction_cancelled(error: &anyhow::Error) -> bool
stream_loop.rs:428 - pub fn build_user_content_with_images(...)
```

### stop_reason & FinalResponse Handling (PROV-039)
- Line 745: `let mut final_stop_reason: Option<String> = None;`
- Line 747: `// PROV-040: Track consecutive truncation retries`
- Line 962: `Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp)))` — main handler
- Line 968-971: Captures stop_reason from FinalResponse
- Line 975: `let usage = final_resp.usage()` — access to token usage
- Line 1001: `usage.reasoning_tokens.unwrap_or(0)` — reasoning tokens available

### PROV-040 Truncation Recovery Pattern (lines 1556-1640)
- Line 1556: `is_truncated_tool_call_error(&error_str)` — detection gate
- Line 1565: Logs recovery attempt with attempt/max counters
- Line 1576: Builds recovery message
- Line 1589: Starts recovery stream
- Line 1615: Resets final_stop_reason for retry
- Line 1631: Budget exhaustion handling

## ThinkingLevel Enum (codelet/tools/src/facade/thinking_config.rs:64)
```rust
pub enum ThinkingLevel {
    Off,    // Disable thinking/reasoning entirely
    Low,    // Minimal thinking (fast responses)
    Medium, // Balanced thinking (default for most tasks)
    High,   // Maximum thinking (complex reasoning tasks)
}
```

## ThinkingConfigFacade Trait (thinking_config.rs:76)
```rust
pub trait ThinkingConfigFacade {
    fn provider(&self) -> &'static str;
    fn request_config(&self, level: ThinkingLevel) -> Value;
    fn is_thinking_part(&self, part: &Value) -> bool;
    fn extract_thinking_text(&self, part: &Value) -> Option<String>;
}
```

## Provider Implementations
- `Gemini3ThinkingFacade` (line 93): thinkingLevel enum
- `Gemini25ThinkingFacade` (line 140): thinkingBudget token count
- `ClaudeThinkingFacade` (line 229): thinking.budget_tokens with model-aware Adaptive detection

## Key Data Available at Detection Point (FinalResponse handler)
1. `usage.reasoning_tokens` — Option<u64>, how many tokens spent on thinking
2. `usage.output_tokens` — u64, how many tokens for actual output
3. `final_stop_reason` — Option<String>, "max_tokens" for Length, "end_turn" for Stop
4. `assistant_text` — String, the accumulated text output
5. `session.token_tracker` — tracks context window usage

## New Functions Needed for PROV-041
1. `is_thinking_exhaustion(stop_reason, reasoning_tokens, output_tokens, threshold)` — detection
2. `build_thinking_exhaustion_recovery_message(reasoning_tokens, output_tokens)` — recovery msg
3. `build_thinking_budget_exhausted_message(max_retries)` — budget exhausted msg
4. `downgrade_thinking_level(current: ThinkingLevel) -> ThinkingLevel` — level degradation
5. `pub const MAX_THINKING_EXHAUSTION_RETRIES: u32 = 2` — retry budget
6. `pub const THINKING_EXHAUSTION_OUTPUT_THRESHOLD: u64 = 50` — output token threshold
