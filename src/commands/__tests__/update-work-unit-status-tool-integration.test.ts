/**
 * Feature: spec/features/tool-detection-check-functions-not-integrated-into-workflow.feature
 *
 * This test file validates that checkTestCommand and checkQualityCommands
 * are called during status transitions to 'validating' phase.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mkdirSync, writeFileSync, rmSync, existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

describe('Feature: Tool detection check functions not integrated into workflow', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
    mkdirSync(join(testDir, 'spec'), { recursive: true });

    // Create minimal work-units.json
    const workUnitsPath = join(testDir, 'spec', 'work-units.json');
    writeFileSync(
      workUnitsPath,
      JSON.stringify({
        prefixes: { AUTH: { description: 'Auth' } },
        epics: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            prefix: 'AUTH',
            number: 1,
            title: 'Test',
            description: 'Test',
            status: 'implementing',
            type: 'story',
            dependencies: {
              dependsOn: [],
              blocks: [],
              blockedBy: [],
              relatesTo: [],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'implementing',
                enteredAt: new Date().toISOString(),
              },
            ],
          },
        },
      })
    );
  });

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Emit system-reminder when moving to validating with no config', () => {
    it('should call checkTestCommand and emit system-reminder', async () => {
      // Given spec/fspec-config.json does not have tools.test.command configured
      // (No config file exists)

      // Spy on console.log to capture system-reminder output
      const consoleLogSpy = vi
        .spyOn(console, 'log')
        .mockImplementation(() => {});

      // When AI runs 'fspec update-work-unit-status AUTH-001 validating'
      const { updateWorkUnitStatus } = await import(
        '../update-work-unit-status.js'
      );
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'validating',
        cwd: testDir,
      });

      // Then update-work-unit-status command should call checkTestCommand function
      // And system-reminder should be emitted to console output
      // And system-reminder should contain text: 'NO TEST COMMAND CONFIGURED'
      // And system-reminder should tell AI to run: fspec configure-tools --test-command <cmd>

      // Check that console.log was called with system-reminder
      const calls = consoleLogSpy.mock.calls.flat().join('\n');
      expect(calls).toContain('NO TEST COMMAND CONFIGURED');
      expect(calls).toContain('fspec configure-tools --test-command');

      consoleLogSpy.mockRestore();
    });
  });

  describe('Scenario: Emit system-reminder when moving to validating with config present', () => {
    it('should call checkTestCommand and emit configured command', async () => {
      // Given spec/fspec-config.json has tools.test.command = 'npm test'
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(
        configPath,
        JSON.stringify({
          agent: 'claude',
          tools: {
            test: {
              command: 'npm test',
            },
          },
        })
      );

      // Spy on console.log to capture system-reminder output
      const consoleLogSpy = vi
        .spyOn(console, 'log')
        .mockImplementation(() => {});

      // When AI runs 'fspec update-work-unit-status AUTH-001 validating'
      const { updateWorkUnitStatus } = await import(
        '../update-work-unit-status.js'
      );
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'validating',
        cwd: testDir,
      });

      // Then update-work-unit-status command should call checkTestCommand function
      // And system-reminder should be emitted to console output
      // And system-reminder should contain text: 'RUN TESTS'
      // And system-reminder should contain text: 'Run tests: npm test'

      const calls = consoleLogSpy.mock.calls.flat().join('\n');
      expect(calls).toContain('RUN TESTS');
      expect(calls).toContain('Run tests: npm test');

      consoleLogSpy.mockRestore();
    });
  });

  describe('Scenario: Emit quality check reminder when moving to validating', () => {
    it('should call checkQualityCommands and emit chained commands', async () => {
      // Given spec/fspec-config.json has tools.qualityCheck.commands = ['eslint .', 'prettier --check .']
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      writeFileSync(
        configPath,
        JSON.stringify({
          agent: 'claude',
          tools: {
            qualityCheck: {
              commands: ['eslint .', 'prettier --check .'],
            },
          },
        })
      );

      // Spy on console.log to capture system-reminder output
      const consoleLogSpy = vi
        .spyOn(console, 'log')
        .mockImplementation(() => {});

      // When AI runs 'fspec update-work-unit-status AUTH-001 validating'
      const { updateWorkUnitStatus } = await import(
        '../update-work-unit-status.js'
      );
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'validating',
        cwd: testDir,
      });

      // Then update-work-unit-status command should call checkQualityCommands function
      // And system-reminder should be emitted to console output
      // And system-reminder should contain text: 'RUN QUALITY CHECKS'
      // And system-reminder should contain chained command: 'eslint . && prettier --check .'

      const calls = consoleLogSpy.mock.calls.flat().join('\n');
      expect(calls).toContain('RUN QUALITY CHECKS');
      expect(calls).toContain('eslint . && prettier --check .');

      consoleLogSpy.mockRestore();
    });
  });
});
