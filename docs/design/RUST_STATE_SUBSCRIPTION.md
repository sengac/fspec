# Rust State Subscription Architecture for React

## Problem Statement

When users switch sessions while an AI agent is responding:
1. The response chunks go to the `output_buffer` in Rust (stored correctly)
2. BUT the TypeScript callback is detached, so TypeScript never receives them
3. The messages are NEVER persisted to `messages.jsonl` because persistence happens in the TypeScript callback
4. When the user switches back, buffered content is **displayed** from the buffer but **NOT persisted**
5. If the app restarts, those messages are lost forever

### Evidence from User Bug Report

```
Session 4b2d574b (Claude/anthropic):
- User message: "what is in your context?" at 10:24:49
- Next user message: "hello?" at 10:26:42
- NO assistant response persisted between them!

Session 2ef5e698 (GLM/zai):
- User message: "what is in your context?" at 10:24:55
- Assistant response: exists at 10:25:08
- This worked because user didn't switch AWAY from GLM during response
```

The user asked the same question to TWO sessions (Claude and GLM) almost simultaneously. When they switched away from Claude to check GLM's response, Claude's response was lost because the callback was detached.

## Root Cause

The current architecture has a fundamental flaw: **persistence is a side-effect of the UI callback**, not a core system responsibility.

```
Current Flow:
┌─────────┐     ┌─────────────┐     ┌────────────────┐     ┌─────────────┐
│ Rust    │────▶│ Callback    │────▶│ TypeScript UI  │────▶│ Persistence │
│ Agent   │     │ (if attached)     │ (side effect)  │     │ (TS-driven) │
└─────────┘     └─────────────┘     └────────────────┘     └─────────────┘
                    │
                    ▼ (if detached)
              ┌─────────────┐
              │ Buffer Only │ ← Messages are lost here!
              │ (no persist)│
              └─────────────┘
```

## Proposed Solution: Rust-Side Persistence + React `useSyncExternalStore`

### 1. Move Persistence to Rust

Instead of TypeScript calling `persistenceStoreMessageEnvelope`, Rust should persist messages directly in `BackgroundOutput::emit()`:

```rust
// In session_manager.rs - BackgroundOutput
impl StreamOutput for BackgroundOutput {
    fn emit(&self, event: StreamEvent) {
        let chunk = match event {
            StreamEvent::Text(text) => {
                // Persist text content to message store
                self.session.persist_text_chunk(&text);
                StreamChunk::text(text)
            }
            StreamEvent::Done => {
                // Finalize and persist assistant message envelope
                self.session.finalize_assistant_message();
                StreamChunk::done()
            }
            // ... other events
        };
        
        // Buffer for UI display (existing logic)
        self.session.handle_output(chunk);
    }
}
```

### 2. Use `useSyncExternalStore` for React State

React 18's `useSyncExternalStore` is designed exactly for this use case - subscribing to external data stores that change outside of React's control.

```typescript
// src/tui/hooks/useRustSessionState.ts
import { useSyncExternalStore } from 'react';
import { sessionSubscribe, sessionGetSnapshot } from '@anthropic/codelet-napi';

export function useSessionMessages(sessionId: string) {
  const subscribe = (callback: () => void) => {
    return sessionSubscribe(sessionId, callback);
  };
  
  const getSnapshot = () => {
    return sessionGetSnapshot(sessionId);
  };
  
  return useSyncExternalStore(subscribe, getSnapshot);
}
```

### 3. Rust Subscription System

Add a subscription mechanism to `BackgroundSession`:

```rust
// In session_manager.rs
pub struct BackgroundSession {
    // ... existing fields ...
    
    /// Subscribers for state changes (React useSyncExternalStore)
    subscribers: RwLock<Vec<ThreadsafeFunction<(), ()>>>,
    
    /// Message version counter (increments on each state change)
    message_version: AtomicU64,
}

impl BackgroundSession {
    /// Subscribe to state changes
    pub fn subscribe(&self, callback: ThreadsafeFunction<(), ()>) -> SubscriptionHandle {
        let mut subs = self.subscribers.write().unwrap();
        let id = subs.len();
        subs.push(callback);
        SubscriptionHandle { session_id: self.id, subscription_id: id }
    }
    
    /// Notify all subscribers of state change
    fn notify_subscribers(&self) {
        self.message_version.fetch_add(1, Ordering::Release);
        let subs = self.subscribers.read().unwrap();
        for sub in subs.iter() {
            let _ = sub.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
    
    /// Get current state snapshot (for useSyncExternalStore)
    pub fn get_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            version: self.message_version.load(Ordering::Acquire),
            messages: self.get_persisted_messages(),
            tokens: self.get_tokens(),
            status: self.get_status(),
        }
    }
}
```

