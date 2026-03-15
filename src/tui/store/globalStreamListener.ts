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

// BUG-119: Debounce timer for loadData() calls from watcher events.
// Coalesces multiple rapid WorkUnitsUpdate events into a single loadData() call.
let loadDataDebounceTimer: ReturnType<typeof setTimeout> | null = null;
const LOAD_DATA_DEBOUNCE_MS = 150;

let napiModule: {
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

  if (chunk.type === 'WorkUnitsUpdate') {
    // BUG-119: Debounce loadData() calls on the JavaScript side.
    // The Rust watcher already debounces at 100ms, but multiple events can still
    // arrive in quick succession. This JS debounce coalesces them into a single
    // loadData() call, preventing lock contention and error-state oscillation.
    if (loadDataDebounceTimer) {
      clearTimeout(loadDataDebounceTimer);
    }
    loadDataDebounceTimer = setTimeout(() => {
      loadDataDebounceTimer = null;
      // TUI-079: Use loadData() for full re-read instead of lossy updateWorkUnitsFromWatcher().
      // The watcher event serves purely as a "file changed" signal — chunk.workUnits is not used.
      void useFspecStore
        .getState()
        .loadData()
        .then(() => {
          // Sync session context from the freshly-loaded store data (not from chunk)
          const currentWorkUnitId =
            useSessionStore.getState().currentWorkUnitId;
          if (currentWorkUnitId) {
            const storeWorkUnits = useFspecStore.getState().workUnits;
            const updatedWorkUnit = storeWorkUnits.find(
              wu => wu.id === currentWorkUnitId
            );
            if (updatedWorkUnit) {
              // Work unit still exists — sync status if changed
              const currentStatus =
                useSessionStore.getState().currentWorkUnitStatus;
              if (currentStatus !== updatedWorkUnit.status) {
                useSessionStore
                  .getState()
                  .setCurrentWorkUnit(
                    currentWorkUnitId,
                    updatedWorkUnit.status
                  );
                void updateRustContext(
                  currentWorkUnitId,
                  updatedWorkUnit.title,
                  updatedWorkUnit.status
                );
              }
            } else {
              // TUI-079 Gap 8: Work unit was deleted — clear session context
              useSessionStore.getState().setCurrentWorkUnit(null, null);
              // Also clear Rust-side context to prevent stale reattachment
              // via syncWorkUnitContextToStore()
              void clearRustContext();
            }
          }
        });
    }, LOAD_DATA_DEBOUNCE_MS);
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
        // BUG-119: Cancel pending debounced loadData() on cleanup
        if (loadDataDebounceTimer) {
          clearTimeout(loadDataDebounceTimer);
          loadDataDebounceTimer = null;
        }
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

async function clearRustContext(): Promise<void> {
  try {
    const napi = await getNapiModule();
    const activeSessionId = napi.sessionGetActive();

    if (activeSessionId) {
      const { sessionSetWorkUnitContext } = await import(
        '@sengac/codelet-napi'
      );
      sessionSetWorkUnitContext(activeSessionId, null, null, null);
    }
  } catch (e) {
    logger.debug(`[GlobalStreamListener] Error clearing Rust context: ${e}`);
  }
}
