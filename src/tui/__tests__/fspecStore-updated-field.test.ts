/**
 * Feature: spec/features/animated-shimmer-on-last-changed-work-unit.feature
 *
 * Tests for Zustand store passing updated timestamps to components.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { useFspecStore } from '../store/fspecStore';
import * as ensureFiles from '../../utils/ensure-files';

describe('Feature: Animated shimmer on last changed work unit', () => {
  beforeEach(() => {
    // Reset store state before each test
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
    });

    // Clear all mocks
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Zustand store passes updated timestamp from work-units.json to components', () => {
    it('should preserve updated and estimate fields when loading from work-units.json', async () => {
      // @step Given work-units.json contains work unit with updated timestamp
      const mockWorkUnitsData = {
        workUnits: {
          'BOARD-009': {
            id: 'BOARD-009',
            title: 'Test Work Unit',
            status: 'implementing',
            type: 'story',
            estimate: 5,
            updated: '2025-10-27T22:00:00Z',
          },
          'BOARD-008': {
            id: 'BOARD-008',
            title: 'Another Work Unit',
            status: 'testing',
            type: 'bug',
            estimate: 3,
            updated: '2025-10-27T21:30:00Z',
          },
        },
      };

      const mockEpicsData = {
        epics: {},
      };

      // Mock the ensure functions
      vi.spyOn(ensureFiles, 'ensureWorkUnitsFile').mockResolvedValue(
        mockWorkUnitsData
      );
      vi.spyOn(ensureFiles, 'ensureEpicsFile').mockResolvedValue(mockEpicsData);

      // Verify store is empty before loading
      expect(useFspecStore.getState().workUnits).toHaveLength(0);
      expect(useFspecStore.getState().isLoaded).toBe(false);

      // @step When Zustand store loads data from work-units.json
      await useFspecStore.getState().loadData();

      // @step Then WorkUnit interface must include updated and estimate fields
      const loadedWorkUnits = useFspecStore.getState().workUnits;
      expect(loadedWorkUnits).toHaveLength(2);
      expect(useFspecStore.getState().isLoaded).toBe(true);

      // Verify first work unit preserves all fields
      const boardNine = loadedWorkUnits.find(wu => wu.id === 'BOARD-009');
      expect(boardNine).toBeDefined();
      expect(boardNine?.id).toBe('BOARD-009');
      expect(boardNine?.title).toBe('Test Work Unit');
      expect(boardNine?.status).toBe('implementing');
      expect(boardNine?.type).toBe('story');
      expect(boardNine?.updated).toBe('2025-10-27T22:00:00Z');
      expect(boardNine?.estimate).toBe(5);

      // Verify second work unit preserves all fields
      const boardEight = loadedWorkUnits.find(wu => wu.id === 'BOARD-008');
      expect(boardEight).toBeDefined();
      expect(boardEight?.id).toBe('BOARD-008');
      expect(boardEight?.title).toBe('Another Work Unit');
      expect(boardEight?.status).toBe('testing');
      expect(boardEight?.type).toBe('bug');
      expect(boardEight?.updated).toBe('2025-10-27T21:30:00Z');
      expect(boardEight?.estimate).toBe(3);

      // @step And UnifiedBoardLayout receives work units with updated timestamps
      // Verify the interface contract: components receive updated and estimate fields
      loadedWorkUnits.forEach(wu => {
        expect(wu).toHaveProperty('updated');
        expect(wu).toHaveProperty('estimate');
        expect(typeof wu.updated).toBe('string');
        expect(typeof wu.estimate).toBe('number');
      });

      // Verify ensure functions were called correctly
      expect(ensureFiles.ensureWorkUnitsFile).toHaveBeenCalledOnce();
      expect(ensureFiles.ensureEpicsFile).toHaveBeenCalledOnce();
    });

    it('should handle work units without estimate or updated fields gracefully', async () => {
      // @step Given work-units.json contains work unit WITHOUT updated/estimate
      const mockWorkUnitsData = {
        workUnits: {
          'TASK-001': {
            id: 'TASK-001',
            title: 'Task without estimate',
            status: 'backlog',
            type: 'task',
            // No estimate or updated fields
          },
        },
      };

      const mockEpicsData = {
        epics: {},
      };

      vi.spyOn(ensureFiles, 'ensureWorkUnitsFile').mockResolvedValue(
        mockWorkUnitsData
      );
      vi.spyOn(ensureFiles, 'ensureEpicsFile').mockResolvedValue(mockEpicsData);

      // @step When Zustand store loads data from work-units.json
      await useFspecStore.getState().loadData();

      // @step Then WorkUnit interface should allow optional updated/estimate
      const loadedWorkUnits = useFspecStore.getState().workUnits;
      expect(loadedWorkUnits).toHaveLength(1);

      const task = loadedWorkUnits[0];
      expect(task.id).toBe('TASK-001');
      expect(task.updated).toBeUndefined();
      expect(task.estimate).toBeUndefined();

      // Should not throw error - fields are optional
    });
  });
});
