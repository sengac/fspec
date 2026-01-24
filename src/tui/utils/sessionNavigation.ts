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
  sessionManagerList,
} from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';

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
  // Debug: log all sessions in Rust
  const allSessions = sessionManagerList();
  logger.debug(
    `[VIEWNV-001] navigateRight: allSessions=${JSON.stringify(allSessions.map(s => s.id))}`
  );

  const next = sessionGetNext();
  logger.debug(`[VIEWNV-001] navigateRight: sessionGetNext() returned ${next}`);

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
  logger.debug(`[VIEWNV-001] navigateLeft: sessionGetPrev() returned ${prev}`);

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
  logger.warn('[VIEWNV-001] clearActiveSession called');
  logger.warn('[VIEWNV-001] Stack trace:', new Error().stack);
  sessionClearActive();
}

/**
 * Get first session (for BoardView → first session navigation)
 */
export function getFirstSession(): string | null {
  const first = sessionGetFirst();
  logger.debug(
    `[VIEWNV-001] getFirstSession: sessionGetFirst() returned ${first}`
  );
  return first;
}
