# BUG-126: Fix TOOL_PROGRESS_CALLBACK Per-Session Isolation

## Root Cause

`codelet/tools/src/tool_progress.rs:34` defines a **process-global singleton**:

```rust
static TOOL_PROGRESS_CALLBACK: RwLock<Option<ToolProgressCallback>> = RwLock::new(None);
```

When multiple `BackgroundSession` instances run concurrently (via AgentManager subordinates), each session's agent loop calls `set_tool_progress_callback(Some(...))` which **overwrites** the callback for all other sessions. This causes:

1. Bash tool streaming output (`[Tool output]`) appearing in the **wrong** session's TUI
2. Tool progress going **nowhere** (silently lost) when the overwriting session finishes and clears the callback

## Architecture Context

The intended data flow for tool progress:

```
BashTool (tokio task) → emit_tool_progress() → GLOBAL callback → emitter.emit_tool_progress()
                                                                   ↓
                                                    BackgroundOutput.emit() (NAPI mode)
                                                    OR TerminalOutput.emit() (CLI mode)
                                                                   ↓
                                                    session.handle_output(StreamChunk::tool_progress(...))
                                                                   ↓
                                                    GLOBAL_CHUNK_CALLBACK (routes by session_id to TypeScript)
                                                                   ↓
                                                    AgentView.tsx renders "[Tool output]"
```

The `BackgroundProgressEmitter` (created per-session) correctly routes output to its owning session. The bug is that the **single global callback slot** means only ONE session's emitter is active at a time.

## Files to Modify

### 1. `codelet/tools/src/tool_progress.rs` (PRIMARY)

**Current broken code (line 34):**
```rust
static TOOL_PROGRESS_CALLBACK: RwLock<Option<ToolProgressCallback>> = RwLock::new(None);
```

**Fix — convert to per-session HashMap:**
```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;
use uuid::Uuid;

static TOOL_PROGRESS_CALLBACKS: Lazy<RwLock<HashMap<Uuid, ToolProgressCallback>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
```

**Update `set_tool_progress_callback` (lines 44-48):**
```rust
// OLD signature:
pub fn set_tool_progress_callback(callback: Option<ToolProgressCallback>)

// NEW signature:
pub fn set_tool_progress_callback(session_id: Uuid, callback: Option<ToolProgressCallback>)
```

Implementation:
```rust
pub fn set_tool_progress_callback(session_id: Uuid, callback: Option<ToolProgressCallback>) {
    if let Ok(mut guard) = TOOL_PROGRESS_CALLBACKS.write() {
        match callback {
            Some(cb) => { guard.insert(session_id, cb); }
            None => { guard.remove(&session_id); }
        }
    }
}
```

**Update `emit_tool_progress` (lines 58-64):**
```rust
// OLD signature:
pub fn emit_tool_progress(output_chunk: &str, is_stderr: bool)

// NEW signature:
pub fn emit_tool_progress(session_id: Uuid, output_chunk: &str, is_stderr: bool)
```

Implementation:
```rust
pub fn emit_tool_progress(session_id: Uuid, output_chunk: &str, is_stderr: bool) {
    if let Ok(guard) = TOOL_PROGRESS_CALLBACKS.read() {
        if let Some(callback) = guard.get(&session_id) {
            callback(output_chunk, is_stderr);
        }
    }
}
```

**Update `Cargo.toml`:** Ensure `uuid` and `once_cell` are in dependencies (they likely already are).

### 2. `codelet/tools/src/lib.rs` (RE-EXPORTS)

**Current (line 169):**
```rust
pub use tool_progress::{emit_tool_progress, set_tool_progress_callback, ToolProgressCallback};
```

No change to the re-export itself, but callers will need to pass `session_id`.

### 3. `codelet/tools/src/bash.rs` (EMIT CALL SITES)

The `BashTool` struct already has a `session_id: Uuid` field (confirmed). The `emit_tool_progress` calls are inside spawned tokio tasks that currently have NO session context.

**spawn_stderr_reader (line ~396-407):**

Current:
```rust
fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    buffer: Arc<Mutex<String>>,
    stream_to_ui: bool,
) -> tokio::task::JoinHandle<()> {
```

Fix — add `session_id` parameter:
```rust
fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    buffer: Arc<Mutex<String>>,
    stream_to_ui: bool,
    session_id: Uuid,
) -> tokio::task::JoinHandle<()> {
```

And update the call inside (line ~407):
```rust
// OLD:
emit_tool_progress(&line_with_newline, true);

// NEW:
emit_tool_progress(session_id, &line_with_newline, true);
```

**BashTool::call stdout task (line ~680):**

The `Tool::call()` method has access to `self.session_id`. Pass it into the spawned task:

```rust
// OLD (line ~680):
emit_tool_progress(&line_with_newline, false);

// NEW:
emit_tool_progress(session_id, &line_with_newline, false);
```

**call_with_streaming stdout reader (if separate):**

Search for all `emit_tool_progress` calls in bash.rs and update each one.

**Caller of spawn_stderr_reader** — update the call site to pass `self.session_id`.

### 4. `codelet/cli/src/interactive/stream_loop.rs` (SET/CLEAR CALL SITES)

