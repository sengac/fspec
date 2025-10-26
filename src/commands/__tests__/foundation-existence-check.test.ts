/**
 * Feature: spec/features/foundation-existence-check-in-commands.feature
 *
 * This test file validates foundation.json existence checks in PM commands.
 * Tests integration of checkFoundationExists() into commands.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { execa } from 'execa';

describe('Feature: Foundation existence check in commands', () => {
  let tmpDir: string;
  const fspecBin = join(process.cwd(), 'dist', 'index.js');

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(tmpDir, 'spec'), { recursive: true });
    await mkdir(join(tmpDir, 'spec', 'features'), { recursive: true });

    // Initialize work-units.json
    await writeFile(
      join(tmpDir, 'spec', 'work-units.json'),
      JSON.stringify({ workUnits: {}, states: {} })
    );

    // Initialize tags.json
    await writeFile(
      join(tmpDir, 'spec', 'tags.json'),
      JSON.stringify({ tags: [] })
    );
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Run board command without foundation.json', () => {
    it('should exit with error and system reminder to retry board command', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      // (foundation.json not created)

      // When I run 'fspec board'
      const result = await execa(fspecBin, ['board'], {
        cwd: tmpDir,
        reject: false,
      });

      // Then the command should exit with code 1
      expect(result.exitCode).toBe(1);

      // And the output should display an error message
      expect(result.stderr || result.stdout).toContain('foundation.json');

      // And the error message should instruct me to run 'fspec discover-foundation'
      expect(result.stderr || result.stdout).toContain(
        'fspec discover-foundation'
      );

      // And a system reminder should tell me to retry 'fspec board' after discover-foundation completes
      expect(result.stderr || result.stdout).toContain('<system-reminder>');
      expect(result.stderr || result.stdout).toContain('fspec board');
    });
  });

  describe('Scenario: Run create-story without foundation.json', () => {
    it('should exit with error and system reminder including original command', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      // (foundation.json not created)

      // When I run 'fspec create-story AUTH "Login"'
      const result = await execa(fspecBin, ['create-story', 'AUTH', 'Login'], {
        cwd: tmpDir,
        reject: false,
      });

      // Then the command should exit with code 1
      expect(result.exitCode).toBe(1);

      // And the output should display an error message
      expect(result.stderr || result.stdout).toContain('foundation.json');

      // And the error message should instruct me to run 'fspec discover-foundation'
      expect(result.stderr || result.stdout).toContain(
        'fspec discover-foundation'
      );

      // And a system reminder should include the original command to retry
      expect(result.stderr || result.stdout).toContain('<system-reminder>');
      expect(result.stderr || result.stdout).toContain('create-story');
      expect(result.stderr || result.stdout).toContain('AUTH');
      expect(result.stderr || result.stdout).toContain('Login');
    });
  });

  describe('Scenario: Run board command with foundation.json present', () => {
    it('should execute normally with no foundation check error', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" exists
      await writeFile(
        join(tmpDir, 'spec', 'foundation.json'),
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
      const result = await execa(fspecBin, ['board'], {
        cwd: tmpDir,
        reject: false,
      });

      // Then the command should execute normally
      expect(result.exitCode).toBe(0);

      // And no foundation check error should be displayed
      expect(result.stderr || result.stdout).not.toContain(
        'fspec discover-foundation'
      );
      expect(result.stderr || result.stdout).not.toContain(
        'foundation.json does not exist'
      );
    });
  });

  describe('Scenario: Run validate command without foundation.json (read-only exempt)', () => {
    it('should execute normally without foundation check', async () => {
      // Given I am in a project directory
      // And the file "spec/foundation.json" does not exist
      // (foundation.json not created)

      // Create a valid feature file to validate
      await writeFile(
        join(tmpDir, 'spec', 'features', 'test.feature'),
        `Feature: Test
  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`
      );

      // When I run 'fspec validate'
      const result = await execa(fspecBin, ['validate'], {
        cwd: tmpDir,
        reject: false,
      });

      // Then the command should execute normally
      expect(result.exitCode).toBe(0);

      // And no foundation check error should be displayed
      expect(result.stderr || result.stdout).not.toContain(
        'fspec discover-foundation'
      );

      // And validation should proceed for feature files
      expect(result.stdout).toContain('valid');
    });
  });
});
