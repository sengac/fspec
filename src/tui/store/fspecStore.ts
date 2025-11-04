/**
 * fspecStore - Zustand State Management
 *
 * Global state management for fspec TUI using Zustand with Immer middleware.
 * Provides immutable state updates for work units, epics, and other fspec data.
 *
 * Coverage:
 * - ITF-001: Zustand store updates trigger Ink component re-renders
 * - ITF-002: Load and sync all fspec data from JSON files
 * - ITF-005: Real-time file and git status watching in TUI
 */

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { ensureWorkUnitsFile } from '../../utils/ensure-files';
import { ensureEpicsFile } from '../../utils/ensure-files';
import { fileManager } from '../../utils/file-manager';
import git from 'isomorphic-git';
import fs from 'fs';
import { getStagedFiles, getUnstagedFiles } from '../../git/status';

interface StateHistoryEntry {
  state: string;
  timestamp: string;
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
  stateHistory?: StateHistoryEntry[];
}

interface Epic {
  id: string;
  title: string;
  workUnits: string[];
  description?: string;
  createdAt?: string;
}

interface FspecState {
  workUnits: WorkUnit[];
  epics: Epic[];
  stashes: any[];
  stagedFiles: string[];
  unstagedFiles: string[];
  checkpointCounts: { manual: number; auto: number };
  isLoaded: boolean;
  error: string | null;
  cwd: string;

  // Actions
  setCwd: (cwd: string) => void;
  loadData: () => Promise<void>;
  loadStashes: () => Promise<void>;
  loadFileStatus: () => Promise<void>;
  loadCheckpointCounts: () => Promise<void>;
  updateWorkUnitStatus: (id: string, status: string) => void;
  addWorkUnit: (workUnit: WorkUnit) => void;
  moveWorkUnitUp: (workUnitId: string) => Promise<void>;
  moveWorkUnitDown: (workUnitId: string) => Promise<void>;

  // Selectors
  getWorkUnitsByStatus: (status: string) => WorkUnit[];
  getWorkUnitsByEpic: (epicId: string) => WorkUnit[];
}

