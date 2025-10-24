/**
 * Feature: spec/features/data-integrity-validation-missing-in-prioritize-work-unit.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../../types';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';
import { prioritizeWorkUnit } from '../prioritize-work-unit';

describe('Feature: Data integrity validation missing in prioritize-work-unit', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(
      process.cwd(),
      'test-tmp-prioritize-work-unit-data-integrity'
    );
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec/features'), { recursive: true });

    // Create foundation.json for all tests (required by commands)
    await createMinimalFoundation(testDir);
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Detect work unit in wrong states array', () => {
    it('should throw error when work unit status does not match its states array', async () => {
      // Given work unit AUTH-001 has status "specifying"
      // But AUTH-001 is in the states.testing array instead of states.specifying
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Feature 1',
            status: 'specifying', // Status says specifying
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [], // Should be here but isn't
          testing: ['AUTH-001'], // Actually here (data corruption)
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec prioritize-work-unit AUTH-001 --position top"
      const error = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 'top',
        cwd: testDir,
      }).catch((e: Error) => e);

      // Then the command should fail
      expect(error).toBeInstanceOf(Error);

      if (error instanceof Error) {
        // And the error message should contain "Data integrity error"
        expect(error.message).toContain('Data integrity error');
        // And the error message should contain "AUTH-001 has status 'specifying' but is not in states.specifying array"
        expect(error.message).toContain('AUTH-001');
        expect(error.message).toContain('specifying');
        expect(error.message).toContain('states.specifying');
        // And the error message should contain "Run 'fspec repair-work-units' to fix data corruption"
        expect(error.message).toContain('fspec repair-work-units');
      }
    });
  });

  describe('Scenario: Detect --before target in wrong states array', () => {
    it('should throw error when --before target is not in the same states array', async () => {
      // Given work unit FEAT-017 is in specifying status
      // And FEAT-017 is correctly in the states.specifying array
      // And work unit AUTH-001 has status "specifying"
      // But AUTH-001 is NOT in the states.specifying array
      const workUnits: WorkUnitsData = {
        workUnits: {
          'FEAT-017': {
            id: 'FEAT-017',
            title: 'Feature 17',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth Feature',
            status: 'specifying', // Status says specifying
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['FEAT-017'], // Only FEAT-017, not AUTH-001
          testing: ['AUTH-001'], // AUTH-001 is here (data corruption)
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec prioritize-work-unit FEAT-017 --before AUTH-001"
      const error = await prioritizeWorkUnit({
        workUnitId: 'FEAT-017',
        before: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      // Then the command should fail
      expect(error).toBeInstanceOf(Error);

      if (error instanceof Error) {
        // And the error message should contain "Data integrity error"
        expect(error.message).toContain('Data integrity error');
        // And the error message should contain "AUTH-001 has status 'specifying' but is not in states.specifying array"
        expect(error.message).toContain('AUTH-001');
        expect(error.message).toContain('specifying');
        expect(error.message).toContain('states.specifying');
      }
    });
  });

  describe('Scenario: Valid data passes integrity check', () => {
    it('should succeed when work unit is in correct states array', async () => {
      // Given work unit AUTH-001 has status "implementing"
      // And AUTH-001 is correctly in the states.implementing array
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth Feature',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Feature 2',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-002', 'AUTH-001'], // Both in correct array
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec prioritize-work-unit AUTH-001 --position top"
      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 'top',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      // And AUTH-001 should be first in the states.implementing array
      expect(updated.states.implementing[0]).toBe('AUTH-001');
    });
  });
});
