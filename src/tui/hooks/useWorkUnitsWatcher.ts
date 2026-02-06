/**
 * TUI-060: useWorkUnitsWatcher Hook
 *
 * Reusable hook for watching spec/work-units.json and triggering store updates.
 * Follows DRY/SOLID/COMPOSABLE principles by extracting file watching logic
 * that was previously duplicated in BoardView.
 *
 * Used by:
 * - BoardView: Auto-refresh board when work units change
 * - AgentView: Realtime status updates in SessionHeader
 */

import { useEffect } from 'react';
import fs from 'fs';
import path from 'path';
import chokidar from 'chokidar';
import { useFspecStore } from '../store/fspecStore';

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
 * Hook that watches spec/work-units.json for changes and calls loadData on the store.
 *
 * @returns Object with getWorkUnitById function for querying work units
 */
export function useWorkUnitsWatcher(): UseWorkUnitsWatcherReturn {
  const cwd = useFspecStore(state => state.cwd);
  const loadData = useFspecStore(state => state.loadData);
  const workUnits = useFspecStore(state => state.workUnits);

  // Watch spec/work-units.json for changes
  useEffect(() => {
    const workUnitsPath = path.join(cwd, 'spec', 'work-units.json');

    // Check if file exists before watching
    if (!fs.existsSync(workUnitsPath)) {
      return;
    }

    // Chokidar watches specific file, handles atomic operations automatically
    const watcher = chokidar.watch(workUnitsPath, {
      ignoreInitial: true, // Don't trigger on initial scan
      persistent: false,
    });

    // Listen for all change events (chokidar normalizes across platforms)
    watcher.on('change', () => {
      void loadData();
    });

    // Add error handler to prevent silent failures
    watcher.on('error', error => {
      console.warn('Work units watcher error:', error.message);
    });

    // Cleanup watcher on unmount
    return () => {
      void watcher.close();
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