### 4. NAPI Bindings

```rust
#[napi]
pub fn session_subscribe(
    session_id: String,
    callback: ThreadsafeFunction<(), ()>
) -> Result<SubscriptionHandle> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.subscribe(callback))
}

#[napi]
pub fn session_unsubscribe(handle: SubscriptionHandle) -> Result<()> {
    let session = SessionManager::instance().get_session(&handle.session_id)?;
    session.unsubscribe(handle.subscription_id);
    Ok(())
}

#[napi(object)]
pub struct SessionSnapshot {
    pub version: u64,
    pub messages: Vec<MessageEnvelope>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub status: String,
}

#[napi]
pub fn session_get_snapshot(session_id: String) -> Result<SessionSnapshot> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_snapshot())
}
```

## Architecture Diagram

```
New Flow:
┌─────────────┐     ┌─────────────────────┐     ┌──────────────────┐
│ Rust Agent  │────▶│ BackgroundOutput    │────▶│ Persistence      │
│             │     │ emit()              │     │ (Rust-driven)    │
└─────────────┘     └─────────────────────┘     └──────────────────┘
                           │                            │
                           │ notify_subscribers()       │
                           ▼                            ▼
                    ┌─────────────┐              ┌─────────────┐
                    │ Subscribers │◀────────────│ Message     │
                    │ (TSFn)      │              │ Store       │
                    └─────────────┘              └─────────────┘
                           │
                           ▼
                    ┌─────────────────────────────────────────┐
                    │ React useSyncExternalStore              │
                    │ - subscribe: sessionSubscribe           │
                    │ - getSnapshot: sessionGetSnapshot       │
                    └─────────────────────────────────────────┘
                           │
                           ▼
                    ┌─────────────────────────────────────────┐
                    │ React Component Re-render               │
                    │ - Messages always from Rust             │
                    │ - No lost messages on session switch    │
                    └─────────────────────────────────────────┘
```

## Key Benefits

1. **Messages are NEVER lost** - Persistence happens in Rust, not dependent on UI callbacks
2. **Single source of truth** - Rust owns all state, React just subscribes
3. **Efficient updates** - `useSyncExternalStore` only re-renders when snapshot changes
4. **Concurrent-safe** - Works correctly with React 18 concurrent features
5. **No race conditions** - Rust handles all state transitions atomically

## Implementation Steps

### Phase 1: Rust-Side Persistence (Critical Fix)
1. Add message accumulator to `BackgroundSession`
2. Persist messages in `BackgroundOutput::emit()` instead of relying on TypeScript
3. Create `session_get_messages(session_id)` that returns persisted messages

### Phase 2: Subscription System
1. Add subscription management to `BackgroundSession`
2. Implement `session_subscribe` / `session_unsubscribe` NAPI bindings
3. Implement `session_get_snapshot` NAPI binding

### Phase 3: React Integration
1. Create `useRustSessionState` hook using `useSyncExternalStore`
2. Refactor `AgentView` to use the hook instead of local state
3. Remove TypeScript persistence calls (now handled by Rust)

### Phase 4: Cleanup
1. Remove `persistenceStoreMessageEnvelope` from TypeScript (or keep as backup)
2. Update tests
3. Migration for existing sessions

## References

- [React useSyncExternalStore](https://react.dev/reference/react/useSyncExternalStore)
- [napi-rs ThreadsafeFunction](https://napi.rs/docs/concepts/threadsafe-function)
- [Functions and Callbacks in NAPI-RS](https://napi.rs/blog/function-and-callbacks)

## Alternative: Immediate Fix (Without Full Refactor)

If the full refactor is too risky, a simpler fix is to persist buffered messages when resuming:

```typescript
// In resumeSessionById, after getting mergedChunks:
const mergedChunks = sessionGetMergedOutput(sessionId);

// Persist any chunks that weren't persisted during detach
for (const chunk of mergedChunks) {
  if (chunk.type === 'Text' || chunk.type === 'ToolCall' || chunk.type === 'ToolResult') {
    // Check if already persisted, if not, persist now
    await persistMissingChunk(sessionId, chunk);
  }
}
```

This is a band-aid but would prevent data loss in the short term.
