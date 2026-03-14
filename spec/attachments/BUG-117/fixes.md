# BUG-117 Fixes — Wrong Pattern Used in Initial Implementation

## Problem

The initial implementation used the **FspecCommandRequest pattern** (emit StreamChunk → GlobalSessionStreamManager intercepts → callback renders UI). This is the wrong pattern for HITL because:

1. `setHitlHandler()` on `GlobalSessionStreamManager` is never called in production code — nobody registers a callback
2. No TUI component renders the questions — `handleHitlRequest()` has no UI, just fires a callback that doesn't exist
3. The StreamChunk intercept pattern is designed for fire-and-forget command execution (fspec CLI), not for interactive user input
4. With no handler registered, every HITL request auto-cancels silently

## Correct Pattern: Pause System

HITL `request_user_input` **is a pause**. The agent blocks, shows the user something inline, waits for their input, resumes. This is exactly what the pause system does.

### Pause System End-to-End Trace

1. **Rust handler closure** (`session_manager.rs:5346`):
   - `session.set_pause_state(Some(state))` — stores pause info in session
   - `session.set_status(SessionStatus::Paused)` — changes session status
   - `session.wait_for_pause_response()` — **BLOCKS** on mpsc channel
   - On response: status back to Running, clear pause state, return response

2. **Status change propagates to TypeScript** automatically via `SessionStateChange` chunk (already emitted for any status change)

3. **`persistentSessionStateHandler.ts`**: Calls `refreshRustState()` → triggers `useSyncExternalStore`

4. **`useRustSessionState.ts:148`**: `isPaused = status === 'paused'` + `pauseInfo = getPauseState(sessionId)` → snapshot updates

5. **`InputTransition.tsx:344`**: `if (isPaused && pauseInfo)` → renders inline UI (Y/N/Enter/←→ depending on pause kind)

6. **`AgentView.tsx:5005-5090`**: `useInputCompat` handler captures keys and calls NAPI: `sessionPauseResume()` / `sessionPauseConfirm()` / `sessionPauseTriple()`

7. **NAPI functions** (`session_manager.rs:6535-6575`): Call `session.send_pause_response()` → unblocks step 1

### Key Architectural Insight

The pause system does **NOT** use a StreamChunk for the request. It:
- Sets session state directly (stored on `BackgroundSession`)  
- TypeScript polls this state via NAPI getter on each React re-render cycle
- Keyboard handler directly calls NAPI response functions

## Fix Plan

### What to REMOVE (wrong pattern)

- `HitlRequest` StreamChunk variant and all associated NAPI types (`HitlRequestInfo`, `HitlQuestionInfo`, `HitlOptionInfo`)
- `StreamChunk::hitl_request()` constructor and `to_json_value()` arm
- `GlobalSessionStreamManager` intercept code: `handleHitlRequest()`, `setHitlHandler()`, `clearHitlHandler()`, `sendHitlResponse()`, `hitlHandler` field
- TypeScript test file `hitl-handler-wiring.test.ts` (will be rewritten)
- `NapiModule.sessionSendHitlResponse` type in `globalSessionStreamManager.ts`

### What to KEEP (correct plumbing)

- `hitl_response_tx/rx` channel pair on `BackgroundSession` — correct blocking mechanism
- `wait_for_hitl_response()` / `send_hitl_response()` — correct
- `session_send_hitl_response()` NAPI function — correct
- `HitlResponseInfo` / `HitlAnswerEntry` NAPI types — correct (response direction)
- `set_hitl_handler(session.id, Some(handler))` / `None` in agent_loop — correct lifecycle

### What to ADD (pause pattern)

1. **`BackgroundSession` state field**: `hitl_request: RwLock<Option<HitlRequestState>>` storing questions + session_id
2. **HITL handler closure changes**: Instead of emitting StreamChunk, set hitl_request state + `set_status(Paused)`, block, then clear state + set Running
3. **NAPI getter**: `session_get_hitl_request(session_id)` → returns `Option<NapiHitlRequestState>` with questions
4. **`useRustSessionState`**: When `isPaused`, also check for HITL request → add `hitlRequest` to `RustSessionSnapshot`
5. **`rustStateSource.ts`**: Add `getHitlRequest(sessionId)` method wrapping the NAPI getter
6. **`InputTransition.tsx`**: New pause kind branch — renders questions with selectable options and freeform input inline
7. **`AgentView.tsx`**: New `useInputCompat` handler alongside pause handler — ↑/↓ navigate questions, ←/→ navigate options, Enter selects/advances, Tab for freeform, Esc cancels
8. **`sessionSendHitlResponse`** already exists for the response path

### Rendering Approach (inline, like pause)

```
⏸ request_user_input: 2 questions
  
  [1/2] Approach — Which approach do you prefer?
    ● Option A (Recommended) — First choice
    ○ Option B — Second choice
  
  [Tab] notes | [Enter] next | [Esc] cancel
```

After all questions answered:
```
  [2/2] Priority — What is the priority?
    ○ High — Urgent
    ● Low — Not urgent
  
  [Enter] submit all | [←] back | [Esc] cancel
```

This is a multi-step inline wizard rendered in `InputTransition`, exactly like the triple-pause renders ←/→ options.
