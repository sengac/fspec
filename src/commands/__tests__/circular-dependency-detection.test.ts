/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Circular dependency detection prevents A→B→A loops
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { addDependency } from '../add-dependency';
import type { WorkUnitsData } from '../../types';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Work Unit Dependency Management', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('circular-dependency-detection');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Circular dependency detection prevents A→B→A loops', () => {
    it('should prevent creating circular dependencies', async () => {
      // Given I have a project with spec directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // And work unit "AUTH-001" blocks "API-001"
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth work unit',
            status: 'implementing',
            blocks: ['API-001'], // AUTH-001 already blocks API-001
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
            status: 'blocked',
            blockedBy: ['AUTH-001'], // API-001 is already blocked by AUTH-001
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: ['API-001'],
        },
      };

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // When I run "fspec add-dependency API-001 --blocks=AUTH-001"
      // Then the command should fail
      await expect(
        addDependency({
          workUnitId: 'API-001',
          blocks: 'AUTH-001',
          cwd: setup.testDir,
        })
      ).rejects.toThrow();

      // And the error should contain "Circular dependency detected"
      try {
        await addDependency({
          workUnitId: 'API-001',
          blocks: 'AUTH-001',
          cwd: setup.testDir,
        });
        // Should not reach here
        expect(true).toBe(false);
      } catch (error: any) {
        expect(error.message).toContain('Circular dependency detected');
        // And the error should show cycle: "AUTH-001 → API-001 → AUTH-001"
        expect(error.message).toMatch(/API-001.*AUTH-001/);
      }

      // And no circular dependency should be created
      const finalContent = await readFile(
        join(setup.testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const finalData: WorkUnitsData = JSON.parse(finalContent);

      // Verify API-001 does NOT have AUTH-001 in its blocks array
      expect(finalData.workUnits['API-001'].blocks).toBeUndefined();

      // Verify AUTH-001 does NOT have API-001 in its blockedBy array
      expect(finalData.workUnits['AUTH-001'].blockedBy).toBeUndefined();

      // Verify original dependencies are still intact
      expect(finalData.workUnits['AUTH-001'].blocks).toContain('API-001');
      expect(finalData.workUnits['API-001'].blockedBy).toContain('AUTH-001');
    });
  });
});
