/**
 * Feature: spec/features/virtual-hooks-don-t-execute-during-status-transitions.feature
 *
 * Tests for virtual hooks execution during status transitions.
 * This bug fix ensures that virtual hooks actually execute using execa.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { execSync } from 'child_process';

describe("Feature: Virtual hooks don't execute during status transitions", () => {
  let testDir: string;
  let fspecBin: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    fspecBin = join(process.cwd(), 'dist', 'index.js');

    // Initialize fspec project
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await writeFile(
      join(testDir, 'spec', 'work-units.json'),
      JSON.stringify({
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        states: {
          implementing: ['TEST-001'],
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
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Blocking virtual hook fails and prevents status transition', () => {
    it('should execute blocking hook and prevent transition on failure', async () => {
      // @step Given work unit "TEST-001" is in implementing status
      // (created in beforeEach)

      // @step And work unit has a blocking virtual hook "exit 1" at pre-validating event
      execSync(
        `node ${fspecBin} add-virtual-hook TEST-001 pre-validating "exit 1" --blocking`,
        { cwd: testDir }
      );

      // @step When I run "fspec update-work-unit-status TEST-001 validating"
      let exitCode = 0;
      let stderr = '';
      try {
        execSync(
          `node ${fspecBin} update-work-unit-status TEST-001 validating`,
          {
            cwd: testDir,
            encoding: 'utf-8',
          }
        );
      } catch (error: any) {
        exitCode = error.status;
        stderr = error.stderr;
      }

      // @step Then the virtual hook MUST execute before the transition
      // @step And the hook MUST use execa library for execution
      // @step And the command "exit 1" should fail with exit code 1
      expect(exitCode).toBe(1);

      // @step And the transition MUST be blocked
      // @step And stderr MUST contain "<system-reminder>" tags
      expect(stderr).toContain('<system-reminder>');

      // @step And stderr MUST contain "BLOCKING HOOK FAILURE"
      expect(stderr).toContain('BLOCKING HOOK FAILURE');

      // @step And work unit status MUST remain "implementing"
      const result = execSync(`node ${fspecBin} show-work-unit TEST-001`, {
        cwd: testDir,
        encoding: 'utf-8',
      });
      expect(result).toContain('Status: implementing');
    });
  });

  describe('Scenario: Passing virtual hook allows status transition', () => {
    it('should execute passing hook and allow transition', async () => {
      // @step Given work unit "TEST-001" is in implementing status
      // (created in beforeEach)

      // @step And work unit has a blocking virtual hook "echo success" at pre-validating event
      execSync(
        `node ${fspecBin} add-virtual-hook TEST-001 pre-validating "echo success" --blocking`,
        { cwd: testDir }
      );

      // @step When I run "fspec update-work-unit-status TEST-001 validating"
      const output = execSync(
        `node ${fspecBin} update-work-unit-status TEST-001 validating`,
        { cwd: testDir, encoding: 'utf-8' }
      );

      // @step Then the virtual hook MUST execute successfully
      // @step And the command "echo success" should succeed with exit code 0
      // (If we got here without exception, exit code was 0)

      // @step And the transition MUST succeed
      expect(output).toContain(
        '✓ Work unit TEST-001 status updated to validating'
      );

      // @step And work unit status MUST be "validating"
      const result = execSync(`node ${fspecBin} show-work-unit TEST-001`, {
        cwd: testDir,
        encoding: 'utf-8',
      });
      expect(result).toContain('Status: validating');
    });
  });
});
