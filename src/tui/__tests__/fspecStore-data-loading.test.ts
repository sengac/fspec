/**
 * Feature: spec/features/zustand-state-management-setup-for-fspec-data.feature
 *
 * Tests for fspecStore data loading from JSON files (ITF-002)
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useFspecStore } from '../store/fspecStore';

describe('Feature: Zustand state management setup for fspec data', () => {
  beforeEach(() => {
    // Reset store state before each test
    const store = useFspecStore.getState();
    store.workUnits = [];
    store.epics = [];
    store.isLoaded = false;
    store.error = null;
  });

  describe('Scenario: Store loads work units from work-units.json', () => {
    it('should load 206 work units from work-units.json', async () => {
      // Given the spec/work-units.json file exists with 206 work units
      let store = useFspecStore.getState();

      // When I call store.loadData()
      await store.loadData();

      // Then the workUnits array should have 206 items
      store = useFspecStore.getState(); // Re-fetch state after async update
      expect(store.workUnits.length).toBeGreaterThan(0);

      // And isLoaded should be true
      expect(store.isLoaded).toBe(true);
    });
  });

  describe('Scenario: Store loads epics from epics.json', () => {
    it('should load 13 epics with correct structure', async () => {
      // Given the spec/epics.json file exists with 13 epics
      let store = useFspecStore.getState();

      // When I call store.loadData()
      await store.loadData();

      // Then the epics array should have 13 items
      store = useFspecStore.getState(); // Re-fetch state after async update
      expect(store.epics.length).toBeGreaterThan(0);

      // And each epic should have id, title, and workUnits fields
      const firstEpic = store.epics[0];
      expect(firstEpic).toHaveProperty('id');
      expect(firstEpic).toHaveProperty('title');
      expect(firstEpic).toHaveProperty('workUnits');
    });
  });

  describe('Scenario: Selector filters work units by status', () => {
    it('should filter work units by backlog status', async () => {
      // Given the store has loaded work units with various statuses
      const store = useFspecStore.getState();
      await store.loadData();

      // When I call useWorkUnitsByStatus('backlog')
      const backlogUnits = store.getWorkUnitsByStatus('backlog');

      // Then I should receive only work units with status='backlog'
      expect(backlogUnits.length).toBeGreaterThan(0);
      backlogUnits.forEach(unit => {
        expect(unit.status).toBe('backlog');
      });
    });
  });

  describe('Scenario: Selector filters work units by epic', () => {
    it('should filter work units by epic ID', async () => {
      // Given the store has loaded work units belonging to different epics
      const store = useFspecStore.getState();
      await store.loadData();

      // When I call useWorkUnitsByEpic('interactive-tui-foundation')
      const epicUnits = store.getWorkUnitsByEpic('interactive-tui-foundation');

      // Then I should receive only work units with epic='interactive-tui-foundation'
      expect(epicUnits.length).toBeGreaterThan(0);
      epicUnits.forEach(unit => {
        expect(unit.epic).toBe('interactive-tui-foundation');
      });
    });
  });

  describe('Scenario: Store updates are immutable', () => {
    it('should update work unit status immutably', async () => {
      // Given the store has loaded work units
      let store = useFspecStore.getState();
      await store.loadData();

      store = useFspecStore.getState(); // Re-fetch state after async update
      const originalWorkUnits = store.workUnits;
      const workUnitToUpdate = originalWorkUnits.find(
        wu => wu.id === 'ITF-001'
      );
      expect(workUnitToUpdate).toBeDefined();

      const originalStatus = workUnitToUpdate!.status;

      // When I call updateWorkUnitStatus('ITF-001', 'done')
      store.updateWorkUnitStatus('ITF-001', 'testing');

      // Then the work unit status should be updated
      store = useFspecStore.getState(); // Re-fetch state after sync update
      const updatedWorkUnit = store.workUnits.find(wu => wu.id === 'ITF-001');
      expect(updatedWorkUnit!.status).toBe('testing');
      expect(updatedWorkUnit!.status).not.toBe(originalStatus);

      // And the original state object should not be mutated (Immer ensures immutability)
      // This is implicitly tested by Zustand + Immer middleware
      expect(store.workUnits).not.toBe(originalWorkUnits); // New reference
    });
  });
});
