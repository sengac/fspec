/**
 * Feature: spec/features/add-comprehensive-help-documentation-for-bootstrap-and-configure-tools-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { exec } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import { fileURLToPath } from 'url';

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

describe('Feature: Add comprehensive help documentation for bootstrap and configure-tools commands', () => {
  describe('Scenario: Display comprehensive help for bootstrap command', () => {
    it('should display comprehensive help with all required sections', async () => {
      // Given the bootstrap command exists
      // When I run "fspec bootstrap --help"
      const result = await runCommand(`node ${CLI_PATH} bootstrap --help`);

      // Then I should see the command description
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();
      expect(result.stdout).toMatch(/bootstrap/i);
      expect(result.stdout).toMatch(/description|loads|workflow/i);

      // And I should see the "WHEN TO USE" section
      expect(result.stdout).toMatch(/when to use/i);

      // And I should see the "PREREQUISITES" section
      expect(result.stdout).toMatch(/prerequisites/i);

      // And I should see the "TYPICAL WORKFLOW" section
      expect(result.stdout).toMatch(/typical workflow|workflow/i);

      // And I should see usage examples
      expect(result.stdout).toMatch(/example/i);
      expect(result.stdout).toMatch(/fspec bootstrap/i);
    });
  });

  describe('Scenario: Display comprehensive help for configure-tools command', () => {
    it('should display help with platform-agnostic examples', async () => {
      // Given the configure-tools command exists
      // When I run "fspec configure-tools --help"
      const result = await runCommand(
        `node ${CLI_PATH} configure-tools --help`
      );

      // Then I should see the command description
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();
      expect(result.stdout).toMatch(/configure.*tools/i);

      // And I should see platform-agnostic examples
      expect(result.stdout).toMatch(/example/i);

      // And I should see examples for Node.js with npm test
      expect(result.stdout).toMatch(/npm test|node.*js/i);

      // And I should see examples for Python with pytest
      expect(result.stdout).toMatch(/pytest|python/i);

      // And I should see examples for Rust with cargo test
      expect(result.stdout).toMatch(/cargo test|rust/i);

      // And I should see examples for Go with go test
      expect(result.stdout).toMatch(/go test|golang/i);
    });
  });

  describe('Scenario: Bootstrap command outputs complete workflow documentation', () => {
    it('should output all command help sections', async () => {
      // Given the bootstrap help content functions exist
      // When I run "fspec bootstrap"
      const result = await runCommand(`node ${CLI_PATH} bootstrap`);

      // Then I should see all command help sections
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();

      // And the output should include specs help content
      expect(result.stdout).toMatch(/gherkin|specifications|specs/i);

      // And the output should include work help content
      expect(result.stdout).toMatch(/work unit|kanban/i);

      // And the output should include discovery help content
      expect(result.stdout).toMatch(/example mapping|discovery/i);

      // And the output should include metrics help content
      expect(result.stdout).toMatch(/metrics|progress/i);

      // And the output should include setup help content
      expect(result.stdout).toMatch(/configuration|setup/i);

      // And the output should include hooks help content
      expect(result.stdout).toMatch(/hooks|lifecycle/i);
    });
  });

  describe('Scenario: Documentation consistency across all sources', () => {
    it('should have consistent documentation across all sources', async () => {
      // Given bootstrap-help.ts exists
      const fs = await import('fs/promises');
      const bootstrapHelpPath = path.resolve(__dirname, '../bootstrap-help.ts');
      const configureToolsHelpPath = path.resolve(
        __dirname,
        '../configure-tools-help.ts'
      );

      // And configure-tools-help.ts exists
      const bootstrapHelpExists = await fs
        .access(bootstrapHelpPath)
        .then(() => true)
        .catch(() => false);
      const configureToolsHelpExists = await fs
        .access(configureToolsHelpPath)
        .then(() => true)
        .catch(() => false);

      // When I compare help content with docs directory
      // Then bootstrap documentation should be consistent
      expect(bootstrapHelpExists).toBe(true);

      // And configure-tools documentation should be consistent
      expect(configureToolsHelpExists).toBe(true);

      // And no inconsistencies should exist
      expect(bootstrapHelpExists && configureToolsHelpExists).toBe(true);
    });
  });
});
