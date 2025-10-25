/**
 * Feature: spec/features/review-command-with-done-status-integration.feature
 *
 * This test file validates the system-reminder integration when transitioning to done status.
 * Tests the acceptance criteria for suggesting review before finalizing work units.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Review command with done status integration', () => {
  let testDir: string;
  let consoleLogSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-review-reminder');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

    // Spy on console.log to capture system-reminder output
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
    consoleLogSpy.mockRestore();
  });

  describe('Scenario: System-reminder suggests review when transitioning to done status', () => {
    it('should display system-reminder suggesting fspec review when moving to done', async () => {
      // Given I have a work unit AUTH-001 in validating status
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['AUTH-001'],
          done: [],
          blocked: [],
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User authentication',
            type: 'story',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Test work unit',
            children: [],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
              { state: 'testing', timestamp: new Date().toISOString() },
              { state: 'implementing', timestamp: new Date().toISOString() },
              { state: 'validating', timestamp: new Date().toISOString() },
            ],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // When I run 'fspec update-work-unit-status AUTH-001 done'
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      });

      // Then the status should update to done
      expect(result.success).toBe(true);
      expect(result.newStatus).toBe('done');

      // And a system-reminder should be displayed suggesting 'fspec review AUTH-001'
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toMatch(/<system-reminder>/);
      expect(result.systemReminder).toContain('fspec review AUTH-001');
      expect(result.systemReminder).toMatch(/review|quality|completeness/i);
      expect(result.systemReminder).toMatch(/<\/system-reminder>/);
    });

    it('should NOT display system-reminder when transitioning to other statuses', async () => {
      // Given I have a work unit in testing status
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: ['TEST-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test feature',
            type: 'story',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Test work unit',
            children: [],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
              { state: 'testing', timestamp: new Date().toISOString() },
            ],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // When I run 'fspec update-work-unit-status TEST-001 implementing'
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'implementing',
        cwd: testDir,
      });

      // Then the status should update successfully
      expect(result.success).toBe(true);
      expect(result.newStatus).toBe('implementing');

      // And NO system-reminder about review should be displayed
      if (result.systemReminder) {
        expect(result.systemReminder).not.toContain('fspec review');
      }
    });
  });
});
