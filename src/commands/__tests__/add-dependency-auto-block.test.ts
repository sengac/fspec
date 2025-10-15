/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Adding blockedBy dependency auto-sets work unit to blocked state
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { addDependency } from '../add-dependency';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Adding blockedBy dependency auto-sets work unit to blocked state', () => {
    it('should auto-transition work unit to blocked when adding blockedBy dependency', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work unit "UI-001" exists with status "backlog"
      // And work unit "API-001" exists with status "implementing"
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'UI-001': {
            id: 'UI-001',
            title: 'UI work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['UI-001'],
          specifying: [],
          testing: [],
          implementing: ['API-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec add-dependency UI-001 --blocked-by=API-001"
      const result = await addDependency({
        workUnitId: 'UI-001',
        blockedBy: 'API-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // Read the updated work units file
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      // And work unit "UI-001" status should automatically change to "blocked"
      expect(updatedData.workUnits['UI-001'].status).toBe('blocked');

      // And work unit "UI-001" blockedReason should be "Blocked by API-001"
      expect(updatedData.workUnits['UI-001'].blockedReason).toBe(
        'Blocked by API-001'
      );

      // And work unit "API-001" blocks array should contain "UI-001"
      expect(updatedData.workUnits['API-001'].blocks).toContain('UI-001');

      // Verify state arrays were updated correctly
      expect(updatedData.states.backlog).not.toContain('UI-001');
      expect(updatedData.states.blocked).toContain('UI-001');
    });
  });
});
