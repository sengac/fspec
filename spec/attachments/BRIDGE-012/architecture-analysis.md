# BRIDGE-012: Global Chunk Callback Architecture

## Overview

This document describes the new architecture that replaces per-session attach/detach with a single global callback.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Rust BackgroundSession                        │
├─────────────────────────────────────────────────────────────────┤
│  handle_output(chunk)                                            │
│    │                                                             │
│    ├──► buffer.push(chunk)           (always)                    │
│    │                                                             │
│    ├──► watcher_broadcast.send()     (always - for watchers)     │
│    │                                                             │
│    └──► GLOBAL_CALLBACK(self.id, chunk)  (always - for TS)       │
│         (no attachment state, no session gating)                 │
└─────────────────────────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────────┐
│               TypeScript GlobalSessionStreamManager              │
├─────────────────────────────────────────────────────────────────┤
│  Global callback registered ONCE at app startup                  │
│                                                                  │
│  handleChunk(sessionId, chunk)                                   │
│    │                                                             │
│    ├──► Look up handlers in sessionHandlers.get(sessionId)       │
│    │                                                             │
│    └──► Invoke only handlers for THIS sessionId                  │
│                                                                  │
│  Session isolation via Map lookup, NOT Rust gating               │
└─────────────────────────────────────────────────────────────────┘
              │
      ┌───────┴───────┐
      ▼               ▼
┌─────────────┐  ┌─────────────┐
│  AgentView  │  │   Bridge    │
├─────────────┤  ├─────────────┤
│ Registers   │  │ Filters by  │
│ handler for │  │ bridged     │
│ viewed      │  │ session_id  │
│ session_id  │  │ from tool   │
│             │  │ call        │
└─────────────┘  └─────────────┘
```

## Code Changes Required

### Rust: BackgroundSession

**DELETE:**
- `is_attached: AtomicBool` field
- `attached_callback: RwLock<Option<ThreadsafeFunction<StreamChunk>>>` field
- `pub fn is_attached(&self) -> bool` method
- `pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>)` method
- `pub fn detach(&self)` method

**MODIFY handle_output():**
```rust
pub fn handle_output(&self, chunk: StreamChunk) {
    // Buffer (unchanged)
    buffer.push(chunk.clone());
    
    // Broadcast to watchers (unchanged)
    let _ = self.watcher_broadcast.send(chunk.clone());
    
    // Call global callback with session_id (NEW)
    if let Some(cb) = GLOBAL_CHUNK_CALLBACK.get() {
        let _ = cb.call((self.id.to_string(), chunk), ThreadsafeFunctionCallMode::NonBlocking);
    }
}
```

### Rust: NAPI Functions

**DELETE:**
- `pub fn session_attach(session_id: String, callback: ...) -> Result<()>`
- `pub fn session_detach(session_id: String) -> Result<()>`

**ADD:**
```rust
static GLOBAL_CHUNK_CALLBACK: OnceCell<ThreadsafeFunction<(String, StreamChunk)>> = OnceCell::new();

#[napi]
pub fn session_set_global_chunk_callback(callback: ThreadsafeFunction<(String, StreamChunk)>) -> Result<()> {
    GLOBAL_CHUNK_CALLBACK.set(callback).map_err(|_| {
        napi::Error::from_reason("Global callback already set")
    })
}
```

### TypeScript: GlobalSessionStreamManager

**BEFORE:**
```typescript
private async attachToSession(sessionId: string): Promise<void> {
    napi.sessionAttach(sessionId, (err, chunk) => {
        this.handleChunk(sessionId, chunk);
    });
}
```

**AFTER:**
```typescript
public static init(): void {
    napi.sessionSetGlobalChunkCallback((sessionId: string, chunk: StreamChunk) => {
        GlobalSessionStreamManager.getInstance().handleChunk(sessionId, chunk);
    });
}
```

**handleChunk() remains the same** - it already routes by sessionId via Map lookup.

### TypeScript: Remove All sessionAttach/sessionDetach Usage

Search and remove from:
- `src/tui/services/globalSessionStreamManager.ts`
- `src/tui/hooks/sessionAttachment.ts`
- All test files that mock these functions

## Test Files

| Feature File | Test File |
|--------------|-----------|
| `global-chunk-callback-napi.feature` | `codelet/napi/src/session_manager.rs` |
| `global-session-stream-manager-chunk-routing.feature` | `src/tui/services/__tests__/globalSessionStreamManager.test.ts` |
| `tui-session-chunk-filtering.feature` | `src/tui/__tests__/tui-session-chunk-filtering.test.ts` |
| `bridge-session-chunk-filtering.feature` | TBD (bridge tests) |

## Migration Strategy

- **No backwards compatibility** - complete replacement
- **No feature flags** - single PR
- **No fallbacks** - clean cut
- **No comments about old system** - remove all references
