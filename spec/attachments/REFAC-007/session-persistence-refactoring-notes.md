# Session Persistence Refactoring Investigation Notes

## Executive Summary

Investigation into why resumed sessions sometimes don't have full conversation history revealed a **critical architectural problem**: TypeScript is handling message persistence instead of Rust.

## Current Architecture Problem

### TypeScript handles all persistence (WRONG!)

In `AgentView.tsx`, TypeScript is calling `persistenceStoreMessageEnvelope` at multiple points:
- Line 2638: User message stored before streaming starts
- Line 2917: Assistant message stored when ToolResult chunk arrives  
- Line 2949: Tool result stored immediately when received
- Line 3391: Final assistant message stored AFTER Done chunk

This is fundamentally wrong because:
1. **Rust owns the session state** - The `BackgroundSession` in Rust has the complete conversation
2. **TypeScript shouldn't manage agent state** - It should only handle UI concerns
3. **Race conditions and data loss** - If TypeScript crashes or errors occur, messages are lost
4. **Complexity** - Persistence logic is split between TypeScript and Rust

### Evidence of Data Loss

Analysis of session files found:
- **32% of sessions (30 out of 93)** end with a `tool_result` message
- This means the final assistant response was **never persisted**
- Root cause: Final persist only happens AFTER `await promptComplete` 
- If error occurs, `reject()` is called → jumps to catch block → persistence skipped

### Shell Command to Verify

```bash
# Count sessions ending with tool_result (missing assistant response)
for f in ~/.fspec/sessions/*.json; do
  count=$(jq '.messages | length' "$f" 2>/dev/null)
  if [ "$count" -gt "2" ]; then
    last_id=$(jq -r '.messages[-1].message_id' "$f" 2>/dev/null)
    last_type=$(grep "$last_id" ~/.fspec/messages/messages.jsonl | \
      jq -r '.metadata.message.content[0].type // "text"')
    if [ "$last_type" = "tool_result" ]; then
      echo "INCOMPLETE: $(basename $f)"
    fi
  fi
done
```

## Required Refactoring

### Target Architecture

ALL persistence should happen in Rust:

1. **`agent_loop` in Rust should persist messages** as they are processed
2. **User input** - Persist when received in `agent_loop`
3. **Assistant response** - Persist during/after streaming in `BackgroundOutput`
4. **Tool calls** - Persist when tool_use block is emitted
5. **Tool results** - Persist when tool_result is processed

### Key Files to Modify

**Rust (where persistence SHOULD happen):**
- `codelet/napi/src/session_manager.rs` - `agent_loop` function (~line 3538)
- `codelet/cli/src/interactive/stream_loop.rs` - Streaming output handling

**TypeScript (remove persistence from here):**
- `src/tui/components/AgentView.tsx` - Remove all `persistenceStoreMessageEnvelope` calls

### Benefits of Rust-Only Persistence

1. **Single source of truth** - Rust owns session state AND persistence
2. **Atomic operations** - Can guarantee consistency
3. **No race conditions** - Rust's ownership model prevents data races
4. **Error recovery** - Rust can persist partial content on errors
5. **Simpler TypeScript** - UI only handles display, not state

## Additional Findings

### Sessions with 0 Messages (Expected)

200 sessions have 0 messages. This is **expected behavior** - sessions are created eagerly when TUI opens, before the first message is sent.

### Compaction Summary Restoration

The compaction summary IS being stored correctly and restored for UI display. The issue is specifically about message persistence during streaming.

## Acceptance Criteria for Refactoring

1. ALL message persistence happens in Rust `agent_loop`
2. TypeScript has ZERO `persistenceStoreMessageEnvelope` calls
3. No sessions end with `tool_result` (final assistant response always persisted)
4. Error conditions still persist partial content
5. All existing tests pass
6. New integration tests verify round-trip persistence

## Test Fixtures Created (For Reference)

During investigation, I created test fixtures that demonstrate proper envelope structure. These can be used as reference for the Rust implementation:

```rust
// User text envelope
{
    "type": "user",
    "message": {
        "role": "user",
        "content": [{"type": "text", "text": "Hello"}]
    }
}

// Assistant with tool_use
{
    "type": "assistant",
    "message": {
        "role": "assistant",
        "content": [
            {"type": "text", "text": "I'll help"},
            {"type": "tool_use", "id": "tool_123", "name": "Read", "input": {...}}
        ]
    }
}

// User tool_result
{
    "type": "user",
    "message": {
        "role": "user",
        "content": [{
            "type": "tool_result",
            "tool_use_id": "tool_123",
            "content": "File contents",
            "is_error": false
        }]
    }
}

// Assistant thinking + text
{
    "type": "assistant",
    "message": {
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "Let me analyze..."},
            {"type": "text", "text": "Here's my answer."}
        ]
    }
}
```

## Related Work Units

- NAPI-008: Message envelope format
- SESS-001: Session attachment
- CTX-003: Context tracking
