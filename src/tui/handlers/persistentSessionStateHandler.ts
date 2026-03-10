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
 * SOLID: Single Responsibility — handles only SessionStateChange routing
 * SOLID: Dependency Inversion — all side effects are injected via deps
 */

import type { CompactionProgress } from '../hooks/rustStateSource';

/**
 * Dependencies injected into the handler for testability.
 * Each function represents a side effect that the handler may trigger.
 */
export interface SessionStateChangeDeps {
  /** Reset conversation state (called on Cleared) */
  resetConversation: () => void;
  /** Start compaction tracking (called on Compacting) */
  startCompaction: (
    trigger: string,
    sessionId: string,
    progress?: CompactionProgress
  ) => void;
  /** Get compaction progress from Rust (called on Compacting) */
  getCompactionProgress: (sessionId: string) => CompactionProgress | null;
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
 * - Compacting: starts compaction tracking
 * - ALL states: calls refreshRustState so React picks up isLoading/isPaused transitions
 * - NEVER calls endCompaction — only CompactionComplete should end the compaction indicator
 *
 * @param state - The session state string (e.g., 'Cleared', 'Compacting', 'Running', 'Idle')
 * @param deps - Injected dependencies for side effects
 */
export function handlePersistentSessionStateChange(
  state: string,
  deps: SessionStateChangeDeps
): void {
  if (state === 'Cleared') {
    deps.resetConversation();
  } else if (state === 'Compacting') {
    const sessionId = deps.getCurrentSessionId();
    if (sessionId) {
      const progress = deps.getCompactionProgress(sessionId);
      deps.startCompaction('hook-triggered', sessionId, progress ?? undefined);
    }
  }

  // BUG-101: Always refresh Rust state for ALL SessionStateChange events.
  // This is critical when SessionStateChange(Idle) arrives after apply_pending_dag
  // finishes — the streaming handler has been cleaned up by then, so this persistent
  // handler is the only path to update React state.
  //
  // Do NOT call endCompaction() here — only CompactionComplete should end the
  // compaction indicator.
  deps.refreshRustState(deps.getCurrentSessionId());
}
