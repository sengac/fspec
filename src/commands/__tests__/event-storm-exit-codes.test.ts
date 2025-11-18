/**
 * Feature: spec/features/event-storm-commands-exit-with-code-1-on-success.feature
 *
 * Tests for BUG-086: Event Storm commands exit with code 1 on success
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

describe('Feature: Event Storm commands exit with code 1 on success', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });

    // Initialize work units file with a work unit in specifying state
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
          workUnits: {
            'UI-001': {
              id: 'UI-001',
              type: 'story',
              title: 'Test Story',
              status: 'specifying',
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
            },
          },
          states: {
            backlog: [],
            specifying: ['UI-001'],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
            blocked: [],
          },
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: add-domain-event returns exit code 0 on success', () => {
    it('should exit with code 0 when adding domain event successfully', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-domain-event UI-001 TestEvent"
      let exitCode: number | null = null;
      let error: any = null;
      try {
        await execAsync('fspec add-domain-event UI-001 TestEvent', {
          cwd: testDir,
        });
        exitCode = 0;
      } catch (err: any) {
        exitCode = err.code || 1;
        error = err;
      }

      // @step Then the command should exit with code 0
      expect(exitCode).toBe(0);

      // @step And the domain event "TestEvent" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'TestEvent'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'event'
      );
    });
  });

  describe('Scenario: add-command returns exit code 0 on success', () => {
    it('should exit with code 0 when adding command successfully', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-command UI-001 TestCommand"
      let exitCode: number | null = null;
      try {
        await execAsync('fspec add-command UI-001 TestCommand', {
          cwd: testDir,
        });
        exitCode = 0;
      } catch (err: any) {
        exitCode = err.code || 1;
      }

      // @step Then the command should exit with code 0
      expect(exitCode).toBe(0);

      // @step And the command "TestCommand" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'TestCommand'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'command'
      );
    });
  });

  describe('Scenario: add-policy returns exit code 0 on success', () => {
    it('should exit with code 0 when adding policy successfully', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-policy UI-001 'Send email' --when UserRegistered --then SendEmail"
      let exitCode: number | null = null;
      try {
        await execAsync(
          "fspec add-policy UI-001 'Send email' --when UserRegistered --then SendEmail",
          { cwd: testDir }
        );
        exitCode = 0;
      } catch (err: any) {
        exitCode = err.code || 1;
      }

      // @step Then the command should exit with code 0
      expect(exitCode).toBe(0);

      // @step And the policy "Send email" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'Send email'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'policy'
      );
    });
  });

  describe('Scenario: add-hotspot returns exit code 0 on success', () => {
    it('should exit with code 0 when adding hotspot successfully', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-hotspot UI-001 'Email timeout' --concern 'Unclear timeout duration'"
      let exitCode: number | null = null;
      try {
        await execAsync(
          "fspec add-hotspot UI-001 'Email timeout' --concern 'Unclear timeout duration'",
          { cwd: testDir }
        );
        exitCode = 0;
      } catch (err: any) {
        exitCode = err.code || 1;
      }

      // @step Then the command should exit with code 0
      expect(exitCode).toBe(0);

      // @step And the hotspot "Email timeout" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'Email timeout'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'hotspot'
      );
    });
  });

  describe('Scenario: Command chaining with && executes all commands on success', () => {
    it('should execute both commands when chained with &&', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-domain-event UI-001 Event1 && fspec add-domain-event UI-001 Event2"
      let exitCode: number | null = null;
      try {
        await execAsync(
          'fspec add-domain-event UI-001 Event1 && fspec add-domain-event UI-001 Event2',
          { cwd: testDir, shell: '/bin/bash' }
        );
        exitCode = 0;
      } catch (err: any) {
        exitCode = err.code || 1;
      }

      // @step Then both commands should execute
      expect(exitCode).toBe(0);

      // @step And the domain event "Event1" should be added to UI-001
      // @step And the domain event "Event2" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(2);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'Event1'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[1].text).toBe(
        'Event2'
      );
    });
  });

  describe('Scenario: Event Storm command returns exit code 1 on error', () => {
    it('should exit with code 1 when work unit does not exist', async () => {
      // @step Given I have no work unit "NONEXISTENT-001"
      // (No such work unit exists in beforeEach)

      // @step When I run "fspec add-domain-event NONEXISTENT-001 Event"
      let exitCode: number | null = null;
      let stderr = '';
      try {
        await execAsync('fspec add-domain-event NONEXISTENT-001 Event', {
          cwd: testDir,
        });
        exitCode = 0;
      } catch (err: any) {
        exitCode = err.code || 1;
        stderr = err.stderr || '';
      }

      // @step Then the command should exit with code 1
      expect(exitCode).toBe(1);

      // @step And an error message should be displayed
      expect(stderr).toContain('NONEXISTENT-001');
    });
  });
});
