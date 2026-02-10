/**
 * Global Stream Listener
 *
 * Singleton that listens for StreamChunk events from Rust and updates
 * Zustand stores globally. This ensures work unit context updates work
 * regardless of which component is mounted.
 */

import { useSessionStore } from './sessionStore';
import { useFspecStore } from './fspecStore';
import { logger } from '../../utils/logger';
import type { StreamChunk } from '@sengac/codelet-napi';

let isInitialized = false;
let cleanupFn: (() => void) | null = null;

let napiModule: {
  sessionSubscribe: (
    sessionId: string,
    callback: (err: Error | null, chunk: StreamChunk) => void
  ) => void;
  sessionDetach: (sessionId: string) => void;
  sessionGetActive: () => string | null;
  sessionGetWorkUnitContext: (
    sessionId: string
  ) => { id: string; title: string; status: string } | null;
  startWorkUnitsWatcher: (
    projectRoot: string,
    callback: (chunk: StreamChunk) => void
  ) => void;
  stopWorkUnitsWatcher: () => void;
  isWorkUnitsWatcherActive: () => boolean;
} | null = null;

async function getNapiModule() {
  if (!napiModule) {
    const napi = await import('@sengac/codelet-napi');
    napiModule = {
      sessionSubscribe: napi.sessionSubscribe,
      sessionDetach: napi.sessionDetach,
      sessionGetActive: napi.sessionGetActive,
      sessionGetWorkUnitContext: napi.sessionGetWorkUnitContext,
      startWorkUnitsWatcher: napi.startWorkUnitsWatcher,
      stopWorkUnitsWatcher: napi.stopWorkUnitsWatcher,
      isWorkUnitsWatcherActive: napi.isWorkUnitsWatcherActive,
    };
  }
  return napiModule;
}

function handleStreamChunk(
  errOrChunk: Error | null | StreamChunk,
  maybeChunk?: StreamChunk
): void {
  let chunk: StreamChunk | null | undefined;
  if (maybeChunk !== undefined) {
    chunk = maybeChunk;
  } else if (
    errOrChunk &&
    typeof errOrChunk === 'object' &&
    'type' in errOrChunk
  ) {
    chunk = errOrChunk as StreamChunk;
  } else {
    return;
  }

  if (!chunk) {
    return;
  }

  if (chunk.type === 'WorkUnitsUpdate' && chunk.workUnits) {
    useFspecStore.getState().updateWorkUnitsFromWatcher(chunk.workUnits);

    const currentWorkUnitId = useSessionStore.getState().currentWorkUnitId;
    if (currentWorkUnitId) {
      const updatedWorkUnit = chunk.workUnits.find(
        wu => wu.id === currentWorkUnitId
      );
      if (updatedWorkUnit) {
        const currentStatus = useSessionStore.getState().currentWorkUnitStatus;
        if (currentStatus !== updatedWorkUnit.status) {
          useSessionStore
            .getState()
            .setCurrentWorkUnit(currentWorkUnitId, updatedWorkUnit.status);
          void updateRustContext(
            currentWorkUnitId,
            updatedWorkUnit.title,
            updatedWorkUnit.status
          );
        }
      }
    }
  }
}

export async function initGlobalStreamListener(cwd: string): Promise<void> {
  if (isInitialized) {
    return;
  }

  try {
    const napi = await getNapiModule();

    if (!napi.isWorkUnitsWatcherActive()) {
      napi.startWorkUnitsWatcher(cwd, handleStreamChunk);
    }

    isInitialized = true;

    cleanupFn = () => {
      try {
        if (napiModule?.isWorkUnitsWatcherActive()) {
          napiModule.stopWorkUnitsWatcher();
        }
      } catch (e) {
        logger.debug(`[GlobalStreamListener] Error during cleanup: ${e}`);
      }
      isInitialized = false;
      cleanupFn = null;
    };
  } catch (e) {
    logger.debug(`[GlobalStreamListener] Failed to initialize: ${e}`);
  }
}

export function stopGlobalStreamListener(): void {
  if (cleanupFn) {
    cleanupFn();
  }
}

export function isGlobalStreamListenerInitialized(): boolean {
  return isInitialized;
}

export async function syncWorkUnitContextToStore(): Promise<void> {
  try {
    const napi = await getNapiModule();
    const activeSessionId = napi.sessionGetActive();

    if (activeSessionId) {
      const ctx = napi.sessionGetWorkUnitContext(activeSessionId);
      if (ctx) {
        useSessionStore.getState().setCurrentWorkUnit(ctx.id, ctx.status);
      } else {
        useSessionStore.getState().setCurrentWorkUnit(null, null);
      }
    }
  } catch (e) {
    logger.debug(`[GlobalStreamListener] Error during manual sync: ${e}`);
  }
}

async function updateRustContext(
  workUnitId: string,
  title: string,
  status: string
): Promise<void> {
  try {
    const napi = await getNapiModule();
    const activeSessionId = napi.sessionGetActive();

    if (activeSessionId) {
      const { sessionSetWorkUnitContext } = await import(
        '@sengac/codelet-napi'
      );
      sessionSetWorkUnitContext(activeSessionId, workUnitId, title, status);
    }
  } catch (e) {
    logger.debug(`[GlobalStreamListener] Error updating Rust context: ${e}`);
  }
}
