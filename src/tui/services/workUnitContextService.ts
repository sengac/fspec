/**
 * Work Unit Context Service
 *
 * Handles work unit context operations for sessions.
 */

import type {
  WorkUnitContext,
  WorkUnitContextChange,
} from '../types/workUnitContext';
import { logger } from '../../utils/logger';
import * as napiModule from '@sengac/codelet-napi';

function safeSessionSetWorkUnitContext(
  sessionId: string,
  id: string | null,
  title: string | null,
  status: string | null
): void {
  try {
    napiModule.sessionSetWorkUnitContext(sessionId, id, title, status);
  } catch {
    logger.debug('[WorkUnitContext] sessionSetWorkUnitContext not available');
  }
}

function safeSessionGetWorkUnitContext(
  sessionId: string
): { id: string; title: string; status: string } | null {
  try {
    return napiModule.sessionGetWorkUnitContext(sessionId);
  } catch {
    logger.debug('[WorkUnitContext] sessionGetWorkUnitContext not available');
    return null;
  }
}

function safeSessionGetActive(): string | null {
  try {
    return napiModule.sessionGetActive();
  } catch {
    logger.debug('[WorkUnitContext] sessionGetActive not available');
    return null;
  }
}

export function detectWorkUnitChangeFromContext(
  currentContext: WorkUnitContext | null,
  newWorkUnitId: string,
  newWorkUnit: { title: string; status: string },
  sessionId: string
): WorkUnitContextChange | null {
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

export function setWorkUnitContext(
  sessionId: string,
  context: WorkUnitContext | null
): void {
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

export function getActiveWorkUnitContext(): WorkUnitContext | null {
  const activeSessionId = safeSessionGetActive();

  if (!activeSessionId) {
    return null;
  }

  return getWorkUnitContext(activeSessionId);
}

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
