/**
 * VIEWNV-001: Session Navigation Utilities
 *
 * SIMPLE DESIGN: Rust is the source of truth.
 *
 * Rust's SessionManager:
 * - Tracks sessions in insertion order (IndexMap)
 * - Tracks the currently active (attached) session
 * - Provides get_next/get_prev/get_first functions
 *
 * TypeScript just calls Rust. That's it.
 */

import {
  sessionGetNext,
  sessionGetPrev,
  sessionGetFirst,
  sessionClearActive,
} from '@sengac/codelet-napi';

export type NavigationResult =
  | { type: 'session'; sessionId: string }
  | { type: 'board' }
  | { type: 'create-dialog' };

/**
 * Navigate right (Shift+Right)
 * - From BoardView (no active session): go to first session
 * - From a session: go to next session
 * - If no next session: show create dialog
 */
export function navigateRight(): NavigationResult {
  const next = sessionGetNext();

  if (next) {
    return { type: 'session', sessionId: next };
  } else {
    return { type: 'create-dialog' };
  }
}

/**
 * Navigate left (Shift+Left)
 * - From first session: go to board
 * - From a session: go to previous session
 * - From BoardView: no-op (stays on board)
 */
export function navigateLeft(): NavigationResult {
  const prev = sessionGetPrev();

  if (prev) {
    return { type: 'session', sessionId: prev };
  } else {
    return { type: 'board' };
  }
}

/**
 * Clear active session tracking (call when returning to BoardView)
 */
export function clearActiveSession(): void {
  sessionClearActive();
}

/**
 * Get first session (for BoardView → first session navigation)
 */
export function getFirstSession(): string | null {
  return sessionGetFirst();
}
