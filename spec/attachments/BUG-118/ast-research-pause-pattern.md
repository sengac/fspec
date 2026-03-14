# AST Research: Pause Pattern for HITL Replication

## Research Date: 2026-03-14
## Scope: src/tui/hooks/rustStateSource.ts, useRustSessionState.ts, InputTransition.tsx, AgentView.tsx, types/pause.ts

## Data Flow

```
Rust sets session status to "paused"
  → SessionStateChange chunk emitted through NAPI stream
  → GlobalSessionStreamManager → persistentChunkHandler (AgentView)
  → handlePersistentSessionStateChange() calls refreshRustState(sessionId)
  → refreshSessionState() increments version + notifies subscribers
  → useSyncExternalStore calls getSnapshot → fetchFreshSnapshot()
  → source.getStatus() returns "paused" → isPaused = true
  → source.getPauseState() → parsePauseInfo() → PauseInfo { kind, toolName, message, details }
  → React re-renders AgentView with new rustSnapshot
  → InputTransition renders pause UI (replaces ThinkingIndicator/MultiLineInput)
  → useInputCompat (HIGH priority, isActive: displayIsPaused)
  → Captures keyboard → calls sessionPauseResume/Confirm/Triple NAPI functions
  → Rust resumes → status changes to "running" → cycle repeats
```

## Key Layers

1. **Types** (`src/tui/types/pause.ts`): PauseKind, PauseInfo, parsePauseInfo(), pauseInfoEqual()
2. **NAPI abstraction** (`rustStateSource.ts`): RustStateSource interface with getPauseState() wrapping NAPI calls with try/catch
3. **Snapshot** (`useRustSessionState.ts`): RustSessionSnapshot with isPaused + pauseInfo fields, lazy-fetch when status==="paused"
4. **Notification** (`persistentSessionStateHandler.ts`): Already handles ALL state changes via refreshRustState()
5. **UI** (`InputTransition.tsx`): Priority chain render — pause UI replaces loading/input, renders per kind
6. **Keyboard** (`AgentView.tsx`): useInputCompat HIGH priority handler, isActive when paused, per-kind keyboard handling

## HITL Replication Plan

For HITL, the session status is "paused" (same as regular pause), but with hitl_request state filled:

1. **Types**: Create `src/tui/types/hitlRequest.ts` — HitlQuestion, HitlRequestInfo, parseHitlRequestInfo(), hitlRequestInfoEqual()
2. **NAPI abstraction**: Add getHitlRequest(sessionId) to RustStateSource interface, import sessionGetHitlRequest NAPI function
3. **Snapshot**: Add hitlRequest: HitlRequestInfo | null to RustSessionSnapshot, fetch when isPaused
4. **UI**: New priority branch in InputTransition for isPaused && hitlRequest
5. **Keyboard**: New useInputCompat handler for HITL up/down/Enter/Esc navigation

## Existing NAPI Functions (Already implemented by BUG-117)

- `sessionGetHitlRequest(sessionId)` — returns questions array when paused for HITL
- `sessionSendHitlResponse(sessionId, response)` — sends answers/cancellation back

## InputTransition Render Priority Chain

```
1. isPaused && pauseInfo → pause UI (highest priority)
2. isPaused && hitlRequest → HITL question UI (NEW - same level as pause)
3. actionPrompt → action prompt UI
4. animationPhase === 'loading' → ThinkingIndicator
5. animation phases → transition animations
6. normal → MultiLineInput
```
