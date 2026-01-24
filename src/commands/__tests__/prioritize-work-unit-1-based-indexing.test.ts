/**
 * Feature: spec/features/1-based-indexing-not-implemented-in-prioritize-work-unit.feature
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

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: 1-based indexing not implemented in prioritize-work-unit', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('prioritize-work-unit-1-based-indexing');
    await mkdir(join(testDir, 'spec/features'), { recursive: true });

    // Create foundation.json for all tests (required by commands)
    await createMinimalFoundation(testDir);
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Position 1 means first item', () => {
    it('should place work unit at first position when using --position 1', async () => {
      // Given work units AUTH-001, AUTH-002, AUTH-003 are in backlog
      // And they are ordered: AUTH-002, AUTH-003, AUTH-001
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Feature 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Feature 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Feature 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-002', 'AUTH-003', 'AUTH-001'],
          specifying: [],
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

      // When I run "fspec prioritize-work-unit AUTH-001 --position 1"
      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 1,
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      // And the backlog order should be: AUTH-001, AUTH-002, AUTH-003
      expect(updated.states.backlog).toEqual([
        'AUTH-001',
        'AUTH-002',
        'AUTH-003',
      ]);
      // And AUTH-001 should be first in backlog
      expect(updated.states.backlog[0]).toBe('AUTH-001');
    });
  });

  describe('Scenario: Position 3 means third item', () => {
    it('should place work unit at third position when using --position 3', async () => {
      // Given work units AUTH-001, AUTH-002, AUTH-003, AUTH-004 are in backlog
      // And they are ordered: AUTH-002, AUTH-003, AUTH-004, AUTH-001
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Feature 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Feature 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Feature 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-004': {
            id: 'AUTH-004',
            title: 'Feature 4',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-002', 'AUTH-003', 'AUTH-004', 'AUTH-001'],
          specifying: [],
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

      // When I run "fspec prioritize-work-unit AUTH-001 --position 3"
      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 3,
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      // And the backlog order should be: AUTH-002, AUTH-003, AUTH-001, AUTH-004
      expect(updated.states.backlog).toEqual([
        'AUTH-002',
        'AUTH-003',
        'AUTH-001',
        'AUTH-004',
      ]);
      // And AUTH-001 should be third in backlog
      expect(updated.states.backlog[2]).toBe('AUTH-001');
    });
  });

  describe('Scenario: Reject position 0 as invalid', () => {
    it('should fail when using --position 0', async () => {
      // Given work unit AUTH-001 is in backlog
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Feature 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
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

      // When I run "fspec prioritize-work-unit AUTH-001 --position 0"
      const error = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 0,
        cwd: testDir,
      }).catch((e: Error) => e);

      // Then the command should fail
      expect(error).toBeInstanceOf(Error);

      if (error instanceof Error) {
        // And the error message should contain "Invalid position: 0"
        expect(error.message).toContain('Invalid position: 0');
        // And the error message should contain "Position must be >= 1 (1-based index)"
        expect(error.message).toContain('Position must be >= 1');
        expect(error.message).toContain('1-based index');
      }
    });
  });

  describe('Scenario: Reject negative position as invalid', () => {
    it('should fail when using --position -1', async () => {
      // Given work unit AUTH-001 is in backlog
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Feature 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
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

      // When I run "fspec prioritize-work-unit AUTH-001 --position -1"
      const error = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: -1,
        cwd: testDir,
      }).catch((e: Error) => e);

      // Then the command should fail
      expect(error).toBeInstanceOf(Error);

      if (error instanceof Error) {
        // And the error message should contain "Invalid position: -1"
        expect(error.message).toContain('Invalid position: -1');
        // And the error message should contain "Position must be >= 1 (1-based index)"
        expect(error.message).toContain('Position must be >= 1');
        expect(error.message).toContain('1-based index');
      }
    });
  });
});
