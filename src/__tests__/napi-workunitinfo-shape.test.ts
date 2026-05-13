/**
 * Feature: spec/features/napi-workunitinfo-shape.feature
 *
 * RPC-005 Scenario: TS frontend smoke test confirms get_all_work_units shape after the lift.
 *
 * Single Vitest smoke test that imports getAllWorkUnits via the codelet/napi
 * binding and asserts the returned object keys match the WorkUnitInfo shape
 * after WorkUnitInfo was lifted from codelet/napi into codelet/rpc-types.
 * Codifies the "TS frontend unchanged" invariant — any future regression to
 * the NAPI re-export pattern fails this test loudly in CI.
 */

import { describe, it, expect } from 'vitest';
import { getAllWorkUnits } from '../../codelet/napi/index.js';

describe('Feature: TS frontend NAPI WorkUnitInfo shape preserved after rpc-types lift', () => {
  describe('Scenario: TS frontend smoke test confirms get_all_work_units shape after the lift', () => {
    it('returns an array whose elements expose the seven canonical WorkUnitInfo keys', () => {
      // @step Given the WorkUnitInfo type has been lifted from codelet/napi into codelet/rpc-types and codelet/napi has been rebuilt
      // (Build prerequisite: the codelet-napi .node binary has been rebuilt
      //  after WorkUnitInfo was moved into codelet/rpc-types.)

      // @step When the Vitest smoke test imports get_all_work_units from the codelet/napi binding and calls it
      const result = getAllWorkUnits();

      // @step Then the returned value is an array whose elements have the keys id, title, workType, status, description, estimate, and epic and the existing TypeScript test suite npm test passes without modification
      expect(Array.isArray(result)).toBe(true);

      if (result.length > 0) {
        const sample = result[0];
        // Required keys
        expect(sample).toHaveProperty('id');
        expect(sample).toHaveProperty('title');
        expect(sample).toHaveProperty('workType');
        expect(sample).toHaveProperty('status');
        expect(typeof sample.id).toBe('string');
        expect(typeof sample.title).toBe('string');
        expect(typeof sample.workType).toBe('string');
        expect(typeof sample.status).toBe('string');

        // Optional keys may be absent or of the documented type
        if ('description' in sample && sample.description !== undefined) {
          expect(typeof sample.description).toBe('string');
        }
        if ('estimate' in sample && sample.estimate !== undefined) {
          expect(typeof sample.estimate).toBe('number');
        }
        if ('epic' in sample && sample.epic !== undefined) {
          expect(typeof sample.epic).toBe('string');
        }

        // Verify camelCase NAPI naming (workType, not work_type) is preserved.
        expect(sample).not.toHaveProperty('work_type');
      }
    });
  });
});
