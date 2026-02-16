# TUI-066: Architecture Analysis - /clear Command State Management

## Problem Summary

The TUI `/clear` command manually updates React state after calling the Rust NAPI function. This violates the single-source-of-truth principle and creates DRY violations.

## Current (Broken) Architecture

```
TUI /clear → React handler → {
  1. Call NAPI sessionClearHistory()  ← Rust state updated
  2. setConversation([])              ← React state updated MANUALLY
  3. setTokenUsage({...})             ← React state updated MANUALLY  
  4. setContextFillPercentage(0)      ← React state updated MANUALLY
}
```

### Problems with Current Approach

1. **DRY Violation**: Two `/clear` handlers exist in `AgentView.tsx`:
   - Line ~2305 in `handleSubmit`
   - Line ~3782 in `handleSubmitWithCommand`
   
2. **State Desync Risk**: React state is updated manually, separate from Rust state changes. If either update fails or is missed, states diverge.

3. **Inconsistent with Bridge**: The Telegram bridge does it correctly - it sends a control message to Rust, Rust updates state, and any UI updates flow from that.

## Correct Architecture (Like Bridge)

```
TUI /clear → Call NAPI sessionClearHistory() → Rust clears state → 
  Rust emits "SessionCleared" chunk → TUI stream handler receives chunk → 
  React state updated AS SIDE EFFECT
```

### How Bridge Does It Correctly

**File: `bridge/telegram-endpoint.ts` (lines 675-684)**
```typescript
if (result.action && state.currentSession.ws) {
  const actionMap: Record<string, string> = {
    stop: 'interrupt',
    clear: 'clear',
  };
  sendControlMessage(
    state.currentSession.ws,
    state.currentSession.sessionId || '',
    actionMap[result.action]
  );
}
```

**File: `codelet/napi/src/session_manager.rs` (lines 4540-4548)**
```rust
"clear" => {
    tracing::info!("Bridge control: clearing session");
    tokio::task::block_in_place(|| {
        // DRY: Use the shared clear_history method
        session_for_control.clear_history();
    });
}
```

The Bridge sends a control message → Rust handles it → Rust is the single source of truth.

## Code Locations

### TUI Handlers (both need to be consolidated)

**`src/tui/components/AgentView.tsx`**

Handler 1 (~line 2305):
```typescript
if (userMessage === '/clear') {
  setInputValue('');
  if (currentSessionId) {
    try {
      sessionClearHistory(currentSessionId);
    } catch (err) {
      logger.error('[AgentView] Failed to clear session history:', err);
    }
  }
  setConversation([]);
  setTokenUsage({ inputTokens: 0, outputTokens: 0 });
  setContextFillPercentage(0);
  return;
}
```

Handler 2 (~line 3782):
```typescript
if (userMessage === '/clear') {
  setInputValue('');
  if (currentSessionId) {
    try {
      sessionClearHistory(currentSessionId);
    } catch (err) {
      logger.error('[AgentView] Failed to clear session history:', err);
    }
  }
  setConversation([]);
  setTokenUsage({ inputTokens: 0, outputTokens: 0 });
  setContextFillPercentage(0);
  return;
}
```

### Rust clear_history Method

**`codelet/napi/src/session.rs`** (from AST research):
```rust
pub fn clear_history(&self) -> Result<()> {
    let mut session = self.inner.blocking_lock();

    session.messages.clear();
    session.turns.clear();
    session.token_tracker = codelet_core::compaction::TokenTracker::default();

    // Reinject context reminders to restore CLAUDE.md and environment info
    session.inject_context_reminders();

    Ok(())
}
```

## Proposed Solution

1. **Add `SessionCleared` chunk type** to the stream protocol (or reuse existing chunk type)

2. **Modify `session_clear_history` NAPI** to emit a chunk after clearing:
   ```rust
   pub fn clear_history(&self) -> Result<()> {
       // ... existing clear logic ...
       
       // Emit chunk so UI can react
       self.emit_chunk(StreamChunk::SessionCleared {
           tokens: TokenUsage { input: 0, output: 0 },
       });
       
       Ok(())
   }
   ```

3. **TUI stream handler** reacts to `SessionCleared` chunk:
   ```typescript
   case 'SessionCleared':
     setConversation([]);
     setTokenUsage({ inputTokens: 0, outputTokens: 0 });
     setContextFillPercentage(0);
     break;
   ```

4. **TUI `/clear` handler** becomes trivial:
   ```typescript
   if (userMessage === '/clear') {
     setInputValue('');
     if (currentSessionId) {
       sessionClearHistory(currentSessionId);
     }
     return;
   }
   ```

## Related Work Units

- **TUI-065**: Original bug - `/clear` wasn't calling `sessionClearHistory()` in one of the handlers
- **BRIDGE-008**: Bridge control channel implementation (shows correct pattern)
- **BRIDGE-010**: Telegram slash commands

## Files to Modify

1. `codelet/napi/src/session_manager.rs` - Add chunk emission after clear
2. `codelet/napi/src/lib.rs` - Add `SessionCleared` to `StreamChunk` enum if needed
3. `src/tui/components/AgentView.tsx` - Handle chunk in stream handler, simplify `/clear` handlers
4. Possibly consolidate the two `/clear` handlers into the shared `handleClearCommand` helper that was added
