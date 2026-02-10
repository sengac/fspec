/**
 * useWorkUnitsWatcher Hook
 *
 * Singleton hook for watching spec/work-units.json and triggering store updates.
 * Uses Rust-based file watcher (notify crate) for cross-platform reliability.
 * Used by BoardView only - AgentView subscribes to the store directly.
 */

import { useEffect, useRef } from 'react';
import { useFspecStore } from '../store/fspecStore';
import { logger } from '../../utils/logger';
import type { StreamChunk } from '@sengac/codelet-napi';

// Lazy import for NAPI functions to avoid issues when module is not ready
let napiModule: {
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
      startWorkUnitsWatcher: napi.startWorkUnitsWatcher,
      stopWorkUnitsWatcher: napi.stopWorkUnitsWatcher,
      isWorkUnitsWatcherActive: napi.isWorkUnitsWatcherActive,
    };
  }
  return napiModule;
}

interface WorkUnit {
  id: string;
  title: string;
  status: string;
  type: string;
  description?: string;
  epic?: string;
  estimate?: number;
  updated?: string;
}

interface UseWorkUnitsWatcherReturn {
  /** Get a work unit by ID from the store */
  getWorkUnitById: (workUnitId: string) => WorkUnit | undefined;
}

/**
 * Hook that watches spec/work-units.json for changes using Rust file watcher.
 * When the file changes, the Rust watcher emits a WorkUnitsUpdate chunk,
 * and we reload the full data from fspecStore.loadData().
 *
 * @returns Object with getWorkUnitById function for querying work units
 */
export function useWorkUnitsWatcher(): UseWorkUnitsWatcherReturn {
  const cwd = useFspecStore(state => state.cwd);
  const loadData = useFspecStore(state => state.loadData);
  const workUnits = useFspecStore(state => state.workUnits);

  // Track mount state and watcher state via refs
  const isMountedRef = useRef(true);
  const isWatcherStartedRef = useRef(false);

  // Start Rust file watcher for spec/work-units.json
  useEffect(() => {
    // Skip in test environment - tests should mock the NAPI module
    if (process.env.NODE_ENV === 'test' || process.env.VITEST) {
      return;
    }

    // Reset mounted flag on each effect run
    isMountedRef.current = true;

    if (!cwd || isWatcherStartedRef.current) {
      return;
    }

    const startWatcher = async () => {
      try {
        const napi = await getNapiModule();

        // Double-check we're still mounted after async import
        if (!isMountedRef.current) {
          return;
        }

        // Check if watcher is already active (from another component instance)
        if (napi.isWorkUnitsWatcherActive()) {
          logger.debug(
            '[useWorkUnitsWatcher] Watcher already active, skipping start'
          );
          return;
        }

        logger.debug(`[useWorkUnitsWatcher] Starting Rust watcher for: ${cwd}`);

        // Start the Rust file watcher with callback
        napi.startWorkUnitsWatcher(cwd, (chunk: StreamChunk) => {
          if (!chunk) {
            return;
          }

          if (!isMountedRef.current) {
            return;
          }

          if (chunk.type === 'WorkUnitsUpdate') {
            void loadData();
          }
        });

        isWatcherStartedRef.current = true;
      } catch (e) {
        logger.error(`[useWorkUnitsWatcher] Failed to start watcher: ${e}`);
      }
    };

    void startWatcher();

    // Cleanup function
    return () => {
      isMountedRef.current = false;

      if (isWatcherStartedRef.current) {
        logger.debug('[useWorkUnitsWatcher] Stopping Rust watcher');
        try {
          // Only stop if we actually started it
          getNapiModule()
            .then(napi => {
              if (napi.isWorkUnitsWatcherActive()) {
                napi.stopWorkUnitsWatcher();
              }
            })
            .catch(e => {
              logger.warn(`[useWorkUnitsWatcher] Error stopping watcher: ${e}`);
            });
        } catch (e) {
          logger.warn(`[useWorkUnitsWatcher] Error stopping watcher: ${e}`);
        }
        isWatcherStartedRef.current = false;
      }
    };
  }, [cwd, loadData]);

  // Helper to get a work unit by ID
  const getWorkUnitById = (workUnitId: string): WorkUnit | undefined => {
    return workUnits.find(wu => wu.id === workUnitId);
  };

  return {
    getWorkUnitById,
  };
}
