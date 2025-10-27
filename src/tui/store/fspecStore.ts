/**
 * fspecStore - Zustand State Management
 *
 * Global state management for fspec TUI using Zustand with Immer middleware.
 * Provides immutable state updates for work units, epics, and other fspec data.
 *
 * Coverage:
 * - ITF-001: Zustand store updates trigger Ink component re-renders
 * - ITF-002: Load and sync all fspec data from JSON files
 */

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { ensureWorkUnitsFile } from '../../utils/ensure-files';
import { ensureEpicsFile } from '../../utils/ensure-files';

interface WorkUnit {
  id: string;
  title: string;
  status: string;
  type: string;
  description?: string;
  epic?: string;
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
  isLoaded: boolean;
  error: string | null;

  // Actions
  loadData: () => Promise<void>;
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
    isLoaded: false,
    error: null,

    loadData: async () => {
      set(state => {
        state.error = null;
      });
      try {
        const cwd = process.cwd();
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
