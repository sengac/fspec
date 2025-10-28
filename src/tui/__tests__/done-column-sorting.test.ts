/**
 * Feature: spec/features/work-unit-details-panel-shows-incorrect-work-unit-after-reordering.feature
 *
 * Tests for done column display order and work unit details panel correctness.
 * Verifies that TUI displays work units in exact file array order (no runtime sorting).
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { useFspecStore } from '../store/fspecStore';
import * as ensureFiles from '../../utils/ensure-files';

describe('Feature: Work unit details panel shows incorrect work unit after reordering', () => {
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

    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Done column displays work units sorted by most recent updated timestamp', () => {
    it('should display done column work units in order matching states.done array', async () => {
      // @step Given the done column has 3 work units with updated timestamps
      const mockWorkUnitsData = {
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T10:00:00Z',
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Work Unit 2',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T11:00:00Z', // Most recent
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T09:00:00Z', // Oldest
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['BOARD-002', 'BOARD-001', 'BOARD-003'], // Sorted by timestamp (most recent first)
          blocked: [],
        },
      };

      const mockEpicsData = {
        epics: {},
      };

      vi.spyOn(ensureFiles, 'ensureWorkUnitsFile').mockResolvedValue(
        mockWorkUnitsData
      );
      vi.spyOn(ensureFiles, 'ensureEpicsFile').mockResolvedValue(mockEpicsData);

      // @step When I view the TUI board
      await useFspecStore.getState().loadData();

      const loadedWorkUnits = useFspecStore.getState().workUnits;

      // @step Then the done column should display work units in order: BOARD-002, BOARD-001, BOARD-003
      const doneWorkUnits = loadedWorkUnits.filter(wu => wu.status === 'done');
      expect(doneWorkUnits).toHaveLength(3);
      expect(doneWorkUnits[0].id).toBe('BOARD-002'); // Position 0 (most recent)
      expect(doneWorkUnits[1].id).toBe('BOARD-001'); // Position 1
      expect(doneWorkUnits[2].id).toBe('BOARD-003'); // Position 2 (oldest)

      // @step And when I select position 0 in done column
      // @step Then the Work Unit Details panel should show BOARD-002 title and description
      const selectedWorkUnit = doneWorkUnits[0];
      expect(selectedWorkUnit.id).toBe('BOARD-002');
      expect(selectedWorkUnit.title).toBe('Work Unit 2');
      expect(selectedWorkUnit.updated).toBe('2025-10-28T11:00:00Z');
    });
  });

  describe('Scenario: TUI displays done column in exact file array order', () => {
    it('should display work units in exact states.done array order without runtime sorting', async () => {
      // @step Given the states.done array in work-units.json is [BOARD-003, BOARD-005, BOARD-001]
      const mockWorkUnitsData = {
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T09:00:00Z',
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T11:00:00Z',
          },
          'BOARD-005': {
            id: 'BOARD-005',
            title: 'Work Unit 5',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T10:00:00Z',
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['BOARD-003', 'BOARD-005', 'BOARD-001'], // Already sorted in file
          blocked: [],
        },
      };

      const mockEpicsData = {
        epics: {},
      };

      vi.spyOn(ensureFiles, 'ensureWorkUnitsFile').mockResolvedValue(
        mockWorkUnitsData
      );
      vi.spyOn(ensureFiles, 'ensureEpicsFile').mockResolvedValue(mockEpicsData);

      // @step When I view the TUI board and navigate to done column
      await useFspecStore.getState().loadData();

      const loadedWorkUnits = useFspecStore.getState().workUnits;
      const doneWorkUnits = loadedWorkUnits.filter(wu => wu.status === 'done');

      // @step Then position 0 should display BOARD-003
      expect(doneWorkUnits[0].id).toBe('BOARD-003');

      // @step And position 1 should display BOARD-005
      expect(doneWorkUnits[1].id).toBe('BOARD-005');

      // @step And position 2 should display BOARD-001
      expect(doneWorkUnits[2].id).toBe('BOARD-001');

      // @step And when I select position 1
      // @step Then the Work Unit Details panel should show BOARD-005 (not BOARD-001)
      const selectedAtPosition1 = doneWorkUnits[1];
      expect(selectedAtPosition1.id).toBe('BOARD-005');
      expect(selectedAtPosition1.title).toBe('Work Unit 5');
      expect(selectedAtPosition1.id).not.toBe('BOARD-001'); // Explicitly verify it's NOT BOARD-001
    });

    it('should maintain file order after loadData is called multiple times', async () => {
      // Verify that loadData doesn't introduce runtime sorting
      const mockWorkUnitsData = {
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T09:00:00Z',
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Work Unit 2',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T12:00:00Z', // Most recent
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T10:00:00Z',
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['BOARD-003', 'BOARD-001', 'BOARD-002'], // Specific order in file
          blocked: [],
        },
      };

      const mockEpicsData = {
        epics: {},
      };

      vi.spyOn(ensureFiles, 'ensureWorkUnitsFile').mockResolvedValue(
        mockWorkUnitsData
      );
      vi.spyOn(ensureFiles, 'ensureEpicsFile').mockResolvedValue(mockEpicsData);

      // Load data first time
      await useFspecStore.getState().loadData();
      let doneWorkUnits = useFspecStore
        .getState()
        .workUnits.filter(wu => wu.status === 'done');
      const firstLoadOrder = doneWorkUnits.map(wu => wu.id);

      // Load data second time (simulating refresh)
      await useFspecStore.getState().loadData();
      doneWorkUnits = useFspecStore
        .getState()
        .workUnits.filter(wu => wu.status === 'done');
      const secondLoadOrder = doneWorkUnits.map(wu => wu.id);

      // Order should be identical and match file order
      expect(firstLoadOrder).toEqual(['BOARD-003', 'BOARD-001', 'BOARD-002']);
      expect(secondLoadOrder).toEqual(['BOARD-003', 'BOARD-001', 'BOARD-002']);
      expect(firstLoadOrder).toEqual(secondLoadOrder);
    });
  });

  describe('Scenario: Other columns maintain manual order from states arrays', () => {
    it('should NOT sort non-done columns by timestamp', async () => {
      // Verify that only done column is sorted, all others maintain file order
      const mockWorkUnitsData = {
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Backlog Unit 1',
            status: 'backlog',
            type: 'story',
            updated: '2025-10-28T12:00:00Z', // Most recent
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Backlog Unit 2',
            status: 'backlog',
            type: 'story',
            updated: '2025-10-28T09:00:00Z', // Oldest
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Backlog Unit 3',
            status: 'backlog',
            type: 'story',
            updated: '2025-10-28T10:00:00Z',
          },
        },
        states: {
          backlog: ['BOARD-002', 'BOARD-003', 'BOARD-001'], // Manual order (NOT sorted by timestamp)
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      const mockEpicsData = {
        epics: {},
      };

      vi.spyOn(ensureFiles, 'ensureWorkUnitsFile').mockResolvedValue(
        mockWorkUnitsData
      );
      vi.spyOn(ensureFiles, 'ensureEpicsFile').mockResolvedValue(mockEpicsData);

      await useFspecStore.getState().loadData();

      const loadedWorkUnits = useFspecStore.getState().workUnits;
      const backlogWorkUnits = loadedWorkUnits.filter(
        wu => wu.status === 'backlog'
      );

      // Should match file order, NOT sorted by timestamp
      expect(backlogWorkUnits[0].id).toBe('BOARD-002');
      expect(backlogWorkUnits[1].id).toBe('BOARD-003');
      expect(backlogWorkUnits[2].id).toBe('BOARD-001');

      // Verify it's NOT sorted by timestamp (which would be BOARD-001, BOARD-003, BOARD-002)
      expect(backlogWorkUnits.map(wu => wu.id)).not.toEqual([
        'BOARD-001',
        'BOARD-003',
        'BOARD-002',
      ]);
    });
  });
});
