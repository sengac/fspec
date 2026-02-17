# BRIDGE-012: Remove is_attached Gating from Rust Chunk Forwarding

## Problem Statement

When input comes from the Telegram bridge, the TUI doesn't show Claude's response text, but Telegram does. The bridge input message appears in the TUI, but the subsequent LLM response chunks are missing.

## Observed Behavior

| Source | Input Shown | Response Shown |
|--------|-------------|----------------|
| TUI keyboard | ✅ Yes | ✅ Yes |
| Telegram bridge | ✅ Yes | ❌ No |

## Root Cause Analysis

### The Problem: `is_attached` Gating

In `codelet/napi/src/session_manager.rs`, the `handle_output()` method has two paths:

```rust
pub fn handle_output(&self, chunk: StreamChunk) {
    // Always buffer (unbounded)
    buffer.push(chunk.clone());

    // PATH 1: Broadcast to watcher sessions - NO GATING
    let _ = self.watcher_broadcast.send(chunk.clone());
    
    // PATH 2: Forward to TUI callback - GATED BY is_attached
    if is_attached {
        callback.call(chunk);  // TUI receives chunk
    } else {
        // Chunks are DROPPED here - never reach TypeScript
    }
}
```

### Why Telegram Works But TUI Doesn't

1. **Telegram bridge** receives chunks via `watcher_broadcast.send()` which has **no gating**
2. **TUI** receives chunks via `attached_callback.call()` which is **gated by `is_attached`**

When `is_attached` is false, chunks are dropped before they ever reach TypeScript.

## Business Rules

1. **Rust exposes a single global callback that TypeScript registers once at startup.** This callback receives `(session_id, chunk)` for ALL chunks from ALL sessions. Remove per-session `attach()/detach()` pattern entirely.

2. **TypeScript is responsible for routing and displaying chunks based on session_id** - Rust has no knowledge of which session is "active".

## Examples (Acceptance Tests)

1. When bridge sends input, the TUI shows both the bridge input AND the LLM response chunks in the conversation
2. When user types in TUI, the LLM response chunks appear in the conversation

## Architecture Notes

1. **session_id routing already works** - Captured in TypeScript closure and passed to `sessionSendFspecResult` for tool callbacks
2. **PROBLEM** - `is_attached` gating in `handle_output()` drops chunks BEFORE they reach TypeScript. `watcher_broadcast` bypasses this (so Telegram works), but `attached_callback` is gated (so TUI doesn't)
3. **SOLUTION** - One global callback that receives `(session_id, chunk)` for ALL sessions

## Proposed Solution

### New Architecture: Pub/Sub (Observer Pattern)

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust BackgroundSession                    │
├─────────────────────────────────────────────────────────────┤
│  handle_output(chunk)                                        │
│    │                                                         │
│    ├──► buffer.push(chunk)           (always)                │
│    │                                                         │
│    ├──► watcher_broadcast.send()     (always - for watchers) │
│    │                                                         │
│    └──► GLOBAL_CALLBACK(session_id, chunk)  (always - for TS)│
│         (no is_attached check)                               │
└─────────────────────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│               TypeScript GlobalSessionStreamManager          │
├─────────────────────────────────────────────────────────────┤
│  handleChunk(sessionId, chunk)                               │
│    │                                                         │
│    ├──► Route to session-specific handlers                   │
│    │                                                         │
│    └──► Display based on active session                      │
└─────────────────────────────────────────────────────────────┘
```

### Why This is SOLID

- **Single Responsibility**: Rust emits, TypeScript routes/displays
- **Open/Closed**: Add new consumers without modifying Rust
- **Dependency Inversion**: Both sides depend on the abstraction (chunk stream)

### Why This is DRY

- One emission point in Rust
- No per-session callback registration/deregistration
- No `is_attached` tracking per session

### Why This is Composable

- Multiple subscribers can listen to the same stream
- TUI subscribes → routes to UI
- Telegram bridge subscribes → forwards to WebSocket
- Future debugger subscribes → logs to file

## Dead Code to Remove

- `is_attached: AtomicBool` field
- `attached_callback: RwLock<Option<ThreadsafeFunction<StreamChunk>>>` field
- `pub fn is_attached(&self) -> bool` method
- `pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>)` method
- `pub fn detach(&self)` method
- Per-session `session_attach()` / `session_detach()` NAPI functions

## Files Affected

### Rust
- `codelet/napi/src/session_manager.rs` - BackgroundSession, handle_output()

### TypeScript
- `src/tui/services/globalSessionStreamManager.ts` - Update for new callback signature
- `src/tui/services/sessionService.ts` - Remove per-session subscribe calls
- `src/tui/hooks/useSessionStreamManager.ts` - Simplify

## Testing Considerations

1. Bridge input → TUI shows response
2. Keyboard input → TUI shows response (no regression)
3. Multiple sessions → Correct routing to each
4. Session switching → No lost chunks during transition
