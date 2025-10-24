/**
 * Feature: spec/features/work-unit-ordering-across-all-kanban-columns.feature
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

describe('Feature: Work unit ordering across all Kanban columns', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(
      process.cwd(),
      'test-tmp-work-unit-ordering-across-all-kanban-columns'
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

  describe('Scenario: Prioritize work unit within specifying column', () => {
    it('should reorder work unit to top of specifying column', async () => {
      // Given work unit FEAT-017 is in specifying status
      // And the specifying column contains multiple work units
      const workUnits: WorkUnitsData = {
        workUnits: {
          'FEAT-016': {
            id: 'FEAT-016',
            title: 'Feature 16',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'FEAT-017': {
            id: 'FEAT-017',
            title: 'Feature 17',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'FEAT-018': {
            id: 'FEAT-018',
            title: 'Feature 18',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['FEAT-016', 'FEAT-017', 'FEAT-018'],
          testing: [],
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

      // When I run "fspec prioritize-work-unit FEAT-017 --position top"
      const result = await prioritizeWorkUnit({
        workUnitId: 'FEAT-017',
        position: 'top',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      // And FEAT-017 should be first in the states.specifying array
      expect(updated.states.specifying[0]).toBe('FEAT-017');

      // And FEAT-017 should remain in specifying status
      expect(updated.workUnits['FEAT-017'].status).toBe('specifying');
    });
  });

  describe('Scenario: Prioritize work unit within implementing column', () => {
    it('should reorder work unit to top of implementing column', async () => {
      // Given work unit AUTH-001 is in implementing status
      // And the implementing column contains multiple work units
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
          implementing: ['AUTH-002', 'AUTH-001'],
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

      // And AUTH-001 should remain in implementing status
      expect(updated.workUnits['AUTH-001'].status).toBe('implementing');
    });
  });

  describe('Scenario: Error when prioritizing across columns using --before', () => {
    it('should fail when trying to prioritize across different columns', async () => {
      // Given work unit FEAT-017 is in specifying status
      // And work unit AUTH-001 is in testing status
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
            title: 'Auth Test',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['FEAT-017'],
          testing: ['AUTH-001'],
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
        // And the error message should contain "Cannot prioritize across columns"
        expect(error.message).toContain('Cannot prioritize across columns');

        // And the error message should contain "FEAT-017 (specifying) and AUTH-001 (testing) are in different columns"
        expect(error.message).toContain('FEAT-017');
        expect(error.message).toContain('specifying');
        expect(error.message).toContain('AUTH-001');
        expect(error.message).toContain('testing');
      }
    });
  });

  describe('Scenario: Error when prioritizing work unit in done column', () => {
    it('should fail when trying to prioritize done work unit', async () => {
      // Given work unit AUTH-001 is in done status
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth Feature',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
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
        // And the error message should contain "Cannot prioritize work units in done column"
        expect(error.message).toContain(
          'Cannot prioritize work units in done column'
        );

        // And a system-reminder should be emitted (checked via error message content)
        // The system-reminder explains that done items cannot be manually reordered
        // and lists allowed columns
        expect(error.message).toContain('backlog');
        expect(error.message).toContain('specifying');
        expect(error.message).toContain('testing');
        expect(error.message).toContain('implementing');
        expect(error.message).toContain('validating');
        expect(error.message).toContain('blocked');
      }
    });
  });

  describe('Scenario: Single work unit in column succeeds as no-op', () => {
    it('should succeed when prioritizing the only work unit in a column', async () => {
      // Given work unit AUTH-001 is the only work unit in implementing status
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth Feature',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
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

      // And AUTH-001 should remain the only work unit in implementing status
      expect(updated.states.implementing).toEqual(['AUTH-001']);
      expect(updated.workUnits['AUTH-001'].status).toBe('implementing');
    });
  });

  // Note: Scenario 6 "Command description updated to reflect multi-column support"
  // is tested via integration test checking help text output
  // This would require executing the CLI and checking --help output
});
