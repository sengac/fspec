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
  isLoaded: boolean;
  error: string | null;
  cwd: string;

  // Actions
  setCwd: (cwd: string) => void;
  loadData: () => Promise<void>;
  loadStashes: () => Promise<void>;
  loadFileStatus: () => Promise<void>;
  updateWorkUnitStatus: (id: string, status: string) => void;
  addWorkUnit: (workUnit: WorkUnit) => void;

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
        set(state => {
          state.workUnits = Object.values(workUnitsData.workUnits);
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

    getWorkUnitsByStatus: (status: string) => {
      return get().workUnits.filter(wu => wu.status === status);
    },

    getWorkUnitsByEpic: (epicId: string) => {
      return get().workUnits.filter(wu => wu.epic === epicId);
    },
  }))
);
