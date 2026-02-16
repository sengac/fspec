# AST Research: Clear Handler Implementation

## Work Unit: AGENT-022 - Clear context command for session reset

## Research Summary

### Current Implementation Location

**File:** `codelet/napi/src/session_manager.rs`
**Lines:** 4508-4516

### Current Clear Handler Code

```rust
"clear" => {
    tracing::info!("Bridge control: clearing session");
    // Clear the output buffer (conversation history display)
    if let Ok(mut buffer) = session_for_control.output_buffer.write() {
        buffer.clear();
    }
    // Reset the interrupt flag
    session_for_control.reset_interrupt();
}
```

### Problem Analysis

The current implementation only:
1. Clears the output buffer (display only)
2. Resets the interrupt flag

It does **NOT**:
1. Clear `session.messages` (actual conversation history sent to LLM)
2. Clear `session.turns` (turn-based conversation structure)
3. Reset `token_tracker` (input/output token counters)
4. Call `inject_context_reminders()` (to restore CLAUDE.md and environment info)

### Required Fix

The clear handler should follow the same pattern as the existing test in:
`codelet/napi/tests/clear_history_context_test.rs`

The fix should:
```rust
"clear" => {
    tracing::info!("Bridge control: clearing session");
    
    // Clear the output buffer (display)
    if let Ok(mut buffer) = session_for_control.output_buffer.write() {
        buffer.clear();
    }
    
    // Clear actual session state
    if let Ok(mut inner) = session_for_control.inner.write() {
        inner.messages.clear();
        inner.turns.clear();
        inner.token_tracker = codelet_core::compaction::TokenTracker::default();
        
        // CRITICAL: Reinject context reminders so AI retains project context
        inner.inject_context_reminders();
    }
    
    // Reset the interrupt flag
    session_for_control.reset_interrupt();
}
```

### Related Code References

1. **Test file with correct clear behavior:**
   - `codelet/napi/tests/clear_history_context_test.rs`
   - Functions: `test_clear_history_resets_conversation_state`, `test_clear_history_reinjects_context_reminders`

2. **Session struct definition:**
   - `codelet/cli/src/session.rs`
   - Contains: `messages: Vec<Message>`, `turns: Vec<Turn>`, `token_tracker: TokenTracker`

3. **inject_context_reminders method:**
   - Defined in `Session` impl
   - Restores CLAUDE.md and environment info system reminders

### Telegram Bridge Integration

The `/clear` command flows as:
1. User sends `/clear` to Telegram bot
2. `telegram-slash-commands.ts` returns `action: 'clear'`
3. `telegram-endpoint.ts` sends control message via WebSocket
4. Rust bridge relay receives control message
5. Control handler in `session_manager.rs` processes clear action
