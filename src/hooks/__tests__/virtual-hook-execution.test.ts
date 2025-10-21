/**
 * Feature: spec/features/work-unit-scoped-hooks-for-dynamic-validation.feature
 *
 * This test file validates virtual hook execution integration with the existing hook system.
 * Tests map to scenarios: blocking hooks, execution order, git context.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { runCommandWithHooks } from '../../hooks/integration';
import { showWorkUnit } from '../../commands/show-work-unit';

describe('Feature: Virtual hook execution integration', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-virtual-exec-test-'));
    await mkdir(join(testDir, 'spec', 'hooks', '.virtual'), {
      recursive: true,
    });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Blocking virtual hook prevents workflow transition until fixed', () => {
    it('should block transition when virtual hook fails', async () => {
      // Given: Work unit AUTH-001 has blocking virtual hook
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'failing-hook',
                event: 'post-test-command',
                command: 'sh -c "exit 1"', // Command that fails
                blocking: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: Hook executes and fails (returns non-zero exit code)
      const result = await runCommandWithHooks(
        'test-command',
        { workUnitId: 'AUTH-001' },
        async () => {
          return { success: true };
        },
        testDir
      );

      // Then: Command should execute but exit code should be 1 (hook failure)
      expect(result.commandExecuted).toBe(true);
      expect(result.exitCode).toBe(1);
      expect(result.postHookResults[0].success).toBe(false);

      // And: Error output should be wrapped in <system-reminder> tags
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('</system-reminder>');
    });

    it('should allow transition when virtual hook passes', async () => {
      // Given: Work unit has blocking virtual hook that succeeds
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'passing-hook',
                event: 'post-test-command',
                command: 'echo "Hook passed"', // Command that succeeds
                blocking: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create empty hooks config to ensure hook system is initialized
      await writeFile(
        join(testDir, 'spec', 'fspec-hooks.json'),
        JSON.stringify({ hooks: {} }, null, 2)
      );

      // When: Hook executes and succeeds (returns exit code 0)
      const result = await runCommandWithHooks(
        'test-command',
        { workUnitId: 'AUTH-001' },
        async () => {
          return { success: true };
        },
        testDir
      );

      // Then: Command should execute and hook should succeed
      expect(result.commandExecuted).toBe(true);
      expect(result.postHookResults[0].success).toBe(true);

      // Exit code should be 0 when blocking hook succeeds
      if (result.postHookResults[0].success) {
        expect(result.exitCode).toBe(0);
      }
    });
  });

  describe('Scenario: Virtual hooks run BEFORE global hooks', () => {
    it('should execute virtual hooks before global hooks for same event', async () => {
      // Given: Work unit has virtual hook at post-implementing
      // And: Global hook exists for post-implementing
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'virtual-eslint',
                event: 'post-test-command',
                command: 'echo "virtual hook executed"',
                blocking: false,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const globalHooksConfig = {
        hooks: {
          'post-test-command': [
            {
              name: 'global-lint',
              command: 'echo "global hook executed"',
              blocking: false,
            },
          ],
        },
      };

      await writeFile(
        join(testDir, 'spec', 'fspec-hooks.json'),
        JSON.stringify(globalHooksConfig, null, 2)
      );

      // When: Hooks execute for test-command
      // (runCommandWithHooks generates post-test-command event)
      const result = await runCommandWithHooks(
        'test-command',
        { workUnitId: 'AUTH-001' },
        async () => {
          return { success: true };
        },
        testDir
      );

      // Then: Both hooks should execute
      expect(result.postHookResults).toHaveLength(2);
      expect(result.postHookResults[0].success).toBe(true);
      expect(result.postHookResults[1].success).toBe(true);

      // And: Virtual hook output should appear first (index 0)
      expect(result.postHookResults[0].stdout).toContain(
        'virtual hook executed'
      );
      expect(result.postHookResults[1].stdout).toContain(
        'global hook executed'
      );
    });
  });

  describe('Scenario: Handle virtual hook execution failure gracefully', () => {
    it('should detect command not found errors', async () => {
      // Given: Virtual hook with command that doesn't exist
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'nonexistent',
                event: 'post-test-command',
                command: 'nonexistent-command-that-does-not-exist',
                blocking: false,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: Hook attempts to execute
      const result = await runCommandWithHooks(
        'test-command',
        { workUnitId: 'AUTH-001' },
        async () => {
          return { success: true };
        },
        testDir
      );

      // Then: Should return execution failure
      expect(result.postHookResults[0].success).toBe(false);

      // And: Error message should indicate command not found
      const errorOutput =
        result.postHookResults[0].stderr ||
        result.postHookResults[0].stdout ||
        '';
      expect(errorOutput.toLowerCase()).toMatch(
        /command not found|enoent|not found/
      );
    });

    it('should wrap blocking hook errors in <system-reminder> tags', async () => {
      // Given: Blocking virtual hook that fails
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'failing-hook',
                event: 'post-test-command',
                command: 'sh -c "echo Hook failed >&2 && exit 1"',
                blocking: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: Hook executes and fails
      const result = await runCommandWithHooks(
        'test-command',
        { workUnitId: 'AUTH-001' },
        async () => {
          return { success: true };
        },
        testDir
      );

      // Then: stderr should be wrapped in <system-reminder> tags
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('</system-reminder>');
      expect(result.output).toContain('Hook failed');
    });
  });

  describe('Scenario: Create virtual hook with git context for changed files only', () => {
    it('should store gitContext flag when virtual hook is created', async () => {
      // Given: Work unit exists
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'eslint-changed',
                event: 'post-test-command',
                command: 'eslint',
                blocking: true,
                gitContext: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: show-work-unit is called
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then: Virtual hook should have gitContext flag
      expect(result.virtualHooks).toBeDefined();
      expect(result.virtualHooks![0].gitContext).toBe(true);
    });

    it('should execute virtual hook with gitContext flag (placeholder for git provider)', async () => {
      // Given: Virtual hook with gitContext: true
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'echo-context',
                event: 'post-test-command',
                command: 'echo "Git context would be provided here"',
                blocking: false,
                gitContext: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create empty hooks config
      await writeFile(
        join(testDir, 'spec', 'fspec-hooks.json'),
        JSON.stringify({ hooks: {} }, null, 2)
      );

      // When: Hook executes
      const result = await runCommandWithHooks(
        'test-command',
        { workUnitId: 'AUTH-001' },
        async () => {
          return { success: true };
        },
        testDir
      );

      // Then: Hook should execute successfully
      // Note: Git context provider not yet implemented - hook runs without context for now
      expect(result.postHookResults[0].success).toBe(true);
      expect(result.postHookResults[0].stdout).toContain(
        'Git context would be provided here'
      );
    });
  });

  describe('Scenario: Display virtual hooks in work unit details', () => {
    it('should include Virtual Hooks section in show-work-unit output', async () => {
      // Given: Work unit HOOK-011 has virtual hooks
      const workUnitsData = {
        workUnits: {
          'HOOK-011': {
            id: 'HOOK-011',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'eslint',
                event: 'post-implementing',
                command: 'eslint src/',
                blocking: true,
              },
              {
                name: 'prettier',
                event: 'post-implementing',
                command: 'prettier --check src/',
                blocking: false,
              },
              {
                name: 'test-coverage',
                event: 'post-validating',
                command: 'npm run test:coverage',
                blocking: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: User runs 'fspec show-work-unit HOOK-011'
      const result = await showWorkUnit({
        workUnitId: 'HOOK-011',
        cwd: testDir,
      });

      // Then: Result should include virtual hooks
      expect(result.virtualHooks).toBeDefined();
      expect(result.virtualHooks).toHaveLength(3);

      // And: Hooks should have correct properties
      expect(result.virtualHooks![0].name).toBe('eslint');
      expect(result.virtualHooks![0].blocking).toBe(true);
      expect(result.virtualHooks![1].name).toBe('prettier');
      expect(result.virtualHooks![1].blocking).toBe(false);
      expect(result.virtualHooks![2].name).toBe('test-coverage');
      expect(result.virtualHooks![2].blocking).toBe(true);
    });
  });
});
