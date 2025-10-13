/**
 * Feature: spec/features/command-help-system.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
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
  } catch (error: any) {
    return {
      stdout: error.stdout || '',
      stderr: error.stderr || '',
      exitCode: error.code || 1,
    };
  }
}

describe('Feature: Command Help System', () => {
  describe('Scenario: Display help for simple command with no options', () => {
    it('should display comprehensive help for create-epic command', async () => {
      // Given I am using the fspec CLI
      // When I run "fspec create-epic --help"
      const result = await runCommand(`node ${CLI_PATH} create-epic --help`);

      // Then the output should display a colored help section
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();

      // And the output should include a command description
      expect(result.stdout).toMatch(/create.*epic/i);
      expect(result.stdout).toMatch(/description|purpose|creates/i);

      // And the output should show usage syntax "fspec create-epic <epicId> <title>"
      expect(result.stdout).toMatch(/usage|syntax/i);
      expect(result.stdout).toMatch(/create-epic.*<epicId>.*<title>/i);

      // And the output should indicate no options available
      expect(result.stdout).toMatch(/options|no options/i);

      // And the output should list required arguments "<epicId>" and "<title>"
      expect(result.stdout).toMatch(/<epicId>/i);
      expect(result.stdout).toMatch(/<title>/i);

      // And the output should show 2 practical examples with expected output
      expect(result.stdout).toMatch(/example/i);
      const exampleMatches = result.stdout.match(/fspec create-epic/gi);
      expect(exampleMatches).toBeTruthy();
      expect(exampleMatches!.length).toBeGreaterThanOrEqual(2);

      // And the output should list related commands
      expect(result.stdout).toMatch(/related|see also/i);
      expect(result.stdout).toMatch(/list-epics|show-epic|create-work-unit/i);
    });
  });

  describe('Scenario: Display help for medium command with multiple options', () => {
    it('should display help with WHEN TO USE and workflow sections for list-work-units', async () => {
      // Given I am using the fspec CLI
      // When I run "fspec list-work-units --help"
      const result = await runCommand(
        `node ${CLI_PATH} list-work-units --help`
      );

      // Then the output should display a colored help section
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();

      // And the output should include a "WHEN TO USE" section
      expect(result.stdout).toMatch(/when to use/i);

      // And the output should list all options: "--status", "--prefix", "--epic"
      expect(result.stdout).toMatch(/--status/);
      expect(result.stdout).toMatch(/--prefix/);
      expect(result.stdout).toMatch(/--epic/);

      // And the output should show 3 examples demonstrating different filter combinations
      expect(result.stdout).toMatch(/example/i);
      const exampleMatches = result.stdout.match(/fspec list-work-units/gi);
      expect(exampleMatches).toBeTruthy();
      expect(exampleMatches!.length).toBeGreaterThanOrEqual(3);

      // And the examples should include expected output samples
      // Examples show actual work unit IDs like AUTH-001, UI-002, etc as output
      expect(result.stdout).toMatch(/AUTH-\d+|UI-\d+/i);

      // And the output should include a "TYPICAL WORKFLOW" section mentioning "backlog → specifying" flow
      expect(result.stdout).toMatch(/typical workflow|workflow/i);
      expect(result.stdout).toMatch(/backlog.*specifying|backlog.*→.*specifying/i);
    });
  });

  describe('Scenario: Display help for complex command with multiple modes', () => {
    it('should display comprehensive help with COMMON ERRORS for add-dependency', async () => {
      // Given I am using the fspec CLI
      // When I run "fspec add-dependency --help"
      const result = await runCommand(`node ${CLI_PATH} add-dependency --help`);

      // Then the output should display a colored help section
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();

      // And the output should include a description explaining dependency relationship types
      expect(result.stdout).toMatch(/dependency|relationship|depends/i);
      expect(result.stdout).toMatch(/track|blocker|relationship/i);

      // And the output should show 5 examples demonstrating different modes
      expect(result.stdout).toMatch(/example/i);
      expect(result.stdout).toMatch(/shorthand/i);
      expect(result.stdout).toMatch(/--blocks/);
      expect(result.stdout).toMatch(/--blocked-by/);
      expect(result.stdout).toMatch(/--depends-on/);
      expect(result.stdout).toMatch(/--relates-to/);

      const exampleMatches = result.stdout.match(/fspec add-dependency/gi);
      expect(exampleMatches).toBeTruthy();
      expect(exampleMatches!.length).toBeGreaterThanOrEqual(5);

      // And the output should include a "COMMON ERRORS" section
      expect(result.stdout).toMatch(/common errors|errors|troubleshooting/i);

      // And the "COMMON ERRORS" section should show "work unit not found" error with fix
      expect(result.stdout).toMatch(/work unit.*not found|not found/i);
      expect(result.stdout).toMatch(/fix|solution|resolve/i);

      // And the output should list related commands
      expect(result.stdout).toMatch(/related|see also/i);
      expect(result.stdout).toMatch(
        /remove-dependency|dependencies|export-dependencies/i
      );
    });
  });

  describe('Scenario: Display help for ACDD workflow command with prerequisites', () => {
    it('should display help with PREREQUISITES and workflow states for update-work-unit-status', async () => {
      // Given I am using the fspec CLI
      // When I run "fspec update-work-unit-status --help"
      const result = await runCommand(
        `node ${CLI_PATH} update-work-unit-status --help`
      );

      // Then the output should display a colored help section
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();

      // And the output should include a "PREREQUISITES" section stating "work unit must exist"
      expect(result.stdout).toMatch(/prerequisite/i);
      expect(result.stdout).toMatch(/work unit.*exist|must exist/i);

      // And the output should include a "WHEN TO USE" section explaining "moving through ACDD workflow"
      expect(result.stdout).toMatch(/when to use/i);
      expect(result.stdout).toMatch(/acdd|workflow|moving through/i);

      // And the output should list all valid status values
      expect(result.stdout).toMatch(/backlog/i);
      expect(result.stdout).toMatch(/specifying/i);
      expect(result.stdout).toMatch(/testing/i);
      expect(result.stdout).toMatch(/implementing/i);
      expect(result.stdout).toMatch(/validating/i);
      expect(result.stdout).toMatch(/done/i);
      expect(result.stdout).toMatch(/blocked/i);

      // And the output should include a "TYPICAL WORKFLOW" section showing the complete flow
      expect(result.stdout).toMatch(/typical workflow|workflow/i);
      expect(result.stdout).toMatch(
        /backlog.*specifying.*testing.*implementing.*validating.*done/i
      );

      // And the output should show an example with expected status change output
      expect(result.stdout).toMatch(/example/i);
      expect(result.stdout).toMatch(/fspec update-work-unit-status/i);
      expect(result.stdout).toMatch(/status.*updated|✓.*status/i);
    });
  });

  describe('Scenario: Main help mentions individual command help availability', () => {
    it('should display note about command-specific help in main help', async () => {
      // Given I am using the fspec CLI
      // When I run "fspec --help"
      const result = await runCommand(`node ${CLI_PATH} --help`);

      // Then the output should display the main help menu
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();
      expect(result.stdout).toMatch(/usage|fspec/i);

      // And the output should include a note stating "Get detailed help for any command with: fspec <command> --help"
      expect(result.stdout).toMatch(
        /detailed help|get help|command.*--help|<command>.*--help/i
      );
      expect(result.stdout).toMatch(/fspec.*<command>.*--help/);
    });
  });

  describe('Scenario: Display help for Example Mapping command with patterns', () => {
    it('should display help with COMMON PATTERNS for add-question', async () => {
      // Given I am using the fspec CLI
      // When I run "fspec add-question --help"
      const result = await runCommand(`node ${CLI_PATH} add-question --help`);

      // Then the output should display a colored help section
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toBeTruthy();

      // And the output should include a "WHEN TO USE" section stating "during Example Mapping in specifying phase"
      expect(result.stdout).toMatch(/when to use/i);
      expect(result.stdout).toMatch(
        /example mapping|specifying phase|discovery/i
      );

      // And the output should include a "COMMON PATTERNS" section
      expect(result.stdout).toMatch(/common patterns|patterns/i);

      // And the "COMMON PATTERNS" section should mention "use @human: prefix for questions directed at human"
      expect(result.stdout).toMatch(/@human|@human:/i);
      expect(result.stdout).toMatch(/prefix|questions.*human/i);

      // And the output should show an example demonstrating question with answer flow
      expect(result.stdout).toMatch(/example/i);
      expect(result.stdout).toMatch(/fspec add-question/i);
      expect(result.stdout).toMatch(/answer|answer-question/i);

      // And the output should list related commands
      expect(result.stdout).toMatch(/related|see also/i);
      expect(result.stdout).toMatch(
        /answer-question|add-rule|add-example|generate-scenarios/i
      );
    });
  });
});
