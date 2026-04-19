/**
 * Persistent Session State Change Handler
 *
 * Extracted from AgentView's persistentChunkHandler for testability.
 * Handles SessionStateChange chunks that arrive when no streaming handler is active.
 *
 * BUG-101: Before extraction, this logic lived inline in persistentChunkHandler
 * and returned early WITHOUT calling refreshRustState for non-Cleared/non-Compacting
 * states. This caused isLoading to stay true forever when SessionStateChange(Idle)
 * arrived after apply_pending_dag.
 *
 * CMPCT-034: Removed the Compacting branch entirely. Rust is the source of truth
 * for compaction state — SessionStatus::Compacting is set per-session in Rust, and
 * useRustSessionState already reads isCompacting/compactionProgress from Rust via
 * sessionGetStatus() and sessionGetCompactionProgress(). The refreshRustState() call
 * at the bottom propagates the Compacting status to React automatically. No local
 * React state duplication needed.
 *
 * SOLID: Single Responsibility — handles only SessionStateChange routing
 * SOLID: Dependency Inversion — all side effects are injected via deps
 */

/**
 * Dependencies injected into the handler for testability.
 * Each function represents a side effect that the handler may trigger.
 *
 * CMPCT-034: Removed startCompaction and getCompactionProgress — Rust is the
 * source of truth for compaction state. refreshRustState() is the only path
 * needed to propagate Compacting status to the UI.
 */
export interface SessionStateChangeDeps {
  /** Reset conversation state (called on Cleared) */
  resetConversation: () => void;
  /** Refresh React state from Rust (called for ALL state changes — BUG-101 fix) */
  refreshRustState: (sessionId: string | null) => void;
  /** Get current session ID */
  getCurrentSessionId: () => string | null;
}

/**
 * Handle a SessionStateChange event in the persistent chunk handler.
 *
 * Key behavior:
 * - Cleared: resets conversation state
 * - ALL states (including Compacting): calls refreshRustState so React picks up
 *   isLoading/isPaused/isCompacting transitions from Rust source of truth
 *
 * CMPCT-034: The Compacting branch was removed. Rust already sets
 * SessionStatus::Compacting on the session before emitting the chunk, and
 * useRustSessionState reads isCompacting from Rust via sessionGetStatus().
 * The refreshRustState() call below triggers a re-read, which picks up the
 * Compacting status automatically — no local React state needed.
 *
 * @param sessionId - The routed sessionId the chunk arrived for (NOT the currently-viewed session)
 * @param state - The session state string (e.g., 'Cleared', 'Compacting', 'Running', 'Idle')
 * @param deps - Injected dependencies for side effects
 */
export function handlePersistentSessionStateChange(
  sessionId: string,
  state: string,
  deps: SessionStateChangeDeps
): void {
  if (state === 'Cleared') {
    deps.resetConversation();
  }
  // CMPCT-034: No Compacting branch — Rust is the source of truth.
  // Rust sets SessionStatus::Compacting before emitting the chunk.
  // refreshRustState() below triggers useRustSessionState to re-read from Rust,
  // which picks up isCompacting = true + compactionProgress automatically.

  // BUG-101: Always refresh Rust state for ALL SessionStateChange events.
  // This is critical when SessionStateChange(Idle) arrives after apply_pending_dag
  // finishes — the streaming handler has been cleaned up by then, so this persistent
  // handler is the only path to update React state.
  //
  // We still pass the currently-viewed sessionId here because refreshRustState
  // updates UI state (isLoading, isPaused, isCompacting) for the session the user
  // is looking at — not the one that emitted the chunk.
  deps.refreshRustState(deps.getCurrentSessionId());
}
