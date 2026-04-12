# BUG-129: Fix BASH_ABORT_FLAG Per-Session Isolation

## Root Cause

`codelet/tools/src/bash.rs:41` defines a **process-global static**:

```rust
static BASH_ABORT_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
```

When a user presses ESC in any session, `SessionEntry::interrupt()` at `session_manager.rs:1250` calls `request_bash_abort()` which sets this **process-wide** flag. ALL bash commands running in ALL sessions see the abort and terminate, not just the session where ESC was pressed.

Additionally, `SessionEntry::reset_interrupt()` at `session_manager.rs:1260` calls `clear_bash_abort()` which clears the flag for ALL sessions — potentially un-aborting a bash command in a different session that the user actually wanted to stop.

## Contrast with Correct Per-Session Interrupt

The `is_interrupted` flag IS correctly per-session:

```rust
// In BackgroundSession — CORRECT, per-session:
pub fn interrupt(&self) {
    self.is_interrupted.store(true, Ordering::Release);  // ← per-session Arc<AtomicBool>
    request_bash_abort();                                  // ← GLOBAL (BUG!)
    self.interrupt_notify.notify_one();
}
```

The fix should make bash abort follow the same per-session pattern as `is_interrupted`.

## Files to Modify

### 1. `codelet/tools/src/bash.rs` (PRIMARY)

**Current broken code (lines 40-56):**
```rust
/// Shared abort signal for bash tool cancellation
static BASH_ABORT_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set the abort flag to request cancellation of running bash commands
pub fn request_bash_abort() {
    BASH_ABORT_FLAG.store(true, std::sync::atomic::Ordering::Release);
}

/// Clear the abort flag (call before starting a new command)
pub fn clear_bash_abort() {
    BASH_ABORT_FLAG.store(false, std::sync::atomic::Ordering::Release);
}

/// Check if abort has been requested
fn is_bash_abort_requested() -> bool {
    BASH_ABORT_FLAG.load(std::sync::atomic::Ordering::Acquire)
}
```

**Fix — convert to per-session HashMap of Arc<AtomicBool>:**
```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;
use uuid::Uuid;

/// Per-session abort flags for bash tool cancellation
static BASH_ABORT_FLAGS: Lazy<RwLock<HashMap<Uuid, Arc<AtomicBool>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register an abort flag for a session (called when session starts)
pub fn register_bash_abort_flag(session_id: Uuid) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = BASH_ABORT_FLAGS.write() {
        guard.insert(session_id, flag.clone());
    }
    flag
}

/// Remove the abort flag for a session (called on session cleanup)
pub fn unregister_bash_abort_flag(session_id: Uuid) {
    if let Ok(mut guard) = BASH_ABORT_FLAGS.write() {
        guard.remove(&session_id);
    }
}

/// Set the abort flag for a SPECIFIC session
pub fn request_bash_abort(session_id: Uuid) {
    if let Ok(guard) = BASH_ABORT_FLAGS.read() {
        if let Some(flag) = guard.get(&session_id) {
            flag.store(true, Ordering::Release);
        }
    }
}

/// Clear the abort flag for a SPECIFIC session
pub fn clear_bash_abort(session_id: Uuid) {
    if let Ok(guard) = BASH_ABORT_FLAGS.read() {
        if let Some(flag) = guard.get(&session_id) {
            flag.store(false, Ordering::Release);
        }
    }
}

/// Check if abort has been requested for a SPECIFIC session
fn is_bash_abort_requested(session_id: Uuid) -> bool {
    BASH_ABORT_FLAGS.read()
        .ok()
        .and_then(|guard| guard.get(&session_id).map(|f| f.load(Ordering::Acquire)))
        .unwrap_or(false)
}
```

**Alternative simpler approach**: Since `BashTool` already has a `session_id` field, an even simpler fix is to make the abort check use the session's `is_interrupted` flag directly instead of a separate abort flag. But this requires the `Arc<AtomicBool>` to be accessible from the bash tool, which may need threading changes.

**Update ALL callers of `is_bash_abort_requested()` in bash.rs:**

Each call site needs `session_id`:

| Line | Location | Current | Fix |
|------|----------|---------|-----|
| ~375 | `spawn_stdout_reader()` | `is_bash_abort_requested()` | `is_bash_abort_requested(session_id)` — add `session_id: Uuid` param |
| ~401 | `spawn_stderr_reader()` | `is_bash_abort_requested()` | `is_bash_abort_requested(session_id)` — add `session_id: Uuid` param |
| ~425 | `wait_for_tasks_with_abort()` (Unix) | `is_bash_abort_requested()` | `is_bash_abort_requested(session_id)` — add `session_id: Uuid` param |
| ~449 | `wait_for_tasks_with_abort()` (non-Unix) | `is_bash_abort_requested()` | `is_bash_abort_requested(session_id)` — add `session_id: Uuid` param |
| ~675 | `Tool::call()` inline stdout reader | `is_bash_abort_requested()` | `is_bash_abort_requested(session_id)` — use self.session_id |

**Update `clear_bash_abort()` calls in bash.rs:**

| Line | Location | Current | Fix |
|------|----------|---------|-----|
| ~555 | `call_with_streaming()` | `clear_bash_abort()` | `clear_bash_abort(self.session_id)` |
| ~667 | `Tool::call()` | `clear_bash_abort()` | `clear_bash_abort(self.session_id)` |

**Update helper function signatures:**

