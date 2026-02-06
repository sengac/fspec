/**
 * Work Unit Context Service
 *
 * SOLID: Single Responsibility - Only handles work unit context operations
 * DRY: All work unit context logic in one place
 * COMPOSABLE: Pure functions separated from NAPI calls
 *
 * TUI-059: Work Unit Context in Environment Information and Status Change Notifications
 *
 * Architecture:
 * - Pure functions (detectWorkUnitChangeFromContext, formatWorkUnitChangeReminder)
 *   can be tested without mocks
 * - NAPI wrapper functions (setWorkUnitContext, getWorkUnitContext)
 *   are thin wrappers that delegate to NAPI
 */

import type {
  WorkUnitContext,
  WorkUnitContextChange,
} from '../types/workUnitContext';
import { logger } from '../../utils/logger';

// Import NAPI functions
// TUI-059: These functions are implemented in @sengac/codelet-napi
// We import the module and access functions safely to handle test mocks
// that may not include these functions
import * as napiModule from '@sengac/codelet-napi';

// Safe function accessors that handle missing functions in mocks
// Vitest mocks throw when accessing undefined properties, so we use try/catch
function safeSessionSetWorkUnitContext(
  sessionId: string,
  id: string | null,
  title: string | null,
  status: string | null
): void {
  try {
    napiModule.sessionSetWorkUnitContext(sessionId, id, title, status);
  } catch {
    logger.debug(
      '[WorkUnitContext] sessionSetWorkUnitContext not available (likely in test)'
    );
  }
}

function safeSessionGetWorkUnitContext(
  sessionId: string
): { id: string; title: string; status: string } | null {
  try {
    return napiModule.sessionGetWorkUnitContext(sessionId);
  } catch {
    logger.debug(
      '[WorkUnitContext] sessionGetWorkUnitContext not available (likely in test)'
    );
    return null;
  }
}

function safeSessionGetActive(): string | null {
  try {
    return napiModule.sessionGetActive();
  } catch {
    logger.debug(
      '[WorkUnitContext] sessionGetActive not available (likely in test)'
    );
    return null;
  }
}

// ============================================================================
// PURE FUNCTIONS (Testable without mocks)
// ============================================================================

/**
 * Detect if work unit context has changed (pure function)
 *
 * This is the pure logic for change detection, separated from NAPI calls.
 * Can be tested directly with test data.
 *
 * @param currentContext - The current work unit context (or null if none)
 * @param newWorkUnitId - The new work unit ID being operated on
 * @param newWorkUnit - The new work unit details (title, status)
 * @param sessionId - The session ID (used in the change object)
 * @returns Change details if different, null if same work unit
 */
export function detectWorkUnitChangeFromContext(
  currentContext: WorkUnitContext | null,
  newWorkUnitId: string,
  newWorkUnit: { title: string; status: string },
  sessionId: string
): WorkUnitContextChange | null {
  // No change if same work unit
  if (currentContext?.id === newWorkUnitId) {
    return null;
  }

  return {
    previous: currentContext,
    current: {
      id: newWorkUnitId,
      title: newWorkUnit.title,
      status: newWorkUnit.status,
    },
    sessionId,
  };
}

/**
 * Format work unit change as system reminder (pure function)
 *
 * @param change - The work unit context change
 * @returns Formatted system reminder text
 */
export function formatWorkUnitChangeReminder(
  change: WorkUnitContextChange
): string {
  if (change.previous) {
    return (
      `Work unit context changed:\n` +
      `  Previous: ${change.previous.id} (${change.previous.title})\n` +
      `  Current: ${change.current.id} (${change.current.title})\n\n` +
      `You are now working on ${change.current.id}.`
    );
  }

  return (
    `Work unit context set:\n` +
    `  Current: ${change.current.id} (${change.current.title})\n\n` +
    `You are now working on ${change.current.id}.`
  );
}

// ============================================================================
// NAPI WRAPPER FUNCTIONS (Thin wrappers, tested via integration tests)
// ============================================================================

/**
 * Set work unit context for a session
 *
 * @param sessionId - The session ID
 * @param context - The work unit context to set (or null to clear)
 */
export function setWorkUnitContext(
  sessionId: string,
  context: WorkUnitContext | null
): void {
  logger.debug(
    `[WorkUnitContext] Setting context for session ${sessionId}:`,
    context
  );

  if (context) {
    safeSessionSetWorkUnitContext(
      sessionId,
      context.id,
      context.title,
      context.status
    );
  } else {
    safeSessionSetWorkUnitContext(sessionId, null, null, null);
  }
}

/**
 * Get work unit context for a session
 *
 * @param sessionId - The session ID
 * @returns The work unit context or null if not set
 */
export function getWorkUnitContext(sessionId: string): WorkUnitContext | null {
  const rustContext = safeSessionGetWorkUnitContext(sessionId);

  if (!rustContext) {
    return null;
  }

  return {
    id: rustContext.id,
    title: rustContext.title,
    status: rustContext.status,
  };
}

/**
 * Get the currently active session's work unit context
 *
 * @returns The active session's work unit context or null if no active session
 */
export function getActiveWorkUnitContext(): WorkUnitContext | null {
  const activeSessionId = safeSessionGetActive();

  if (!activeSessionId) {
    return null;
  }

  return getWorkUnitContext(activeSessionId);
}

/**
 * Detect if work unit context has changed (NAPI version)
 *
 * This is the convenience function that fetches current context from NAPI.
 * Delegates to detectWorkUnitChangeFromContext for the actual logic.
 *
 * @param sessionId - The session ID
 * @param newWorkUnitId - The new work unit ID being operated on
 * @param newWorkUnit - The new work unit details (title, status)
 * @returns Change details if different, null if same work unit
 */
export function detectWorkUnitChange(
  sessionId: string,
  newWorkUnitId: string,
  newWorkUnit: { title: string; status: string }
): WorkUnitContextChange | null {
  const currentContext = getWorkUnitContext(sessionId);
  return detectWorkUnitChangeFromContext(
    currentContext,
    newWorkUnitId,
    newWorkUnit,
    sessionId
  );
}
