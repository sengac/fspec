/**
 * Work Unit Status Hook
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
import { sendIPCMessage } from '../../utils/ipc';

let napiModule: { sessionGetActive: () => string | null } | null = null;
let napiLoaded = false;

async function getSessionActive(): Promise<string | null> {
  if (!napiLoaded) {
    napiLoaded = true;
    try {
      napiModule = await import('@sengac/codelet-napi');
    } catch {
      napiModule = null;
    }
  }
  return napiModule?.sessionGetActive() ?? null;
}

export interface WorkUnitStatusHookResult {
  systemReminder: string | null;
}

export async function onWorkUnitStatusUpdated(
  workUnitId: string,
  newStatus: string,
  workUnitTitle: string
): Promise<WorkUnitStatusHookResult> {
  try {
    const activeSessionId = await getSessionActive();

    if (!activeSessionId) {
      return { systemReminder: null };
    }

    const change = detectWorkUnitChange(activeSessionId, workUnitId, {
      title: workUnitTitle,
      status: newStatus,
    });

    if (!change) {
      setWorkUnitContext(activeSessionId, {
        id: workUnitId,
        title: workUnitTitle,
        status: newStatus,
      });
      return { systemReminder: null };
    }

    setWorkUnitContext(activeSessionId, change.current);

    try {
      await sendIPCMessage({
        type: 'work-unit-changed',
        payload: {
          workUnitId,
          sessionId: activeSessionId,
        },
      });
    } catch {
      // TUI might not be running
    }

    const reminderText = formatWorkUnitChangeReminder(change);

    return {
      systemReminder: wrapInSystemReminder(reminderText),
    };
  } catch {
    return { systemReminder: null };
  }
}
