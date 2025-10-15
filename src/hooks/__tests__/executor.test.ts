/**
 * Feature: spec/features/hook-execution-engine.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, chmod } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import type { HookDefinition, HookContext, HookExecutionResult } from '../types.js';
import { executeHook, executeHooks } from '../executor.js';

let testDir: string;

describe('Feature: Hook execution engine', () => {
  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec/hooks'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Execute single non-blocking hook that succeeds', () => {
    it('should execute hook, capture output, and continue', async () => {
      // Given I have a hook configuration with a non-blocking hook "lint" for "post-implementing"
      const hookScript = join(testDir, 'spec/hooks/lint.sh');
      await writeFile(hookScript, '#!/bin/bash\necho "Linting complete"\nexit 0');
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'lint',
        command: 'spec/hooks/lint.sh',
        blocking: false,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'TEST-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      // When I execute the hook for event "post-implementing"
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook should be spawned as a child process
      // And the hook stdout should be captured and displayed
      expect(result.stdout).toContain('Linting complete');

      // And the hook should complete successfully
      expect(result.exitCode).toBe(0);
      expect(result.success).toBe(true);

      // And the command should continue execution
      expect(result.timedOut).toBe(false);
    });
  });

  describe('Scenario: Execute single non-blocking hook that fails', () => {
    it('should show warning but continue execution', async () => {
      // Given I have a hook configuration with a non-blocking hook "check" for "post-testing"
      const hookScript = join(testDir, 'spec/hooks/check.sh');
      await writeFile(hookScript, '#!/bin/bash\necho "Check failed"\nexit 1');
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'check',
        command: 'spec/hooks/check.sh',
        blocking: false,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'TEST-001',
        event: 'post-testing',
        timestamp: new Date().toISOString(),
      };

      // When I execute the hook for event "post-testing"
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook should be spawned as a child process
      // And a warning should be displayed about the hook failure
      expect(result.exitCode).toBe(1);
      expect(result.success).toBe(false);

      // And the command should continue execution despite the failure
      expect(result.stdout).toContain('Check failed');
    });
  });

  describe('Scenario: Execute blocking hook that fails', () => {
    it('should halt command execution on blocking hook failure', async () => {
      // Given I have a hook configuration with a blocking hook "validate" for "post-implementing"
      const hookScript = join(testDir, 'spec/hooks/validate.sh');
      await writeFile(hookScript, '#!/bin/bash\necho "Validation failed"\nexit 1');
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'validate',
        command: 'spec/hooks/validate.sh',
        blocking: true,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'TEST-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      // When I execute the hook for event "post-implementing"
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook should be spawned as a child process
      // And an error should be displayed about the hook failure
      expect(result.exitCode).toBe(1);
      expect(result.success).toBe(false);

      // And the command execution should be halted
      // And the fspec command should exit with non-zero code
      expect(result.stdout).toContain('Validation failed');
    });
  });

  describe('Scenario: Hook times out after configured duration', () => {
    it(
      'should kill hook process after timeout',
      async () => {
        // Given I have a hook configuration with a hook "slow-test" with timeout 1 second
        const hookScript = join(testDir, 'spec/hooks/slow-test.sh');
        // Use a Node.js script that properly handles signals
        await writeFile(
          hookScript,
          '#!/usr/bin/env node\nconst start = Date.now();\nwhile (Date.now() - start < 10000) { /* busy loop */ }'
        );
        await chmod(hookScript, 0o755);

        const hook: HookDefinition = {
          name: 'slow-test',
          command: 'spec/hooks/slow-test.sh',
          blocking: false,
          timeout: 1,
        };

        const context: HookContext = {
          workUnitId: 'TEST-001',
          event: 'post-testing',
          timestamp: new Date().toISOString(),
        };

        // When I execute the hook for event "post-testing"
        const startTime = Date.now();
        const result: HookExecutionResult = await executeHook(hook, context, testDir);
        const duration = Date.now() - startTime;

        // Then the hook process should be killed after ~1 second
        expect(duration).toBeGreaterThan(900); // At least 0.9s
        expect(duration).toBeLessThan(3000); // Less than 3s

        // And a timeout should be indicated
        expect(result.timedOut).toBe(true);

        // And a timeout error should be displayed
        expect(result.success).toBe(false);
      },
      5000
    ); // 5 second timeout for this test
  });

  describe('Scenario: Hook receives context via stdin', () => {
    it('should pass JSON context to hook via stdin', async () => {
      // Given I have a hook configuration with a hook "notify" for "post-implementing"
      const hookScript = join(testDir, 'spec/hooks/notify.sh');
      await writeFile(
        hookScript,
        '#!/bin/bash\nread -r input\necho "Received: $input"'
      );
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'notify',
        command: 'spec/hooks/notify.sh',
        blocking: false,
        timeout: 60,
      };

      // And I am executing in context of work unit "AUTH-001"
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: '2025-01-15T10:30:00Z',
      };

      // When I execute the hook for event "post-implementing"
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the hook should receive JSON context via stdin
      // And the context should include "workUnitId" field with value "AUTH-001"
      expect(result.stdout).toContain('AUTH-001');

      // And the context should include "event" field with value "post-implementing"
      expect(result.stdout).toContain('post-implementing');

      // And the context should include "timestamp" field with ISO 8601 format
      expect(result.stdout).toContain('2025-01-15T10:30:00Z');
    });
  });

  describe('Scenario: Multiple hooks execute sequentially', () => {
    it('should execute hooks in order, one after another', async () => {
      // Given I have a hook configuration with hooks "first" and "second" for "post-implementing"
      const firstScript = join(testDir, 'spec/hooks/first.sh');
      await writeFile(firstScript, '#!/bin/bash\necho "First hook"\nsleep 1');
      await chmod(firstScript, 0o755);

      const secondScript = join(testDir, 'spec/hooks/second.sh');
      await writeFile(secondScript, '#!/bin/bash\necho "Second hook"');
      await chmod(secondScript, 0o755);

      const hooks: HookDefinition[] = [
        {
          name: 'first',
          command: 'spec/hooks/first.sh',
          blocking: false,
          timeout: 60,
        },
        {
          name: 'second',
          command: 'spec/hooks/second.sh',
          blocking: false,
          timeout: 60,
        },
      ];

      const context: HookContext = {
        workUnitId: 'TEST-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      // When I execute the hooks for event "post-implementing"
      const startTime = Date.now();
      const results = await executeHooks(hooks, context, testDir);
      const duration = Date.now() - startTime;

      // Then hook "first" should start execution
      // And hook "first" should complete before hook "second" starts
      // And hook "second" should start execution after hook "first" completes
      expect(results).toHaveLength(2);
      expect(results[0].stdout).toContain('First hook');
      expect(results[1].stdout).toContain('Second hook');

      // And hooks should execute in sequential order
      // First hook sleeps for 1s, so total time should be at least 1s
      expect(duration).toBeGreaterThanOrEqual(1000);
    });
  });

  describe('Scenario: Hook stdout and stderr are captured and displayed', () => {
    it('should capture both stdout and stderr', async () => {
      // Given I have a hook configuration with a hook "test" for "post-testing"
      const hookScript = join(testDir, 'spec/hooks/test.sh');
      await writeFile(
        hookScript,
        '#!/bin/bash\necho "Running tests..."\necho "Warning: deprecated API" >&2'
      );
      await chmod(hookScript, 0o755);

      const hook: HookDefinition = {
        name: 'test',
        command: 'spec/hooks/test.sh',
        blocking: false,
        timeout: 60,
      };

      const context: HookContext = {
        workUnitId: 'TEST-001',
        event: 'post-testing',
        timestamp: new Date().toISOString(),
      };

      // When I execute the hook for event "post-testing"
      const result: HookExecutionResult = await executeHook(hook, context, testDir);

      // Then the stdout message "Running tests..." should be captured
      expect(result.stdout).toContain('Running tests...');

      // And the stderr message "Warning: deprecated API" should be captured
      expect(result.stderr).toContain('Warning: deprecated API');

      // And both stdout and stderr should be displayed to the user
      expect(result.success).toBe(true);
    });
  });
});