export const useFspecStore = create<FspecState>()(
  immer((set, get) => ({
    workUnits: [],
    epics: [],
    stashes: [],
    stagedFiles: [],
    unstagedFiles: [],
    checkpointCounts: { manual: 0, auto: 0 },
    isLoaded: false,
    error: null,
    cwd: process.cwd(),

    setCwd: (cwd: string) => {
      set(state => {
        state.cwd = cwd;
      });
    },

    loadData: async () => {
      set(state => {
        state.error = null;
      });
      try {
        const cwd = get().cwd;
        const workUnitsData = await ensureWorkUnitsFile(cwd);
        const epicsData = await ensureEpicsFile(cwd);

        // BOARD-010: Build workUnits array FROM states arrays to preserve display order
        // Iterate states arrays in column order, look up workUnit details by ID
        const orderedWorkUnits: WorkUnit[] = [];
        const columns = [
          'backlog',
          'specifying',
          'testing',
          'implementing',
          'validating',
          'done',
          'blocked',
        ];

        for (const column of columns) {
          const statesArray = workUnitsData.states[column] || [];
          for (const workUnitId of statesArray) {
            const workUnit = workUnitsData.workUnits[workUnitId];
            if (workUnit) {
              orderedWorkUnits.push(workUnit);
            }
          }
        }

        set(state => {
          state.workUnits = orderedWorkUnits;
          state.epics = Object.values(epicsData.epics);
          state.isLoaded = true;
        });
      } catch (error) {
        set(state => {
          state.error = (error as Error).message;
        });
      }
    },

    loadStashes: async () => {
      try {
        const cwd = get().cwd;
        const logs = await git.log({
          fs,
          dir: cwd,
          ref: 'refs/stash',
          depth: 10,
        });
        set(state => {
          state.stashes = logs;
        });
      } catch (error) {
        // No stashes exist or error reading - set to empty array
        set(state => {
          state.stashes = [];
        });
      }
    },

    loadFileStatus: async () => {
      try {
        const cwd = get().cwd;
        const [staged, unstaged] = await Promise.all([
          getStagedFiles(cwd),
          getUnstagedFiles(cwd),
        ]);
        set(state => {
          state.stagedFiles = staged;
          state.unstagedFiles = unstaged;
        });
      } catch (error) {
        set(state => {
          state.stagedFiles = [];
          state.unstagedFiles = [];
        });
      }
    },

    loadCheckpointCounts: async () => {
      try {
        const cwd = get().cwd;
        const path = await import('path');
        const fs = await import('fs/promises');
        const checkpointIndexDir = path.join(
          cwd,
          '.git',
          'fspec-checkpoints-index'
        );

        const files = await fs.readdir(checkpointIndexDir);
        const manual = files.filter(f => !f.includes('-auto-')).length;
        const auto = files.filter(f => f.includes('-auto-')).length;

        set(state => {
          state.checkpointCounts = { manual, auto };
        });
      } catch (error) {
        set(state => {
          state.checkpointCounts = { manual: 0, auto: 0 };
        });
      }
    },

    updateWorkUnitStatus: (id: string, status: string) => {
      set(state => {
        const workUnit = state.workUnits.find(wu => wu.id === id);
        if (workUnit) {
          workUnit.status = status;
        }
      });
    },

    addWorkUnit: (workUnit: WorkUnit) => {
      set(state => {
        state.workUnits.push(workUnit);
      });
    },

    moveWorkUnitUp: async (workUnitId: string) => {
      const state = get();
      const workUnit = state.workUnits.find(wu => wu.id === workUnitId);
      if (!workUnit) {
        return;
      }

      // Don't allow reordering in done column
      if (workUnit.status === 'done') {
        return;
      }

      try {
        const { join } = await import('path');
        const { moveWorkUnitInArray } = await import(
          '../../utils/states-array'
        );

        const workUnitsPath = join(state.cwd, 'spec', 'work-units.json');

        // LOCK-002: Use fileManager.transaction() for atomic read-modify-write
        await fileManager.transaction(workUnitsPath, async workUnitsData => {
          // BOARD-016: Use shared utility for array manipulation
          const updatedWorkUnitsData = moveWorkUnitInArray(
            workUnitsData,
            workUnitId,
            workUnit.status,
            'up'
          );

          // Check if any change was made
          if (
            updatedWorkUnitsData.states[workUnit.status] ===
            workUnitsData.states[workUnit.status]
          ) {
            return; // No change (already at top)
          }

          // Mutate data in place (transaction pattern)
          Object.assign(workUnitsData, updatedWorkUnitsData);
        });
      } catch (error) {
        console.error('Failed to persist work unit order:', error);
      }
    },

    moveWorkUnitDown: async (workUnitId: string) => {
      const state = get();
      const workUnit = state.workUnits.find(wu => wu.id === workUnitId);
      if (!workUnit) {
        return;
      }

      // Don't allow reordering in done column
      if (workUnit.status === 'done') {
        return;
      }

      try {
        const { join } = await import('path');
        const { moveWorkUnitInArray } = await import(
          '../../utils/states-array'
        );

        const workUnitsPath = join(state.cwd, 'spec', 'work-units.json');

        // LOCK-002: Use fileManager.transaction() for atomic read-modify-write
        await fileManager.transaction(workUnitsPath, async workUnitsData => {
          // BOARD-016: Use shared utility for array manipulation
          const updatedWorkUnitsData = moveWorkUnitInArray(
            workUnitsData,
            workUnitId,
            workUnit.status,
            'down'
          );

          // Check if any change was made
          if (
            updatedWorkUnitsData.states[workUnit.status] ===
            workUnitsData.states[workUnit.status]
          ) {
            return; // No change (already at bottom)
          }

          // Mutate data in place (transaction pattern)
          Object.assign(workUnitsData, updatedWorkUnitsData);
        });
      } catch (error) {
        console.error('Failed to persist work unit order:', error);
      }
    },

    getWorkUnitsByStatus: (status: string) => {
      return get().workUnits.filter(wu => wu.status === status);
    },

    getWorkUnitsByEpic: (epicId: string) => {
      return get().workUnits.filter(wu => wu.epic === epicId);
    },
  }))
);
