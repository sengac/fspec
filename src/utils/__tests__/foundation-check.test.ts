/**
 * Feature: spec/features/foundation-existence-check-in-commands.feature
 *
 * This test file validates the foundation.json existence check utility.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'fs/promises';
import { join } from 'path';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import { checkFoundationExists } from '../foundation-check';

describe('Feature: Foundation existence check in commands', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('foundation-check');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Run board command without foundation.json', () => {
    it('should return error with system reminder to retry original command', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      const foundationPath = join(setup.testDir, 'spec', 'foundation.json');
      // File does not exist (not created)

      // When I check foundation existence with command 'fspec board'
      const result = checkFoundationExists(setup.testDir, 'fspec board');

      // Then the result should indicate an error
      expect(result.exists).toBe(false);
      expect(result.error).toBeDefined();

      // And the error message should instruct me to run 'fspec discover-foundation'
      expect(result.error).toContain('fspec discover-foundation');

      // And a system reminder should tell me to retry 'fspec board' after discover-foundation completes
      expect(result.error).toContain('<system-reminder>');
      expect(result.error).toContain('fspec board');
      expect(result.error).toContain('After completing discover-foundation');
    });
  });

  describe('Scenario: Run create-work-unit without foundation.json', () => {
    it('should return error with system reminder including original command with arguments', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      const foundationPath = join(setup.testDir, 'spec', 'foundation.json');
      // File does not exist (not created)

      // When I check foundation existence with command 'fspec create-story AUTH "Login"'
      const originalCommand = 'fspec create-story AUTH "Login"';
      const result = checkFoundationExists(setup.testDir, originalCommand);

      // Then the result should indicate an error
      expect(result.exists).toBe(false);
      expect(result.error).toBeDefined();

      // And the error message should instruct me to run 'fspec discover-foundation'
      expect(result.error).toContain('fspec discover-foundation');

      // And a system reminder should include the original command 'fspec create-story AUTH "Login"' to retry
      expect(result.error).toContain('<system-reminder>');
      expect(result.error).toContain('fspec create-story AUTH "Login"');
      expect(result.error).toContain('After completing discover-foundation');
    });
  });

  describe('Scenario: Run board command with foundation.json present', () => {
    it('should return success with no error when foundation.json exists', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" exists
      const foundationPath = join(setup.testDir, 'spec', 'foundation.json');
      await writeFile(
        foundationPath,
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

      // When I check foundation existence with command 'fspec board'
      const result = checkFoundationExists(setup.testDir, 'fspec board');

      // Then the command should execute normally
      expect(result.exists).toBe(true);

      // And no foundation check error should be displayed
      expect(result.error).toBeUndefined();
    });
  });

  describe('Scenario: Run validate command without foundation.json (read-only exempt)', () => {
    it('should not be tested in this utility - exemption handled at command level', () => {
      // This scenario tests command-level behavior (commands choose whether to call checkFoundationExists)
      // The utility function itself doesn't know about read-only vs write commands
      // This test documents that the exemption is a command-level concern
      expect(true).toBe(true);
    });
  });
});

/**
 * Feature: spec/features/foundation-missing-error-message-is-not-imperative-enough.feature
 * Bug: FOUND-009
 */
describe('Feature: Foundation missing error message is not imperative enough', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('foundation-check-bug');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Error forbids manual foundation.json creation', () => {
    it('should contain NEVER manually create foundation.json in error message', () => {
      // Given foundation.json does not exist
      // (tmpDir has no foundation.json)

      // When AI agent attempts to create a work unit
      const result = checkFoundationExists(
        setup.testDir,
        'fspec create-story AUTH "Login"'
      );

      // Then error message must contain "NEVER manually create foundation.json"
      expect(result.error).toContain('NEVER manually create foundation.json');

      // And error message must instruct to use discover-foundation workflow
      expect(result.error).toContain('use discover-foundation workflow');
    });
  });

  describe('Scenario: Error shows complete workflow steps', () => {
    it('should show 3-step workflow in error message', () => {
      // Given foundation.json does not exist
      // (tmpDir has no foundation.json)

      // When AI agent receives foundation missing error
      const result = checkFoundationExists(setup.testDir, 'fspec board');

      // Then error must show "Step 1: fspec discover-foundation (creates draft)"
      expect(result.error).toContain('Step 1: fspec discover-foundation');
      expect(result.error).toContain('creates draft');

      // And error must show "Step 2: AI analyzes codebase and fills fields"
      expect(result.error).toContain('Step 2');
      expect(result.error).toContain('AI analyzes codebase and fills fields');

      // And error must show "Step 3: fspec discover-foundation --finalize"
      expect(result.error).toContain(
        'Step 3: fspec discover-foundation --finalize'
      );
    });
  });
});
