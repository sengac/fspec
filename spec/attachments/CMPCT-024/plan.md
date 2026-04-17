# CMPCT-024 — Preserve Partial Assistant Text and Token Tracker on Hook-Triggered Compaction

**Parent:** CMPCT-022
**Bug:** BUG 2 + BUG 6

## The Problem

### BUG 2: Partial `assistant_text` is silently discarded

In `codelet/cli/src/interactive/stream_loop.rs:1156-1161`:

```rust
if is_compaction_cancel && compaction_triggered {
    debug!("[stream_loop] Breaking due to compaction cancellation (expected)");
    break;
}
```

The `break` does NOT call `handle_final_response(&assistant_text, &mut session.messages)`. Any text streamed by the LLM before cancellation is lost.

### Rig can cancel at 5 of 6 sites AFTER tokens are emitted

From `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs`:

| Line | Hook                                   | Cancellable | Tokens emitted? |
|------|----------------------------------------|-------------|-----------------|
| 412  | `on_completion_call`                   | ✅          | ❌ (before API call) |
| 460  | `on_text_delta`                        | ✅          | ✅ (tokens already streamed) |
| 486  | `on_tool_call`                         | ✅          | ✅ (tool call emitted) |
| 509  | `on_tool_result`                       | ✅          | ✅ (tool executed) |
| 542  | `on_tool_call_delta`                   | ✅          | ✅ (tool args streaming) |
| 586  | `on_stream_completion_response_finish` | ✅          | ✅ (full turn done) |

**Only site 412 has no partial data. The other 5 all have accumulated `assistant_text` that would be destroyed by the current bare `break`.**

### BUG 6: Token tracker not updated before cancel break

CMPCT-002 acceptance Rule [2] states:
> Token tracker MUST be updated with cumulative billing before signaling compaction.

The Gemini continuation path correctly does this (`gemini_continuation.rs:338`):
```rust
update_token_tracker(session, display);
super::stream_loop::signal_compaction_needed(parent_token_state);
```

The main stream loop's compaction-cancel path does not. Any `Usage` chunk received before cancellation is tracked in `streaming_display` but never flushed to `session.token_tracker` because:

1. The `break` at line 1160 exits the stream loop.
2. The post-loop update block at `stream_loop.rs:1531-1550` is skipped when `compaction_needed == true` (because control returns directly from `handle_compaction_retry` at line 1526).

Result: `session.token_tracker.cumulative_billed_input` and `cumulative_billed_output` lose one turn of accounting on every compaction cancel.

## The Fix

Update `stream_loop.rs:1156-1161` to mirror the Gemini continuation pattern:

```rust
if is_compaction_cancel && compaction_triggered {
    debug!("[stream_loop] Breaking due to compaction cancellation (expected)");
    
    // BUG 2: Save partial assistant text before breaking
    if !assistant_text.is_empty() {
        handle_final_response(&assistant_text, &mut session.messages)?;
        assistant_text.clear();
    }
    
    // BUG 6: Flush token tracker with current billing
    let partial_display = streaming_display.current();
    let partial_usage = ApiTokenUsage::new(
        partial_display.input_tokens,
        partial_display.cache_read_tokens,
        partial_display.cache_creation_tokens,
        0,
    );
    session.token_tracker.update_from_usage(
        &partial_usage,
        partial_display.output_tokens,
    );
    
    break;
}
```

(Note: this will likely get extracted into the unified helper from CMPCT-023.)

## Reference Implementation

`gemini_continuation.rs:331-345`:

```rust
if is_compaction_cancelled(&e) {
    info!("Compaction triggered during Gemini continuation - handling gracefully");

    if !text.is_empty() {
        handle_final_response(text, &mut session.messages)?;
        info!("Saved {} chars of partial continuation text", text.len());
    }
    update_token_tracker(session, display);
    super::stream_loop::signal_compaction_needed(parent_token_state);

    output.emit_compaction_started();
    let total_turns = session.messages.len() as u32 / 2;
    output.emit_compaction_progress("Context limit reached", 0, total_turns.max(1));
    set_tool_progress_callback(uuid::Uuid::nil(), None);
    return Ok(GeminiContinuationResult::CompactionNeeded);
}
```

## Comparison with Other Preservation Sites

| Location | Saves partial text? |
|----------|---------------------|
| `stream_loop.rs:571-573` (interrupt) | ✅ |
| `stream_loop.rs:622-624` (stall timeout) | ✅ |
| `stream_loop.rs:1245-1248` (truncation recovery) | ✅ |
| `stream_loop.rs:1346-1349` (network retry) | ✅ |
| `gemini_continuation.rs:334-337` (Gemini cancel) | ✅ |
| `stream_loop.rs:1156-1161` (**compaction cancel**) | ❌ |

This is a classic copy-omit defect.

## Acceptance Criteria

1. When `PromptCancelled` fires during streaming after at least one text chunk was emitted, the accumulated `assistant_text` is appended to `session.messages` as an Assistant message BEFORE compaction runs.
2. When `PromptCancelled` fires after at least one `Usage` chunk was received, `session.token_tracker.cumulative_billed_input` and `cumulative_billed_output` reflect that usage.
3. After compaction completes and the retry stream finishes, the preserved partial text appears in the session's conversation history (i.e., is visible in `session.messages`).
4. Gemini continuation behavior is unchanged.

## Files to Modify

- `codelet/cli/src/interactive/stream_loop.rs` (lines 1156-1161)

## Testing

- New integration test: seed a mock stream that yields `[TextChunk("important"), Usage(...), Err(PromptCancelled)]`; verify `session.messages.last()` is `Assistant { content: "important" }` AND `session.token_tracker.cumulative_billed_input > 0`.
- New integration test: same but yield PromptCancelled after `ToolCall` + `ToolResult`; verify both are preserved.
- Run all existing tests in `codelet/cli/tests/` to confirm no regression.
