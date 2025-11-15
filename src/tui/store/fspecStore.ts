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
import { promises as fsPromises } from 'fs';
import path from 'path';
import {
  getStagedFilesWithChangeType,
  getUnstagedFilesWithChangeType,
  type FileStatusWithChangeType,
} from '../../git/status';
import { logger } from '../../utils/logger';
import { moveWorkUnitInArray } from '../../utils/states-array';

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
  stashes: unknown[];
  stagedFiles: FileStatusWithChangeType[];
  unstagedFiles: FileStatusWithChangeType[];
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
        const err = error as Error;
        console.error('[ZUSTAND] loadData error:', err);
        console.error('[ZUSTAND] Stack trace:', err.stack);

        // INIT-016: Only handle corrupted JSON gracefully (temporary state during upgrades)
        // Other errors (permissions, disk full, bugs) should be shown to user
        // Check for JSON parse errors using both error type and message pattern
        const isJsonParseError =
          err instanceof SyntaxError ||
          err.message.includes('Failed to parse') ||
          err.message.includes('JSON');

        if (isJsonParseError) {
          console.warn(
            '[ZUSTAND] Corrupted JSON detected, showing empty board and watching for fix'
          );
          set(state => {
            state.workUnits = [];
            state.epics = [];
            state.isLoaded = true;
            state.error = null; // Clear error to show empty board, file watcher will reload when fixed
          });
        } else {
          // Real error (permission denied, disk full, migration failure, code bug) - show to user
          set(state => {
            state.error = err.stack || err.message;
          });
        }
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
      } catch {
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
          getStagedFilesWithChangeType(cwd),
          getUnstagedFilesWithChangeType(cwd),
        ]);
        set(state => {
          state.stagedFiles = staged;
          state.unstagedFiles = unstaged;
        });
      } catch {
        set(state => {
          state.stagedFiles = [];
          state.unstagedFiles = [];
        });
      }
    },

    loadCheckpointCounts: async () => {
      logger.debug('[ZUSTAND] loadCheckpointCounts() called');
      try {
        const cwd = get().cwd;
        logger.debug(`[ZUSTAND] Loading checkpoints from cwd: ${cwd}`);

        const checkpointIndexDir = path.join(
          cwd,
          '.git',
          'fspec-checkpoints-index'
        );
        logger.debug(`[ZUSTAND] Checkpoint index dir: ${checkpointIndexDir}`);

        // Read all JSON files in checkpoint index directory
        const files = await fsPromises.readdir(checkpointIndexDir);
        logger.debug(
          `[ZUSTAND] Found ${files.length} files in checkpoint index`
        );
        const jsonFiles = files.filter(f => f.endsWith('.json'));
        logger.debug(`[ZUSTAND] Found ${jsonFiles.length} JSON files`);

        let manual = 0;
        let auto = 0;

        // Parse each JSON file and count checkpoints
        for (const jsonFile of jsonFiles) {
          const filePath = path.join(checkpointIndexDir, jsonFile);
          const content = await fsPromises.readFile(filePath, 'utf-8');
          const data = JSON.parse(content);

          logger.debug(
            `[ZUSTAND] Parsing ${jsonFile}: found ${(data.checkpoints || []).length} checkpoints`
          );

          // Count manual vs auto checkpoints based on name pattern
          for (const checkpoint of data.checkpoints || []) {
            if (checkpoint.name.includes('-auto-')) {
              auto++;
            } else {
              manual++;
            }
          }
        }

        logger.debug(
          `[ZUSTAND] Checkpoint counts calculated: manual=${manual}, auto=${auto}`
        );
        logger.debug(
          `[ZUSTAND] Current state before update: ${JSON.stringify(get().checkpointCounts)}`
        );

        set(state => {
          state.checkpointCounts = { manual, auto };
        });

        logger.debug(
          `[ZUSTAND] State updated. New state: ${JSON.stringify(get().checkpointCounts)}`
        );
      } catch (error) {
        logger.error(`[ZUSTAND] Error loading checkpoint counts: ${error}`);
        set(state => {
          state.checkpointCounts = { manual: 0, auto: 0 };
        });
        logger.debug(`[ZUSTAND] State reset to zero due to error`);
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
        const workUnitsPath = path.join(state.cwd, 'spec', 'work-units.json');

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
        const workUnitsPath = path.join(state.cwd, 'spec', 'work-units.json');

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
