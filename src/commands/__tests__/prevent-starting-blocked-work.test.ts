/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Prevent starting work that is blocked
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Prevent starting work that is blocked', () => {
    it('should prevent moving blocked work unit to active state when blockers are incomplete', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work unit "UI-001" is blocked by "AUTH-001"
      // And "AUTH-001" status is "implementing" (not done)
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
            blocks: ['UI-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'UI work unit',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            blockedReason: 'Blocked by AUTH-001',
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
          blocked: ['UI-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec update-work-unit-status UI-001 implementing"
      // Then the command should fail
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'UI-001',
          status: 'implementing',
          cwd: testDir,
        })
      ).rejects.toThrow();

      // And the error should contain "blocked by incomplete dependencies"
      try {
        await updateWorkUnitStatus({
          workUnitId: 'UI-001',
          status: 'implementing',
          cwd: testDir,
        });
        // Should not reach here
        expect(true).toBe(false);
      } catch (error: any) {
        expect(error.message).toContain('blocked by incomplete dependencies');
        // And the error should show "AUTH-001 (status: implementing)"
        expect(error.message).toContain('AUTH-001');
        expect(error.message).toContain('implementing');
      }
    });

    it('should allow moving work unit to active state when all blockers are done', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work unit "UI-001" is blocked by "AUTH-001"
      // And "AUTH-001" status is "done"
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth work unit',
            status: 'done',
            blocks: ['UI-001'],
            linkedFeatures: ['user-authentication'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'UI work unit',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            blockedReason: 'Blocked by AUTH-001',
            linkedFeatures: ['ui-components'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
          blocked: ['UI-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create linked feature files and coverage files for both work units
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

      // User authentication feature with full coverage
      await writeFile(
        join(testDir, 'spec/features/user-authentication.feature'),
        `@AUTH-001
Feature: User Authentication

  Scenario: User Login
    Given a user exists
    When they log in
    Then they are authenticated
`
      );

      await writeFile(
        join(testDir, 'spec/features/user-authentication.feature.coverage'),
        JSON.stringify({
          scenarios: [
            {
              name: 'User Login',
              testMappings: [
                {
                  file: 'src/__tests__/auth.test.ts',
                  lines: '10-20',
                  implMappings: [
                    {
                      file: 'src/auth.ts',
                      lines: [5, 6, 7],
                    },
                  ],
                },
              ],
            },
          ],
        })
      );

      // UI components feature with full coverage
      await writeFile(
        join(testDir, 'spec/features/ui-components.feature'),
        `@UI-001
Feature: UI Components

  Scenario: Display Login Form
    Given a user visits the login page
    When the page loads
    Then the login form is displayed
`
      );

      await writeFile(
        join(testDir, 'spec/features/ui-components.feature.coverage'),
        JSON.stringify({
          scenarios: [
            {
              name: 'Display Login Form',
              testMappings: [
                {
                  file: 'src/__tests__/ui.test.ts',
                  lines: '15-30',
                  implMappings: [
                    {
                      file: 'src/ui/login.ts',
                      lines: [10, 11, 12],
                    },
                  ],
                },
              ],
            },
          ],
        })
      );

      // When I run "fspec update-work-unit-status UI-001 implementing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'UI-001',
        status: 'implementing',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And UI-001 status should be "implementing"
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);
      expect(updatedData.workUnits['UI-001'].status).toBe('implementing');
      expect(updatedData.states.implementing).toContain('UI-001');
      expect(updatedData.states.blocked).not.toContain('UI-001');
    });

    it('should prevent starting work when multiple blockers are incomplete', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work unit "FEAT-001" is blocked by "AUTH-001" and "API-001"
      // And both blockers are not done
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
            blocks: ['FEAT-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
            status: 'testing',
            blocks: ['FEAT-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'FEAT-001': {
            id: 'FEAT-001',
            title: 'Feature work unit',
            status: 'blocked',
            blockedBy: ['AUTH-001', 'API-001'],
            blockedReason: 'Blocked by AUTH-001, API-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['API-001'],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: ['FEAT-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec update-work-unit-status FEAT-001 implementing"
      // Then the command should fail
      try {
        await updateWorkUnitStatus({
          workUnitId: 'FEAT-001',
          status: 'implementing',
          cwd: testDir,
        });
        // Should not reach here
        expect(true).toBe(false);
      } catch (error: any) {
        // And the error should show both blockers
        expect(error.message).toContain('AUTH-001');
        expect(error.message).toContain('API-001');
        expect(error.message).toContain('implementing');
        expect(error.message).toContain('testing');
      }
    });
  });
});
