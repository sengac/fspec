/**
 * Feature: spec/features/migrate-hook-execution-to-use-execa-library.feature
 *
 * This test file validates the acceptance criteria for migrating hook execution
 * from child_process.spawn to execa library.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, chmod } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import type { HookDefinition, HookContext, HookExecutionResult } from '../types.js';
import { executeHook, executeHooks } from '../executor.js';

let testDir: string;

describe('Feature: Migrate hook execution to use execa library', () => {
  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-execa-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec/hooks'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Hook executes successfully and returns exitCode 0 with captured stdout/stderr', () => {
    it('should execute hook using execa and capture all output fields', async () => {
      // Given I have a hook script that exits with code 0
      const hookScript = join(testDir, 'spec/hooks/success.sh');
      // And the hook script writes "success message" to stdout
      // And the hook script writes "debug info" to stderr
      await writeFile(
        hookScript,
        '#!/bin/bash\necho "success message"\necho "debug info" >&2\nexit 0'
      );
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'success-hook',
        command: 'spec/hooks/success.sh',
        blocking: false,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'HOOK-010',
        event: 'pre-implementing',
        timestamp: new Date().toISOString(),
      };

      // When the hook is executed using execa
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook result should have success = true
      expect(result.success).toBe(true);

      // And the hook result should have exitCode = 0
      expect(result.exitCode).toBe(0);

      // And the hook result stdout should contain "success message"
      expect(result.stdout).toContain('success message');

      // And the hook result stderr should contain "debug info"
      expect(result.stderr).toContain('debug info');

      // And the hook result should include hookName, timedOut, and duration fields
      expect(result.hookName).toBe('success-hook');
      expect(result.timedOut).toBe(false);
      expect(result.duration).toBeGreaterThan(0);
      expect(typeof result.duration).toBe('number');
    });
  });

  describe('Scenario: Hook times out after configured timeout period and is killed', () => {
    it(
      'should use execa with AbortController to kill hook after timeout',
      async () => {
        // Given I have a hook script that runs longer than the timeout period
        const hookScript = join(testDir, 'spec/hooks/long-running.sh');
        await writeFile(
          hookScript,
          '#!/usr/bin/env node\nconst start = Date.now();\nwhile (Date.now() - start < 10000) { /* busy loop */ }'
        );
        await chmod(hookScript, 0o755);

        // And the timeout is configured to 2 seconds
        const hook: HookDefinition = {
          name: 'timeout-hook',
          command: 'spec/hooks/long-running.sh',
          blocking: false,
          timeout: 2,
        };

        const context: HookContext = {
          workUnitId: 'HOOK-010',
          event: 'pre-implementing',
          timestamp: new Date().toISOString(),
        };

        // When the hook is executed using execa with AbortController
        const startTime = Date.now();
        const result: HookExecutionResult = await executeHook(hook, context, testDir);
        const duration = Date.now() - startTime;

        // Then the hook should be killed after 2 seconds
        expect(duration).toBeGreaterThan(1800); // At least 1.8s
        expect(duration).toBeLessThan(4000); // Less than 4s

        // And the hook result should have timedOut = true
        expect(result.timedOut).toBe(true);

        // And the hook result should have exitCode = null
        expect(result.exitCode).toBe(null);

        // And the hook result should have success = false
        expect(result.success).toBe(false);
      },
      6000
    ); // 6 second test timeout
  });

  describe('Scenario: Hook fails with non-zero exit code and error message in stderr', () => {
    it('should capture failure exit code and stderr using execa', async () => {
      // Given I have a hook script that exits with code 1
      const hookScript = join(testDir, 'spec/hooks/failure.sh');
      // And the hook script writes "error occurred" to stderr
      await writeFile(
        hookScript,
        '#!/bin/bash\necho "error occurred" >&2\nexit 1'
      );
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'failure-hook',
        command: 'spec/hooks/failure.sh',
        blocking: true,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'HOOK-010',
        event: 'pre-implementing',
        timestamp: new Date().toISOString(),
      };

      // When the hook is executed using execa
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook result should have success = false
      expect(result.success).toBe(false);

      // And the hook result should have exitCode = 1
      expect(result.exitCode).toBe(1);

      // And the hook result stderr should contain "error occurred"
      expect(result.stderr).toContain('error occurred');
    });
  });

  describe('Scenario: Hook receives context via stdin as JSON string', () => {
    it('should pass context using execa input option', async () => {
      // Given I have a hook script that reads from stdin
      const hookScript = join(testDir, 'spec/hooks/read-stdin.sh');
      await writeFile(
        hookScript,
        '#!/bin/bash\nread -r input\necho "Parsed workUnitId: $(echo $input | grep -o \'"workUnitId":"[^"]*"\' | cut -d\':\' -f2 | tr -d \'"\')"\n'
      );
      await chmod(hookScript, 0o755);

      // And I have a hook context with workUnitId "HOOK-010"
      const hook: HookDefinition = {
        name: 'stdin-hook',
        command: 'spec/hooks/read-stdin.sh',
        blocking: false,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'HOOK-010',
        event: 'pre-implementing',
        timestamp: new Date().toISOString(),
      };

      // When the hook is executed using execa with input option
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook should receive the context as JSON via stdin
      // And the hook should be able to parse the workUnitId from stdin
      expect(result.stdout).toContain('HOOK-010');
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Multiple hooks execute sequentially in order', () => {
    it('should execute hooks sequentially using execa', async () => {
      // Given I have three hooks configured: "hook-a", "hook-b", and "hook-c"
      const hookAScript = join(testDir, 'spec/hooks/hook-a.sh');
      await writeFile(
        hookAScript,
        '#!/bin/bash\necho "Hook A executing"\nsleep 0.5\necho "Hook A done"'
      );
      await chmod(hookAScript, 0o755);

      const hookBScript = join(testDir, 'spec/hooks/hook-b.sh');
      await writeFile(
        hookBScript,
        '#!/bin/bash\necho "Hook B executing"\nsleep 0.5\necho "Hook B done"'
      );
      await chmod(hookBScript, 0o755);

      const hookCScript = join(testDir, 'spec/hooks/hook-c.sh');
      await writeFile(
        hookCScript,
        '#!/bin/bash\necho "Hook C executing"\nsleep 0.5\necho "Hook C done"'
      );
      await chmod(hookCScript, 0o755);

      const hooks: HookDefinition[] = [
        {
          name: 'hook-a',
          command: 'spec/hooks/hook-a.sh',
          blocking: false,
          timeout: 60,
        },
        {
          name: 'hook-b',
          command: 'spec/hooks/hook-b.sh',
          blocking: false,
          timeout: 60,
        },
        {
          name: 'hook-c',
          command: 'spec/hooks/hook-c.sh',
          blocking: false,
          timeout: 60,
        },
      ];

      const context: HookContext = {
        workUnitId: 'HOOK-010',
        event: 'pre-implementing',
        timestamp: new Date().toISOString(),
      };

      // When all hooks are executed using executeHooks function
      const startTime = Date.now();
      const results = await executeHooks(hooks, context, testDir);
      const totalDuration = Date.now() - startTime;

      // Then "hook-a" should complete before "hook-b" starts
      // And "hook-b" should complete before "hook-c" starts
      // And all hook results should be returned in execution order
      expect(results).toHaveLength(3);
      expect(results[0].hookName).toBe('hook-a');
      expect(results[0].stdout).toContain('Hook A done');
      expect(results[1].hookName).toBe('hook-b');
      expect(results[1].stdout).toContain('Hook B done');
      expect(results[2].hookName).toBe('hook-c');
      expect(results[2].stdout).toContain('Hook C done');

      // Sequential execution means total time >= sum of individual sleeps (1.5s)
      expect(totalDuration).toBeGreaterThanOrEqual(1500);

      // All hooks should succeed
      expect(results[0].success).toBe(true);
      expect(results[1].success).toBe(true);
      expect(results[2].success).toBe(true);
    });
  });
});
