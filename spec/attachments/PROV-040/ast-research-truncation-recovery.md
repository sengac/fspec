# AST Research: PROV-040 Truncated Tool Call Recovery

## Research Date: 2026-03-21

## 1. Error Detection Point in stream_loop.rs

The `Some(Err(e))` arm at line 1400 handles all streaming errors. The error cascading is:
1. Compaction cancellation (line 1425) → `break`
2. Prompt-too-long (line 1440) → `break` to compaction
3. Image content error (line 1465) → `break` if sanitized
4. **General errors (line 1490)** → `return Err(...)` ← THIS IS WHERE TRUNCATION FALLS THROUGH

## 2. Truncation Error Origin

**Anthropic** (`codelet/patches/rig-core/src/providers/anthropic/streaming.rs`):
- Lines 436-451: Post-loop handler emits enriched "Tool call truncated due to output token limit" error
- Only fires when `captured_stop_reason == "max_tokens"` AND pending tool call exists

**OpenAI** (`codelet/patches/rig-core/src/providers/openai/completion/streaming.rs`):
- No enriched truncation error — flushes partial tool calls with whatever JSON exists
- Only signals via `stop_reason: "max_tokens"` in FinalResponse

**Gemini** (`codelet/patches/rig-core/src/providers/gemini/streaming.rs`):
- Tool calls arrive as complete FunctionCall parts, not incremental JSON
- No accumulation that could be truncated mid-parse

## 3. Recovery Pattern: Compaction Retry (stream_loop.rs:1648-1845)

The compaction retry pattern provides the template:
- Creates fresh `retry_token_state` and `retry_hook`
- Calls `agent.prompt_streaming_with_history_and_hook("Continue", ...)`
- Processes a full retry stream with text/tool_call/tool_result/final handling
- Propagates stop_reason via `emit_done_with_stop_reason`

## 4. Key Functions

- `is_prompt_too_long_error()` (line 78) — Pattern for error string matching
- `is_image_content_error()` (line 96) — Pattern for error string matching
- `signal_compaction_needed()` (line 231) — Setting token state flags
- `run_agent_stream_internal()` (line 421) — Main function signature with generics

## 5. Implementation Plan

Add new function `is_truncated_tool_call_error(error_str: &str) -> bool` matching the PROV-039 error string.

Add new function `build_truncation_recovery_message(error_str: &str) -> String` that generates the recovery instruction.

Insert a new case in the error handling cascade between image content error (line 1488) and general errors (line 1490):
```rust
if is_truncated_tool_call_error(&error_str) && truncation_retry_count < MAX_TRUNCATION_RETRIES {
    // Increment counter
    // Add recovery message to session.messages
    // Start new retry stream
    // Continue processing
}
```

Add `truncation_retry_count: u32` as a turn-level variable initialized at the top of `run_agent_stream_internal`.
