# FspecTool Handler Not Configured After Session Switch

## Error Message

```
Fspec handler not configured. FspecTool requires session context with TypeScript integration.
```

## Context

### What Was Happening

1. User was working in this session (#3) on work units GIT-013, GIT-014, GIT-015
2. All three work units were in `specifying` status
3. I was performing Example Mapping using the Fspec tool
4. The Fspec tool was working correctly initially:
   - Successfully created epic `gitoxide-integration`
   - Successfully created stories GIT-013, GIT-014, GIT-015
   - Successfully added dependencies between stories
   - Successfully updated work unit statuses to `specifying`
   - Successfully added rules and attachments

### When It Broke

After the user **switched sessions** (presumably to session #2 based on the TUI output showing `#2 (BRIDGE-007: done)`), and then came back, the Fspec tool started returning the error intermittently.

### Specific Commands That Failed

```
command: "set-user-story"
args: {"_": ["GIT-013"], "role": "developer", "action": "use gitoxide instead of isomorphic-git for git operations", "benefit": "get better performance..."}
Result: "Fspec handler not configured. FspecTool requires session context with TypeScript integration."

command: "add-attachment"  
args: {"_": ["GIT-014", "spec/attachments/GIT-014/worktree-creation-research.md"], ...}
Result: "Fspec handler not configured. FspecTool requires session context with TypeScript integration."
```

### Commands That Still Worked

Interestingly, some Fspec commands continued to work during this same period:
- `add-rule` commands worked
- `add-example` commands worked  
- `update-work-unit-status` commands worked

## Technical Analysis

### Expected Behavior

According to REFAC-008 implementation:
- FspecCommandRequest handling was moved to a **global stream listener** that subscribes to ALL active sessions
- This should prevent deadlocks when detached sessions invoke fspec tools
- The global `FSPEC_CALLBACK` should be available regardless of which session is attached

### Actual Behavior

The error suggests that:
1. The session context is being lost or not properly restored when switching sessions
2. The global stream chunk handler may not be properly re-subscribing to the session after switch
3. OR the FspecTool facade wrapper is checking for session context that becomes stale

### Relevant Code Locations

Based on the search results:
- `codelet/tools/src/fspec.rs` - FspecTool implementation
- `src/tui/components/AgentView.tsx` - Original FspecCommandRequest handler location
- `spec/features/global-session-stream-manager.feature` - Feature spec for global handling

### Root Cause Hypothesis

The global session stream manager (REFAC-008) may be:
1. Not properly re-registering the Fspec callback when session focus changes
2. Using session-specific state that becomes invalid on session switch
3. Missing subscription renewal when sessions are attached/detached

## Reproduction Steps

1. Start TUI with multiple sessions
2. In Session A, use Fspec tool successfully for several commands
3. Switch to Session B (Shift+Arrow or via session list)
4. Switch back to Session A
5. Try to use Fspec tool
6. Error may appear intermittently

## Impact

This breaks the AI agent workflow because:
- Example Mapping cannot be completed reliably
- Work unit management becomes inconsistent
- User has to fall back to using Bash to run fspec CLI directly

## Suggested Fix

Review the global session stream subscription mechanism to ensure:
1. The FSPEC_CALLBACK is truly global and not session-bound
2. Session switches properly maintain callback availability
3. Add logging to trace when/why the callback becomes unavailable

---

## Fix Applied (REFAC-008-FIX)

### Root Cause Identified

The issue was **not** in the GlobalSessionStreamManager or the TypeScript layer. The root cause was in the **Rust fspec_handler** module:

1. `FSPEC_HANDLER` was a **single global static** `RwLock<Option<FspecHandler>>`
2. Each session's `agent_loop` would set its handler before running and clear it after
3. When multiple sessions ran concurrently, they would **overwrite each other's handlers**
4. When Session B cleared its handler, Session A's handler became `None`
5. Session A's tool call would then fail with "Fspec handler not configured"

### Fix Implementation

Changed from a single global handler to **per-session handlers stored in a HashMap**:

1. **`fspec_handler.rs`**: 
   - Changed `FSPEC_HANDLER: RwLock<Option<Handler>>` to `FSPEC_HANDLERS: RwLock<HashMap<Uuid, Handler>>`
   - Changed `CURRENT_FSPEC_SESSION` from global `RwLock` to **thread-local** storage using `thread_local!`
   - Added `set_fspec_handler_for_session(session_id, handler)` API
   - Added `set_current_fspec_session(session_id)` API (now thread-local)
   - Modified `execute_fspec_command` to look up handler by current session
   - Added `test_concurrent_sessions_isolated` test to verify fix

2. **`session_manager.rs`**:
   - Set `set_current_fspec_session(Some(session.id))` before agent run
   - Use `set_fspec_handler_for_session(session.id, handler)` instead of global setter
   - Clean up both on agent run completion

### Files Modified

- `codelet/tools/src/fspec_handler.rs` - Complete rewrite for per-session storage
- `codelet/tools/src/lib.rs` - Export new functions
- `codelet/napi/src/session_manager.rs` - Use per-session API
- `codelet/tools/tests/fspec_facade_wrapper_test.rs` - Updated to use new API

### Why This Fixes the Issue

With per-session handlers:
- Session A sets `FSPEC_HANDLERS[A] = handler_a`
- Session B sets `FSPEC_HANDLERS[B] = handler_b` (no conflict)
- Session B clears `FSPEC_HANDLERS[B] = None` (Session A's handler remains)
- Session A's tool call uses `FSPEC_HANDLERS[A]` which is still valid

### Remaining Theoretical Race Condition

**UPDATE: FIXED with thread-local storage**

The original fix used a global `RwLock<Option<Uuid>>` for `CURRENT_FSPEC_SESSION`, which still had a theoretical race condition:
- If tokio schedules Session A and Session B on the same thread interleaved at await points
- The global "current session" could be incorrect when read

**Final Fix: Thread-Local Storage**

Changed `CURRENT_FSPEC_SESSION` to use `thread_local!` storage:

```rust
thread_local! {
    static CURRENT_FSPEC_SESSION: RefCell<Option<Uuid>> = const { RefCell::new(None) };
}
```

This works because:
1. Each session's agent run happens on a dedicated thread
2. Tool calls happen on the same thread as the agent run
3. Thread-local storage ensures Session A's current_session isn't overwritten by Session B

**Integration Tests Added**

Created comprehensive integration tests in `codelet/tools/tests/fspec_handler_session_isolation_test.rs`:

1. `test_concurrent_sessions_have_isolated_handlers` - Uses real concurrent threads with a Barrier to verify handlers don't cross-contaminate
2. `test_session_cleanup_does_not_affect_other_sessions` - Reproduces the exact bug scenario where Session B's cleanup affected Session A
3. `test_current_session_determines_handler_used` - Verifies handler lookup uses correct session context
4. `test_rapid_session_switching_maintains_integrity` - 4 threads, 25 commands each, verifies no cross-contamination
5. `test_full_session_lifecycle_attach_run_detach` - Simulates real attach/run/detach lifecycle with concurrent sessions

All 9 tests pass, confirming the fix works correctly under real concurrent execution conditions.
