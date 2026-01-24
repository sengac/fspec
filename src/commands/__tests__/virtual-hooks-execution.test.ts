/**
 * Feature: spec/features/virtual-hooks-don-t-execute-during-status-transitions.feature
 *
 * Tests for virtual hooks execution during status transitions.
 * This bug fix ensures that virtual hooks actually execute.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, readFile } from 'fs/promises';
import { join } from 'path';
import { addVirtualHook } from '../add-virtual-hook';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { showWorkUnit } from '../show-work-unit';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe("Feature: Virtual hooks don't execute during status transitions", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('virtual-hooks-test');

    // Initialize fspec project
    await writeFile(
      join(testDir, 'spec', 'work-units.json'),
      JSON.stringify({
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        states: {
          implementing: ['TEST-001'],
          backlog: [],
          specifying: [],
          testing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Test Story',
            description: 'Test',
            type: 'task', // Use task type to skip test validation
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            virtualHooks: [],
          },
        },
      }),
      'utf-8'
    );
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Blocking virtual hook fails and prevents status transition', () => {
    it('should execute blocking hook and prevent transition on failure', async () => {
      // @step Given work unit "TEST-001" is in implementing status
      // (created in beforeEach)

      // @step And work unit has a blocking virtual hook "exit 1" at pre-validating event
      await addVirtualHook({
        workUnitId: 'TEST-001',
        event: 'pre-validating',
        command: 'exit 1',
        blocking: true,
        cwd: testDir,
      });

      // @step When I run "fspec update-work-unit-status TEST-001 validating"
      let error: Error | null = null;
      let result: any = null;
      try {
        result = await updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'validating',
          cwd: testDir,
        });
      } catch (err) {
        error = err as Error;
      }

      // @step Then the virtual hook MUST execute before the transition
      // @step And the command "exit 1" should fail with exit code 1
      // @step And the transition MUST be blocked
      // Either an error is thrown or success is false
      if (error) {
        // Error message contains info about blocking hook failure
        expect(error.message.toLowerCase()).toContain('blocking');
        expect(error.message).toContain('failed');
      } else {
        expect(result.success).toBe(false);
      }

      // @step And work unit status MUST remain "implementing"
      const workUnitDetails = await showWorkUnit({
        workUnitId: 'TEST-001',
        cwd: testDir,
      });
      expect(workUnitDetails.status).toBe('implementing');
    });
  });

  describe('Scenario: Passing virtual hook allows status transition', () => {
    it('should execute passing hook and allow transition', async () => {
      // @step Given work unit "TEST-001" is in implementing status
      // (created in beforeEach)

      // @step And work unit has a blocking virtual hook "echo success" at pre-validating event
      await addVirtualHook({
        workUnitId: 'TEST-001',
        event: 'pre-validating',
        command: 'echo success',
        blocking: true,
        cwd: testDir,
      });

      // @step When I run "fspec update-work-unit-status TEST-001 validating"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'validating',
        cwd: testDir,
      });

      // @step Then the virtual hook MUST execute successfully
      // @step And the command "echo success" should succeed with exit code 0
      // (If we got here without exception, exit code was 0)
      expect(result.success).toBe(true);

      // @step And the transition MUST succeed
      expect(result.newStatus).toBe('validating');

      // @step And work unit status MUST be "validating"
      const workUnitDetails = await showWorkUnit({
        workUnitId: 'TEST-001',
        cwd: testDir,
      });
      expect(workUnitDetails.status).toBe('validating');
    });
  });
});
