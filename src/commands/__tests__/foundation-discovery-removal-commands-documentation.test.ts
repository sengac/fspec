/**
 * Feature: spec/features/foundation-discovery-removal-commands-documentation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { exec } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import { fileURLToPath } from 'url';
import { discoverFoundation } from '../discover-foundation';
import { mkdir, writeFile, rm } from 'fs/promises';

const execAsync = promisify(exec);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const CLI_PATH = path.resolve(__dirname, '../../../dist/index.js');

interface CommandOutput {
  stdout: string;
  stderr: string;
  exitCode: number;
}

async function runCommand(command: string): Promise<CommandOutput> {
  try {
    const { stdout, stderr } = await execAsync(command);
    return { stdout, stderr, exitCode: 0 };
  } catch (error) {
    const execError = error as {
      stdout?: string;
      stderr?: string;
      code?: number;
    };
    return {
      stdout: execError.stdout || '',
      stderr: execError.stderr || '',
      exitCode: execError.code || 1,
    };
  }
}

describe('Feature: Foundation discovery removal commands documentation', () => {
  describe('Scenario: Help setup section includes remove-persona command', () => {
    it('should include remove-persona in help setup output', async () => {
      // @step Given I am using fspec
      // (No setup needed - fspec is available)

      // @step When I run "fspec help setup"
      const result = await runCommand(`node ${CLI_PATH} help setup`);

      // @step Then the output should include "fspec remove-persona <name>"
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain('fspec remove-persona <name>');

      // @step And the output should include "Description: Remove persona from foundation.json"
      expect(result.stdout).toContain('Description: Remove persona from foundation.json');

      // @step And the output should include example usage for removing personas
      expect(result.stdout).toMatch(/example.*remove-persona/is);

      // @step And the remove-persona section should appear alongside add-persona
      expect(result.stdout).toContain('fspec add-persona');
      expect(result.stdout).toContain('fspec remove-persona');
    });
  });

  describe('Scenario: Help setup section includes remove-capability command', () => {
    it('should include remove-capability in help setup output', async () => {
      // @step Given I am using fspec
      // (No setup needed - fspec is available)

      // @step When I run "fspec help setup"
      const result = await runCommand(`node ${CLI_PATH} help setup`);

      // @step Then the output should include "fspec remove-capability <name>"
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain('fspec remove-capability <name>');

      // @step And the output should include "Description: Remove capability from foundation.json"
      expect(result.stdout).toContain('Description: Remove capability from foundation.json');

      // @step And the output should include example usage for removing capabilities
      expect(result.stdout).toMatch(/example.*remove-capability/is);

      // @step And the remove-capability section should appear alongside add-capability
      expect(result.stdout).toContain('fspec add-capability');
      expect(result.stdout).toContain('fspec remove-capability');
    });
  });

  describe('Scenario: Removal commands show placeholder removal examples', () => {
    it('should show examples of removing placeholder text', async () => {
      // @step Given I am using fspec
      // (No setup needed - fspec is available)

      // @step When I run "fspec help setup"
      const result = await runCommand(`node ${CLI_PATH} help setup`);

      // @step Then the remove-persona examples should include removing placeholder text
      expect(result.exitCode).toBe(0);

      // @step And the output should show 'fspec remove-persona "[QUESTION: Who uses this?]"'
      expect(result.stdout).toContain('fspec remove-persona "[QUESTION: Who uses this?]"');

      // @step And the remove-capability examples should include removing placeholder text
      // @step And the output should show 'fspec remove-capability "[QUESTION: What can users DO?]"'
      expect(result.stdout).toContain('fspec remove-capability "[QUESTION: What can users DO?]"');
    });
  });

  describe('Scenario: discover-foundation error includes removal guidance for placeholders', () => {
    let testDir: string;
    let draftPath: string;
    let finalPath: string;

    beforeEach(async () => {
      // Create temp directory for tests
      testDir = path.join(process.cwd(), `test-temp-${Date.now()}`);
      draftPath = path.join(testDir, 'spec/foundation.json.draft');
      finalPath = path.join(testDir, 'spec/foundation.json');
      await mkdir(path.join(testDir, 'spec'), { recursive: true });
    });

    afterEach(async () => {
      // Clean up
      await rm(testDir, { recursive: true, force: true });
    });

    it('should include removal guidance in error message', async () => {
      // @step Given I have a foundation draft with placeholder personas
      // @step And the draft contains '[QUESTION: Who uses this?]' placeholder
      const draft = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test problem description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: '[QUESTION: What can users DO?]',
              description: 'Placeholder capability',
            },
          ],
        },
        personas: [
          {
            name: '[QUESTION: Who uses this?]',
            description: '[QUESTION: Who uses this?]',
            goals: ['[QUESTION: What are their goals?]'],
          },
        ],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run "fspec discover-foundation --finalize"
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: finalPath,
        cwd: testDir,
      });

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the error message should explain how to fill placeholders
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('fill all placeholder fields');
      expect(result.validationErrors).toContain('fspec update-foundation');
      expect(result.validationErrors).toContain('fspec add-capability');
      expect(result.validationErrors).toContain('fspec add-persona');

      // @step And the error message should explain how to remove unwanted placeholders
      expect(result.validationErrors).toContain('remove unwanted placeholder');

      // @step And the error message should include "fspec remove-persona" command example
      expect(result.validationErrors).toContain('fspec remove-persona');

      // @step And the error message should include "fspec remove-capability" command example
      expect(result.validationErrors).toContain('fspec remove-capability');
    });
  });
});
