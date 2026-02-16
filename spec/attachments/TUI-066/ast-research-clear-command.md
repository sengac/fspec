# AST Research: /clear Command Implementation

## Research Date
2026-02-16

## Work Unit
TUI-066: TUI /clear should update React state as side effect of Rust state change

## Objective
Analyze code structure to understand:
1. Where SessionState enum is defined
2. How clear_history() works
3. Where SessionStateChange chunks are handled

---

## Findings

### 1. SessionState Enum Location

**File:** `codelet/napi/src/types.rs` (line 240)

```rust
#[napi(string_enum)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Running,
    Paused,
    Compacting,
    Interrupted,
}
```

**Action needed:** Add `Cleared` variant to this enum.

---

### 2. clear_history() Implementation

**File:** `codelet/napi/src/session_manager.rs` (line 1424)

```rust
pub fn clear_history(&self) {
    // Clear the output buffer (conversation history display)
    if let Ok(mut buffer) = self.output_buffer.write() {
        buffer.clear();
    }
    
    // Clear actual session state (messages, turns, tokens)
    let mut inner = self.inner.blocking_lock();
    inner.messages.clear();
    inner.turns.clear();
    inner.token_tracker = codelet_core::compaction::TokenTracker::default();
    
    // CRITICAL: Reinject context reminders so AI retains project context
    inner.inject_context_reminders();
    drop(inner);
    
    // Reset the interrupt flag
    self.reset_interrupt();
}
```

**Action needed:** After clearing, emit `StreamChunk::SessionStateChange { state: SessionState::Cleared }`.

---

### 3. SessionStateChange Chunk Handling (TypeScript)

**File:** `src/tui/components/AgentView.tsx`

Two locations handle SessionStateChange:

**Location 1 (line ~3356):**
```typescript
} else if (chunk.type === 'SessionStateChange') {
  // NAPI-010: Internal state change - update state machine, do NOT add to conversation
  if (chunk.state === 'Compacting') {
    // ... compaction handling
  } else {
    compactionRef.current.endCompaction();
  }
  refreshRustState(activeSessionId);
}
```

**Location 2 (line ~4619):**
```typescript
} else if (chunk.type === 'SessionStateChange') {
  if (chunk.state === 'Compacting') {
    // ... compaction handling
  } else {
    compactionRef.current.endCompaction();
  }
  refreshRustState(currentSessionIdRef.current);
}
```

**Action needed:** Add `Cleared` handling in both locations:
```typescript
if (chunk.state === 'Cleared') {
  setConversation([]);
  setTokenUsage({ inputTokens: 0, outputTokens: 0 });
  setContextFillPercentage(0);
}
```

---

### 4. Current /clear Handlers (to be simplified)

**Location 1 (line ~2305):**
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
  // REMOVE THESE - will come from chunk handler
  setConversation([]);
  setTokenUsage({ inputTokens: 0, outputTokens: 0 });
  setContextFillPercentage(0);
  return;
}
```

**Location 2 (line ~3782):**
Identical code - same changes needed.

---

## Implementation Plan

1. **Rust (types.rs):** Add `Cleared` to SessionState enum
2. **Rust (session_manager.rs):** Emit chunk in clear_history()
3. **TypeScript (AgentView.tsx):** Handle 'Cleared' state in both SessionStateChange handlers
4. **TypeScript (AgentView.tsx):** Remove manual state updates from both /clear handlers

## Related Code Patterns

The pattern follows existing SessionStateChange handling for 'Compacting' state, which is already correctly implemented. We're extending the same pattern for 'Cleared'.
