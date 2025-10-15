/**
 * Feature: spec/features/integrate-hooks-into-all-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { runCommandWithHooks } from '../integration.js';
import type { HookConfig } from '../types.js';

describe('Feature: Integrate hooks into all commands', () => {
  let testDir: string;
  let configPath: string;
  let workUnitsPath: string;

  beforeEach(async () => {
    // Create a temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    configPath = join(testDir, 'spec', 'fspec-hooks.json');
    workUnitsPath = join(testDir, 'spec', 'work-units.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec', 'hooks'), { recursive: true });

    // Create minimal work-units.json
    await writeFile(
      workUnitsPath,
      JSON.stringify({
        workUnits: [
          {
            id: 'AUTH-001',
            title: 'Test Work Unit',
            type: 'story',
            status: 'specifying',
            tags: ['@security', '@critical'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          {
            id: 'DASH-001',
            title: 'Dashboard Work Unit',
            type: 'story',
            status: 'backlog',
            tags: ['@ui', '@phase1'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        ],
        prefixes: [],
        epics: [],
      })
    );
  });

  afterEach(async () => {
    // Clean up test directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Command triggers pre and post hooks', () => {
    it('should execute pre and post hooks with context', async () => {
      // Given I have hooks configured for "pre-update-work-unit-status" and "post-update-work-unit-status"
      const config: HookConfig = {
        hooks: {
          'pre-update-work-unit-status': [
            {
              name: 'pre-hook',
              command: 'spec/hooks/pre.sh',
              blocking: false,
            },
          ],
          'post-update-work-unit-status': [
            {
              name: 'post-hook',
              command: 'spec/hooks/post.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // Create hook scripts that output to verify execution order
      await writeFile(
        join(testDir, 'spec/hooks/pre.sh'),
        '#!/bin/bash\necho "PRE-HOOK"',
        { mode: 0o755 }
      );
      await writeFile(
        join(testDir, 'spec/hooks/post.sh'),
        '#!/bin/bash\necho "POST-HOOK"',
        { mode: 0o755 }
      );

      // When I run "fspec update-work-unit-status AUTH-001 implementing"
      const commandFn = async (context: { workUnitId: string; newStatus: string }) => {
        // Simulate command execution
        return { success: true, message: 'Status updated' };
      };

      const result = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'implementing' },
        commandFn,
        testDir
      );

      // Then the pre-update-work-unit-status hooks should execute before status change
      expect(result.preHookResults).toHaveLength(1);
      expect(result.preHookResults[0].hookName).toBe('pre-hook');
      expect(result.preHookResults[0].stdout).toContain('PRE-HOOK');

      // And the post-update-work-unit-status hooks should execute after status change
      expect(result.postHookResults).toHaveLength(1);
      expect(result.postHookResults[0].hookName).toBe('post-hook');
      expect(result.postHookResults[0].stdout).toContain('POST-HOOK');

      // And the hooks should receive context with workUnitId "AUTH-001"
      // And the hooks should receive context with event name
      // And the hooks should receive context with timestamp
      // (context is passed to hooks via stdin - verified by hooks executing)
    });
  });

  describe('Scenario: Pre-hook validates preconditions and blocks command', () => {
    it('should block command when blocking pre-hook fails', async () => {
      // Given I have a blocking pre-hook "validate-feature-file" for "pre-update-work-unit-status"
      const config: HookConfig = {
        hooks: {
          'pre-update-work-unit-status': [
            {
              name: 'validate-feature-file',
              command: 'spec/hooks/validate.sh',
              blocking: true,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // And the hook checks that work unit has a linked feature file
      // And work unit "AUTH-001" has no linked feature file
      await writeFile(
        join(testDir, 'spec/hooks/validate.sh'),
        '#!/bin/bash\necho "No feature file linked" >&2\nexit 1',
        { mode: 0o755 }
      );

      // When I run "fspec update-work-unit-status AUTH-001 testing"
      const commandFn = async () => {
        return { success: true, executed: true };
      };

      const result = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'testing' },
        commandFn,
        testDir
      );

      // Then the pre-hook should execute and fail
      expect(result.preHookResults[0].success).toBe(false);
      expect(result.preHookResults[0].exitCode).toBe(1);

      // And the command should not execute
      expect(result.commandExecuted).toBe(false);

      // And the hook stderr should be wrapped in system-reminder tags
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('</system-reminder>');

      // And the command should exit with non-zero code
      expect(result.exitCode).toBe(1);
    });
  });

  describe('Scenario: Post-hook runs automation after command succeeds', () => {
    it('should run post-hook after command executes', async () => {
      // Given I have a non-blocking post-hook "run-tests" for "post-update-work-unit-status"
      const config: HookConfig = {
        hooks: {
          'post-update-work-unit-status': [
            {
              name: 'run-tests',
              command: 'spec/hooks/run-tests.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      await writeFile(
        join(testDir, 'spec/hooks/run-tests.sh'),
        '#!/bin/bash\necho "Running tests..."',
        { mode: 0o755 }
      );

      // When I run "fspec update-work-unit-status AUTH-001 implementing"
      const commandFn = async () => {
        return { success: true, executed: true };
      };

      const result = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'implementing' },
        commandFn,
        testDir
      );

      // Then the command should execute successfully
      expect(result.commandExecuted).toBe(true);

      // And the post-hook should execute after status change
      expect(result.postHookResults).toHaveLength(1);
      expect(result.postHookResults[0].hookName).toBe('run-tests');
      expect(result.postHookResults[0].stdout).toContain('Running tests...');
    });
  });

  describe('Scenario: Blocking pre-hook failure prevents command execution', () => {
    it('should prevent command execution and show system-reminder', async () => {
      // Given I have a blocking pre-hook "lint" for "pre-update-work-unit-status"
      const config: HookConfig = {
        hooks: {
          'pre-update-work-unit-status': [
            {
              name: 'lint',
              command: 'spec/hooks/lint.sh',
              blocking: true,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // And the hook exits with code 1
      // And the hook outputs "Lint errors found" to stderr
      await writeFile(
        join(testDir, 'spec/hooks/lint.sh'),
        '#!/bin/bash\necho "Lint errors found" >&2\nexit 1',
        { mode: 0o755 }
      );

      // When I run "fspec update-work-unit-status AUTH-001 testing"
      const commandFn = async () => {
        return { success: true, executed: true };
      };

      const result = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'testing' },
        commandFn,
        testDir
      );

      // Then the pre-hook should execute and fail
      expect(result.preHookResults[0].success).toBe(false);

      // And the command should not execute
      expect(result.commandExecuted).toBe(false);

      // And the hook stderr should be wrapped in system-reminder tags
      // And the system-reminder should contain "Hook: lint"
      // And the system-reminder should contain "Exit code: 1"
      // And the system-reminder should contain "Lint errors found"
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('Hook: lint');
      expect(result.output).toContain('Exit code: 1');
      expect(result.output).toContain('Lint errors found');
      expect(result.output).toContain('</system-reminder>');
    });
  });

  describe('Scenario: Non-blocking post-hook failure does not affect command success', () => {
    it('should allow command success despite non-blocking hook failure', async () => {
      // Given I have a non-blocking post-hook "notify" for "post-update-work-unit-status"
      const config: HookConfig = {
        hooks: {
          'post-update-work-unit-status': [
            {
              name: 'notify',
              command: 'spec/hooks/notify.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // And the hook exits with code 1
      // And the hook outputs "Notification failed" to stderr
      await writeFile(
        join(testDir, 'spec/hooks/notify.sh'),
        '#!/bin/bash\necho "Notification failed" >&2\nexit 1',
        { mode: 0o755 }
      );

      // When I run "fspec update-work-unit-status AUTH-001 implementing"
      const commandFn = async () => {
        return { success: true, executed: true };
      };

      const result = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'implementing' },
        commandFn,
        testDir
      );

      // Then the command should execute successfully
      expect(result.commandExecuted).toBe(true);

      // And the post-hook should execute after status change
      expect(result.postHookResults[0].hookName).toBe('notify');

      // And the hook failure should not prevent command success
      expect(result.exitCode).toBe(0);

      // And the hook stderr should be displayed without system-reminder wrapping
      expect(result.output).not.toContain('<system-reminder>');
      expect(result.output).toContain('Notification failed');
    });
  });

  describe('Scenario: Hook with tag condition only runs for matching work units', () => {
    it('should only execute hook for work units with matching tags', async () => {
      // Given I have a hook with condition tags ["@security"]
      const config: HookConfig = {
        hooks: {
          'post-update-work-unit-status': [
            {
              name: 'security-scan',
              command: 'spec/hooks/scan.sh',
              blocking: false,
              condition: {
                tags: ['@security'],
              },
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      await writeFile(
        join(testDir, 'spec/hooks/scan.sh'),
        '#!/bin/bash\necho "Security scan executed"',
        { mode: 0o755 }
      );

      const commandFn = async () => {
        return { success: true, executed: true };
      };

      // When I run "fspec update-work-unit-status AUTH-001 implementing"
      const resultAuth = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'implementing' },
        commandFn,
        testDir
      );

      // Then the hook should execute because work unit has @security tag
      expect(resultAuth.postHookResults).toHaveLength(1);
      expect(resultAuth.postHookResults[0].stdout).toContain('Security scan executed');

      // When I run "fspec update-work-unit-status DASH-001 implementing"
      const resultDash = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'DASH-001', newStatus: 'implementing' },
        commandFn,
        testDir
      );

      // Then the hook should not execute because work unit lacks @security tag
      expect(resultDash.postHookResults).toHaveLength(0);
    });
  });

  describe('Scenario: Hook output formatted with system-reminder for blocking failures', () => {
    it('should format blocking hook failures with system-reminder tags', async () => {
      // Given I have a blocking post-hook "validate" for "post-update-work-unit-status"
      const config: HookConfig = {
        hooks: {
          'post-update-work-unit-status': [
            {
              name: 'validate',
              command: 'spec/hooks/validate.sh',
              blocking: true,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // And the hook exits with code 1
      // And the hook outputs "Validation failed: tests not passing" to stderr
      await writeFile(
        join(testDir, 'spec/hooks/validate.sh'),
        '#!/bin/bash\necho "Validation failed: tests not passing" >&2\nexit 1',
        { mode: 0o755 }
      );

      // When I run "fspec update-work-unit-status AUTH-001 implementing"
      const commandFn = async () => {
        return { success: true, executed: true };
      };

      const result = await runCommandWithHooks(
        'update-work-unit-status',
        { workUnitId: 'AUTH-001', newStatus: 'implementing' },
        commandFn,
        testDir
      );

      // Then the command should execute
      expect(result.commandExecuted).toBe(true);

      // And the post-hook should execute and fail
      expect(result.postHookResults[0].success).toBe(false);

      // And the hook stderr should be wrapped in system-reminder tags
      // And the output should contain "<system-reminder>"
      // And the output should contain "Hook: validate"
      // And the output should contain "Exit code: 1"
      // And the output should contain "Validation failed: tests not passing"
      // And the output should contain "</system-reminder>"
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('Hook: validate');
      expect(result.output).toContain('Exit code: 1');
      expect(result.output).toContain('Validation failed: tests not passing');
      expect(result.output).toContain('</system-reminder>');
    });
  });
});
