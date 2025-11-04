/**
 * Feature: spec/features/duplicate-work-unit-ids-in-state-arrays-when-moving-backward-or-to-same-state.feature
 *
 * BUG-064: Test cases for duplicate work unit IDs bug in state arrays
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { insertWorkUnitSorted } from '../states-array';
import type { WorkUnitsData } from '../../types';

describe('Feature: Duplicate work unit IDs in state arrays when moving backward or to same state', () => {
  let workUnitsData: WorkUnitsData;

  beforeEach(() => {
    workUnitsData = {
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          title: 'Test Work Unit 1',
          type: 'story',
          prefix: 'TEST',
          status: 'testing',
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
        },
        'TEST-002': {
          id: 'TEST-002',
          title: 'Test Work Unit 2',
          type: 'story',
          prefix: 'TEST',
          status: 'implementing',
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
        },
        'TEST-003': {
          id: 'TEST-003',
          title: 'Test Work Unit 3',
          type: 'story',
          prefix: 'TEST',
          status: 'specifying',
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
        },
      },
      states: {
        backlog: [],
        specifying: ['TEST-003'],
        testing: ['TEST-001'],
        implementing: ['TEST-002'],
        validating: [],
        done: [],
        blocked: [],
      },
      nextWorkUnitNumbers: {},
    };
  });

  describe('Scenario: Moving work unit backward through states should not create duplicates', () => {
    it('should not create duplicate entries when moving backward and forward', () => {
      // @step Given a work unit TEST-001 exists in testing state
      expect(workUnitsData.states.testing).toContain('TEST-001');
      expect(workUnitsData.states.testing.length).toBe(1);

      // @step When I move it to specifying state
      let result = insertWorkUnitSorted(
        workUnitsData,
        'TEST-001',
        'testing',
        'specifying'
      );

      // Verify it was moved correctly
      expect(result.states.testing).not.toContain('TEST-001');
      expect(result.states.specifying).toContain('TEST-001');

      // @step And I move it back to testing state
      result = insertWorkUnitSorted(
        result,
        'TEST-001',
        'specifying',
        'testing'
      );

      // @step Then the testing state array should contain TEST-001 exactly once
      const countInTesting = result.states.testing.filter(
        id => id === 'TEST-001'
      ).length;
      expect(countInTesting).toBe(1);

      // @step And the specifying state array should not contain TEST-001
      expect(result.states.specifying).not.toContain('TEST-001');
    });
  });

  describe('Scenario: Moving work unit to same state should be idempotent', () => {
    it('should not create duplicate when moving to same state', () => {
      // @step Given a work unit TEST-002 exists in implementing state
      expect(workUnitsData.states.implementing).toContain('TEST-002');
      expect(workUnitsData.states.implementing.length).toBe(1);

      // @step When I move it to implementing state again
      const result = insertWorkUnitSorted(
        workUnitsData,
        'TEST-002',
        'implementing',
        'implementing'
      );

      // @step Then the implementing state array should contain TEST-002 exactly once
      const countInImplementing = result.states.implementing.filter(
        id => id === 'TEST-002'
      ).length;
      expect(countInImplementing).toBe(1);
    });
  });

  describe('Scenario: Work unit should appear in exactly one state array after any transition', () => {
    it('should ensure work unit appears in exactly one state array', () => {
      // @step Given a work unit TEST-003 exists in any state
      expect(workUnitsData.states.specifying).toContain('TEST-003');

      // @step When I move it to a different state
      const result = insertWorkUnitSorted(
        workUnitsData,
        'TEST-003',
        'specifying',
        'testing'
      );

      // @step Then TEST-003 should appear in exactly one state array
      const allStates = Object.values(result.states).flat();
      const countAcrossAll = allStates.filter(id => id === 'TEST-003').length;
      expect(countAcrossAll).toBe(1);

      // @step And no state array should contain TEST-003 more than once
      for (const [stateName, stateArray] of Object.entries(result.states)) {
        const countInState = stateArray.filter(id => id === 'TEST-003').length;
        expect(countInState).toBeLessThanOrEqual(1);
      }
    });
  });
});