**SET callback (lines 432-436):**

Current:
```rust
if let Some(emitter) = output.progress_emitter() {
    set_tool_progress_callback(Some(Arc::new(move |chunk: &str, is_stderr: bool| {
        emitter.emit_tool_progress("", "bash", chunk, is_stderr);
    })));
}
```

Fix — need `session_id` from the session. The function `run_agent_stream_internal` has access to `session: &mut Session`. Add session_id extraction:

```rust
if let Some(emitter) = output.progress_emitter() {
    let sid = session.id(); // or however the session ID is accessed
    set_tool_progress_callback(sid, Some(Arc::new(move |chunk: &str, is_stderr: bool| {
        emitter.emit_tool_progress("", "bash", chunk, is_stderr);
    })));
}
```

**IMPORTANT**: Check how `Session` exposes its ID. The `Session` struct in `codelet/cli/src/session.rs` should have an `id` field or method. If not, it may need to be added or the session_id should be threaded through `run_agent_stream_internal` as a parameter.

**CLEAR callback (line 1486):**
```rust
// OLD:
set_tool_progress_callback(None);

// NEW:
set_tool_progress_callback(sid, None);
```

**IMPORTANT**: The `sid` variable needs to be available at line 1486. Store it earlier in the function scope.

### 5. `codelet/cli/src/interactive/gemini_continuation.rs` (CLEAR CALL SITES)

**Lines 203, 325, 344, 348, 367** — all call `set_tool_progress_callback(None)`.

Each needs the session_id. The function signature for `handle_gemini_continuation` should receive the session_id (or extract it from the session parameter).

```rust
// OLD (at each of 5 locations):
set_tool_progress_callback(None);

// NEW:
set_tool_progress_callback(session_id, None);
```

### 6. `codelet/cli/src/interactive/compaction_retry.rs` (CLEAR CALL SITE)

**Line 374:**
```rust
// OLD:
set_tool_progress_callback(None);

// NEW:
set_tool_progress_callback(session_id, None);
```

## Reference Pattern (Already Working)

Use `FSPEC_HANDLERS` in `codelet/tools/src/fspec_handler.rs` as the canonical reference:

```rust
// fspec_handler.rs — correct per-session pattern
static FSPEC_HANDLERS: Lazy<RwLock<HashMap<Uuid, FspecHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn set_fspec_handler_for_session(session_id: Uuid, handler: Option<FspecHandler>) {
    if let Ok(mut guard) = FSPEC_HANDLERS.write() {
        match handler {
            Some(h) => { guard.insert(session_id, h); }
            None => { guard.remove(&session_id); }
        }
    }
}
```

Other correctly-implemented per-session handlers to reference:
- `SESSION_SEARCH_HANDLERS` in `codelet/tools/src/session_search/handler.rs`
- `DEEP_SEARCH_HANDLERS` in `codelet/tools/src/deep_search/mod.rs`
- `INJECT_SUMMARY_HANDLERS` in `codelet/tools/src/inject_summary.rs`
- `AGENT_MANAGER_HANDLERS` in `codelet/tools/src/agent_manager.rs`
- `SCHEDULE_HANDLERS` in `codelet/tools/src/schedule_handler.rs`

## Tests to Update

### Existing tests in `tool_progress.rs` (lines 66-166)

All 4 test functions use the old API. Update:

```rust
#[test]
fn test_emit_with_no_callback_is_noop() {
    let _guard = TEST_LOCK.lock().unwrap();
    let session_id = Uuid::new_v4();
    set_tool_progress_callback(session_id, None);
    // Should not panic
    emit_tool_progress(session_id, "output", false);
}
```

### New tests to add

1. **Test session isolation**: Register callbacks for session A and session B with different captured vectors. Emit via session A. Assert only A's vector has output.
2. **Test cleanup doesn't affect other sessions**: Register A and B. Clear A. Assert B still works.
3. **Test concurrent access**: Use multiple threads to register/emit/clear for different sessions.

## Checklist

- [ ] Modify `tool_progress.rs`: Replace `RwLock<Option<...>>` with `Lazy<RwLock<HashMap<Uuid, ...>>>`
- [ ] Update `set_tool_progress_callback` signature to accept `session_id: Uuid`
- [ ] Update `emit_tool_progress` signature to accept `session_id: Uuid`
- [ ] Update `bash.rs`: Thread `session_id` through `spawn_stderr_reader`, `spawn_stdout_reader`, and all `emit_tool_progress` calls
- [ ] Update `stream_loop.rs` line 433: Pass session_id when setting callback
- [ ] Update `stream_loop.rs` line 1486: Pass session_id when clearing callback
- [ ] Update `gemini_continuation.rs` (5 locations): Pass session_id when clearing
- [ ] Update `compaction_retry.rs` line 374: Pass session_id when clearing
- [ ] Update all existing tests in `tool_progress.rs`
- [ ] Add new per-session isolation test
- [ ] Add new cleanup-doesn't-affect-other-sessions test
- [ ] Verify `cargo build` passes
- [ ] Verify `cargo test` passes
- [ ] Verify `npm run build` passes