```rust
// spawn_stdout_reader — add session_id parameter:
fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    buffer: Arc<Mutex<String>>,
    stream_to_ui: bool,
    session_id: Uuid,      // NEW
) -> tokio::task::JoinHandle<()>

// spawn_stderr_reader — add session_id parameter:
fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    buffer: Arc<Mutex<String>>,
    stream_to_ui: bool,
    session_id: Uuid,      // NEW
) -> tokio::task::JoinHandle<()>

// wait_for_tasks_with_abort — add session_id parameter:
async fn wait_for_tasks_with_abort(
    child: &mut tokio::process::Child,
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
    timeout_secs: u64,
    session_id: Uuid,      // NEW
) -> ...
```

### 2. `codelet/tools/src/lib.rs` (RE-EXPORTS)

**Current (line 90):**
```rust
pub use bash::{clear_bash_abort, request_bash_abort};
```

**Update to include new functions:**
```rust
pub use bash::{clear_bash_abort, request_bash_abort, register_bash_abort_flag, unregister_bash_abort_flag};
```

### 3. `codelet/napi/src/session_manager.rs` (CALLERS)

**Import (line 37):**
```rust
// OLD:
use codelet_tools::{clear_bash_abort, request_bash_abort};

// NEW:
use codelet_tools::{clear_bash_abort, request_bash_abort, register_bash_abort_flag, unregister_bash_abort_flag};
```

**interrupt() method (line ~1247-1252):**
```rust
// OLD:
pub fn interrupt(&self) {
    self.is_interrupted.store(true, Ordering::Release);
    request_bash_abort();         // GLOBAL!
    self.interrupt_notify.notify_one();
}

// NEW:
pub fn interrupt(&self) {
    self.is_interrupted.store(true, Ordering::Release);
    request_bash_abort(self.id);  // PER-SESSION!
    self.interrupt_notify.notify_one();
}
```

**reset_interrupt() method (line ~1254-1261):**
```rust
// OLD:
pub fn reset_interrupt(&self) {
    self.is_interrupted.store(false, Ordering::Release);
    clear_bash_abort();           // GLOBAL!
}

// NEW:
pub fn reset_interrupt(&self) {
    self.is_interrupted.store(false, Ordering::Release);
    clear_bash_abort(self.id);    // PER-SESSION!
}
```

**Session creation**: In `BackgroundSession::new()` or wherever sessions are initialized, call `register_bash_abort_flag(session_id)`.

**Session cleanup**: In the cleanup path (near lines 5435-5447), add `unregister_bash_abort_flag(session.id)`.

### 4. `codelet/cli/src/interactive/stream_loop.rs` (CLI MODE)

In CLI mode, there's typically only one session, but still update for consistency. Search for `clear_bash_abort()` and `request_bash_abort()` in stream_loop.rs and pass the session ID.

**Note**: CLI mode may not have a UUID-based session. If `Session` in CLI mode doesn't have a UUID, either:
- Add a UUID to the CLI Session struct, OR
- Use a fixed sentinel UUID for CLI mode (e.g., `Uuid::nil()`)

## BashTool Struct Confirmation

The `BashTool` struct (confirmed by research agent) has:
```rust
pub struct BashTool {
    session_id: Uuid,
    // ... other fields
}
```

So `self.session_id` is available in `Tool::call()` and `call_with_streaming()`.

## Tests to Update

### Existing tests in `codelet/tools/tests/bash_streaming_test.rs`

Lines that call `request_bash_abort()` and `clear_bash_abort()`:
- Line 251, 278, 316, 319: `request_bash_abort()` → add session_id
- Line 259, 304, 322: `clear_bash_abort()` → add session_id

### New tests to add

1. **Session isolation**: Register abort flags for sessions A and B. Request abort for A. Assert only A's flag is set. Assert `is_bash_abort_requested(session_b_id)` returns false.
2. **Clear isolation**: Clear A's abort. Assert B is unaffected.
3. **Unregister cleanup**: Unregister session A. Assert `request_bash_abort(session_a_id)` is a no-op (doesn't panic).
4. **Unknown session**: Call `is_bash_abort_requested(unknown_id)`. Assert returns false.

## Checklist

- [ ] Modify `bash.rs`: Replace global `AtomicBool` with `Lazy<RwLock<HashMap<Uuid, Arc<AtomicBool>>>>`
- [ ] Add `register_bash_abort_flag(session_id)` and `unregister_bash_abort_flag(session_id)` functions
- [ ] Update `request_bash_abort` to accept `session_id: Uuid`
- [ ] Update `clear_bash_abort` to accept `session_id: Uuid`
- [ ] Update `is_bash_abort_requested` to accept `session_id: Uuid`
- [ ] Update `spawn_stdout_reader`, `spawn_stderr_reader`, `wait_for_tasks_with_abort` to accept and pass `session_id`
- [ ] Update `BashTool::call()` and `call_with_streaming()` to pass `self.session_id`
- [ ] Update `lib.rs` re-exports
- [ ] Update `session_manager.rs` interrupt/reset_interrupt to pass `self.id`
- [ ] Add register/unregister calls in session lifecycle
- [ ] Update CLI mode if applicable (stream_loop.rs)
- [ ] Update all existing tests
- [ ] Add new per-session isolation tests
- [ ] Verify `cargo build` passes
- [ ] Verify `cargo test` passes
- [ ] Verify `npm run build` passes
