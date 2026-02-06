/**
 * Work Unit Status Hook
 *
 * SOLID: Open/Closed - Extends status update without modifying original
 * DRY: Reuses workUnitContextService
 *
 * TUI-059: Work Unit Context in Environment Information and Status Change Notifications
 *
 * Called after status update to handle work unit context changes and emit
 * system reminders when the LLM is working on a different work unit.
 */

import {
  detectWorkUnitChange,
  formatWorkUnitChangeReminder,
  setWorkUnitContext,
} from '../../tui/services/workUnitContextService';
import { wrapInSystemReminder } from '../../utils/system-reminder';
import { logger } from '../../utils/logger';

// Cache for dynamic import
let napiModule: { sessionGetActive: () => string | null } | null = null;
let napiLoaded = false;

async function getSessionActive(): Promise<string | null> {
  if (!napiLoaded) {
    napiLoaded = true;
    try {
      napiModule = await import('@sengac/codelet-napi');
    } catch {
      // NAPI not available (CLI mode without TUI)
      napiModule = null;
    }
  }
  return napiModule?.sessionGetActive() ?? null;
}

export interface WorkUnitStatusHookResult {
  /** System reminder to emit, or null if no change */
  systemReminder: string | null;
}

/**
 * Called after status update to handle work unit context changes.
 *
 * When the LLM runs update-work-unit-status on a different work unit than
 * the currently attached one, this emits a system reminder to notify about
 * the context change.
 *
 * @param workUnitId - The work unit ID being updated
 * @param newStatus - The new status
 * @param workUnitTitle - The work unit title
 * @returns System reminder if work unit changed, null otherwise
 */
export async function onWorkUnitStatusUpdated(
  workUnitId: string,
  newStatus: string,
  workUnitTitle: string
): Promise<WorkUnitStatusHookResult> {
  try {
    const activeSessionId = await getSessionActive();

    if (!activeSessionId) {
      // No active session (CLI mode), nothing to do
      logger.debug(
        '[WorkUnitStatusHook] No active session, skipping context check'
      );
      return { systemReminder: null };
    }

    const change = detectWorkUnitChange(activeSessionId, workUnitId, {
      title: workUnitTitle,
      status: newStatus,
    });

    if (!change) {
      // Same work unit, just update status in context
      setWorkUnitContext(activeSessionId, {
        id: workUnitId,
        title: workUnitTitle,
        status: newStatus,
      });
      logger.debug(
        '[WorkUnitStatusHook] Same work unit, updated context status'
      );
      return { systemReminder: null };
    }

    // Work unit changed - update context and generate reminder
    setWorkUnitContext(activeSessionId, change.current);

    const reminderText = formatWorkUnitChangeReminder(change);
    logger.debug('[WorkUnitStatusHook] Work unit changed, emitting reminder');

    return {
      systemReminder: wrapInSystemReminder(reminderText),
    };
  } catch (error) {
    // Log error but don't fail the command
    logger.debug('[WorkUnitStatusHook] Error checking context:', error);
    return { systemReminder: null };
  }
}
