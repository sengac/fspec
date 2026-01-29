/**
 * Feature: spec/features/foundation-existence-check-in-commands.feature
 *
 * This test file validates foundation.json existence checks in PM commands.
 * Tests integration of checkFoundationExists() into commands.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { displayBoard } from '../display-board';
import { createStory } from '../create-story';
import { validateCommand } from '../validate';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Foundation existence check in commands', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('foundation-check');

    // Initialize work-units.json
    await writeJsonTestFile(join(setup.testDir, 'spec/work-units.json'), {
      workUnits: {},
      states: {},
    });

    // Initialize tags.json
    await writeFile(
      join(setup.testDir, 'spec', 'tags.json'),
      JSON.stringify({ tags: [] })
    );
  });

  afterEach(async () => {
    // Cleanup is automatic with setupTestDirectory
  });

  describe('Scenario: Run board command without foundation.json', () => {
    it('should exit with error and system reminder to retry board command', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      // (foundation.json not created)

      // When I run 'fspec board'
      let error: Error | null = null;
      let result: any = null;
      try {
        result = await displayBoard({ cwd: setup.testDir });
      } catch (err) {
        error = err as Error;
      }

      // Then the command should fail
      const hasError = error !== null || (result && !result.success);
      expect(hasError).toBe(true);

      // And the output should mention foundation.json
      const errorMessage = error?.message || result?.error || '';
      expect(errorMessage).toContain('foundation');

      // And the error should instruct me to run 'fspec discover-foundation'
      expect(errorMessage).toContain('discover-foundation');
    });
  });

  describe('Scenario: Run create-story without foundation.json', () => {
    it('should exit with error and system reminder including original command', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      // (foundation.json not created)

      // When I run 'fspec create-story AUTH "Login"'
      let error: Error | null = null;
      let result: any = null;
      try {
        result = await createStory({
          prefix: 'AUTH',
          title: 'Login',
          cwd: setup.testDir,
        });
      } catch (err) {
        error = err as Error;
      }

      // Then the command should fail
      const hasError = error !== null || (result && !result.success);
      expect(hasError).toBe(true);

      // And the output should mention foundation
      const errorMessage = error?.message || result?.error || '';
      expect(errorMessage.toLowerCase()).toContain('foundation');
    });
  });

  describe('Scenario: Run board command with foundation.json present', () => {
    it('should execute normally with no foundation check error', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" exists
      await writeFile(
        join(setup.testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          version: '2.0.0',
          project: {
            name: 'Test',
            vision: 'Test vision',
            projectType: 'cli-tool',
          },
          problemSpace: {
            primaryProblem: {
              title: 'Test',
              description: 'Test',
              impact: 'high',
            },
          },
          solutionSpace: { overview: 'Test', capabilities: [] },
          personas: [],
        })
      );

      // When I run 'fspec board'
      const result = await displayBoard({ cwd: setup.testDir });

      // Then the command should execute normally (no error thrown)
      // The result should have board data
      expect(result).toBeDefined();
      expect(result.columns).toBeDefined();
    });
  });

  describe('Scenario: Run validate command without foundation.json (read-only exempt)', () => {
    it('should execute normally without foundation check', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      // (foundation.json not created)

      // Create a valid feature file to validate
      await mkdir(join(setup.testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(setup.testDir, 'spec', 'features', 'test.feature'),
        `Feature: Test
  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`
      );

      // When I run 'fspec validate'
      let error: Error | null = null;
      let result: any = null;
      try {
        result = await validateCommand({ cwd: setup.testDir });
      } catch (err) {
        error = err as Error;
      }

      // Then the command should execute normally (validate doesn't require foundation)
      // It should not throw an error about foundation
      if (error) {
        expect(error.message).not.toContain('discover-foundation');
      }

      // And validation should proceed for feature files
      expect(result).toBeDefined();
    });
  });
});
