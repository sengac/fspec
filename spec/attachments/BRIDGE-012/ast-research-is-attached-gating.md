# AST Research: is_attached Gating in Rust Chunk Forwarding

## Research Summary

This document captures AST-based code analysis for BRIDGE-012 to understand the `is_attached` gating pattern that causes chunks to be dropped before reaching TypeScript.

## Key Findings

### 1. handle_output() Two-Path Emission

**File**: `codelet/napi/src/session_manager.rs`

The `handle_output()` method emits chunks via two paths:

| Path | Line | Gating | Recipient |
|------|------|--------|-----------|
| `watcher_broadcast.send()` | 1118 | **None** | Bridge/Telegram |
| `cb.call()` (attached_callback) | 1127 | **is_attached check** | TUI |

### 2. AST Pattern Matches

#### is_attached Check (The Problem)
```
Pattern: self.is_attached()
Match: line 1123, column 27
```
This is the gating condition that drops chunks when `is_attached` is false.

#### Ungated Broadcast (Why Telegram Works)
```
Pattern: self.watcher_broadcast.send($$$ARGS)
Match: line 1118, column 17
  → self.watcher_broadcast.send(chunk.clone())
```
This always sends chunks to watchers/bridges with no gating.

#### Gated Callback (Why TUI Drops Chunks)
```
Pattern: cb.call($$$ARGS)
Match: line 1127, column 25
  → cb.call(Ok(chunk), ThreadsafeFunctionCallMode::NonBlocking)
```
This only executes when `is_attached` is true.

### 3. attach()/detach() Pattern

#### Setting is_attached to true
```
Pattern: self.is_attached.store($$$ARGS)
Match: line 1152, column 9
  → self.is_attached.store(true, Ordering::Release)
```
Called in `attach()` method after setting callback.

#### Setting is_attached to false
```
Pattern: self.is_attached.store($$$ARGS)
Match: line 1160, column 9
  → self.is_attached.store(false, Ordering::Release)
```
Called in `detach()` method after clearing callback.

### 4. NAPI Entry Points

#### session_attach NAPI function
```
Pattern: session.attach($$$ARGS)
Matches:
  - line 5190: session.attach(callback)  // session_attach
  - line 5218: session.attach(callback)  // session_subscribe
```

#### session_detach NAPI function
```
Pattern: session.detach()
Matches:
  - line 5201: session.detach()  // session_detach
  - line 5233: session.detach()  // session_unsubscribe
```

### 5. TypeScript Consumer

**File**: `src/tui/services/globalSessionStreamManager.ts`

```
Pattern: napi.sessionAttach($$$ARGS)
Match: line 119, column 7
  → napi.sessionAttach(sessionId, (err: Error | null, chunk: StreamChunk) => { ... })
```

`GlobalSessionStreamManager` calls `sessionAttach` and becomes the sole callback owner.

## Root Cause Diagram

```
handle_output(chunk)
    │
    ├─► watcher_broadcast.send(chunk)  ─── NO GATE ───► Bridge/Telegram ✅
    │
    └─► if is_attached {                ─── GATED ────► TUI ❌ (when false)
            cb.call(chunk)
        }
```

## Solution: Global Callback Architecture

Replace per-session `attached_callback` with a single global callback that receives `(session_id, chunk)` for all sessions. TypeScript handles all routing logic. Rust becomes a pure emitter with zero attachment state.

### Why Global Callback (Not Just Removing the Check)

Simply removing the `is_attached` check would fix the immediate problem, but leaves unnecessary complexity:
- Per-session callbacks that are never used independently
- Attach/detach lifecycle that serves no purpose
- `GlobalSessionStreamManager` already handles routing anyway

The global callback approach:
- **Simpler Rust code** - one callback, no per-session state
- **Single source of truth** - TypeScript owns ALL routing logic
- **Clean architecture** - Rust emits, TypeScript decides what to do

## Files to Modify

### Rust: DELETE

| File | Item | Line |
|------|------|------|
| `codelet/napi/src/session_manager.rs` | `is_attached: AtomicBool` field | 847 |
| `codelet/napi/src/session_manager.rs` | `attached_callback: RwLock<Option<...>>` field | 856 |
| `codelet/napi/src/session_manager.rs` | `pub fn is_attached(&self) -> bool` method | 1085 |
| `codelet/napi/src/session_manager.rs` | `pub fn attach(&self, ...)` method | 1149 |
| `codelet/napi/src/session_manager.rs` | `pub fn detach(&self)` method | 1158 |
| `codelet/napi/src/session_manager.rs` | `session_attach` NAPI function | 5182 |
| `codelet/napi/src/session_manager.rs` | `session_detach` NAPI function | 5198 |

### Rust: ADD

| File | Item |
|------|------|
| `codelet/napi/src/session_manager.rs` | `static GLOBAL_CHUNK_CALLBACK: OnceCell<ThreadsafeFunction<(String, StreamChunk)>>` |
| `codelet/napi/src/session_manager.rs` | `#[napi] pub fn session_set_global_chunk_callback(...)` |

### Rust: MODIFY

| File | Method | Change |
|------|--------|--------|
| `codelet/napi/src/session_manager.rs` | `handle_output()` | Remove is_attached check, call global callback with `(self.id, chunk)` |

### TypeScript: DELETE

| File | Item |
|------|------|
| `src/tui/services/globalSessionStreamManager.ts` | All `sessionAttach` calls |
| `src/tui/services/globalSessionStreamManager.ts` | All `sessionDetach` calls |
| Any imports | `sessionAttach`, `sessionDetach` from `@sengac/codelet-napi` |

### TypeScript: ADD

| File | Item |
|------|------|
| `src/tui/services/globalSessionStreamManager.ts` | Call `sessionSetGlobalChunkCallback` once at init |

## Migration Strategy

- **No backwards compatibility** - complete replacement
- **No feature flags** - single PR
- **No fallbacks** - clean cut
- **No comments about old system** - remove all references
