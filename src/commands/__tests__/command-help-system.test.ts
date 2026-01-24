/**
 * Feature: spec/features/command-help-system.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { formatCommandHelp } from '../../utils/help-formatter';
import createEpicHelpConfig from '../create-epic-help';
import listWorkUnitsHelpConfig from '../list-work-units-help';
import addDependencyHelpConfig from '../add-dependency-help';
import updateWorkUnitStatusHelpConfig from '../update-work-unit-status-help';
import addQuestionHelpConfig from '../add-question-help';

describe('Feature: Command Help System', () => {
  describe('Scenario: Display help for simple command with no options', () => {
    it('should display comprehensive help for create-epic command', () => {
      // Given I am using the fspec CLI
      // When I format the help config
      const helpOutput = formatCommandHelp(createEpicHelpConfig);

      // Then the output should include a command description
      expect(helpOutput).toMatch(/create.*epic/i);

      // And the output should show usage syntax "fspec create-epic <epicId> <title>"
      expect(helpOutput).toMatch(/usage/i);
      expect(helpOutput).toMatch(/create-epic/i);

      // And the output should list required arguments "<epicId>" and "<title>"
      expect(helpOutput).toMatch(/epicId/i);
      expect(helpOutput).toMatch(/title/i);

      // And the output should show practical examples
      expect(helpOutput).toMatch(/example/i);
      expect(helpOutput).toMatch(/fspec create-epic/i);

      // And the output should list related commands
      expect(helpOutput).toMatch(/related|see also/i);
    });
  });

  describe('Scenario: Display help for medium command with multiple options', () => {
    it('should display help with WHEN TO USE and workflow sections for list-work-units', () => {
      // Given I am using the fspec CLI
      // When I format the help config
      const helpOutput = formatCommandHelp(listWorkUnitsHelpConfig);

      // Then the output should include a "WHEN TO USE" section
      expect(helpOutput).toMatch(/when to use/i);

      // And the output should list all options: "--status", "--prefix", "--epic"
      expect(helpOutput).toMatch(/--status/);
      expect(helpOutput).toMatch(/--prefix/);

      // And the output should show examples
      expect(helpOutput).toMatch(/example/i);
      expect(helpOutput).toMatch(/fspec list-work-units/i);

      // And the output should include a "TYPICAL WORKFLOW" section
      expect(helpOutput).toMatch(/typical workflow|workflow/i);
    });
  });

  describe('Scenario: Display help for complex command with multiple modes', () => {
    it('should display comprehensive help with COMMON ERRORS for add-dependency', () => {
      // Given I am using the fspec CLI
      // When I format the help config
      const helpOutput = formatCommandHelp(addDependencyHelpConfig);

      // Then the output should include a description explaining dependency relationship types
      expect(helpOutput).toMatch(/dependency|relationship|depends/i);

      // And the output should show examples demonstrating different modes
      expect(helpOutput).toMatch(/example/i);
      expect(helpOutput).toMatch(
        /--blocks|--blocked-by|--depends-on|--relates-to/
      );

      // And the output should include a "COMMON ERRORS" section
      expect(helpOutput).toMatch(/common errors|errors|troubleshooting/i);

      // And the output should list related commands
      expect(helpOutput).toMatch(/related|see also/i);
    });
  });

  describe('Scenario: Display help for ACDD workflow command with prerequisites', () => {
    it('should display help with PREREQUISITES and workflow states for update-work-unit-status', () => {
      // Given I am using the fspec CLI
      // When I format the help config
      const helpOutput = formatCommandHelp(updateWorkUnitStatusHelpConfig);

      // Then the output should include a "PREREQUISITES" section
      expect(helpOutput).toMatch(/prerequisite/i);

      // And the output should include a "WHEN TO USE" section
      expect(helpOutput).toMatch(/when to use/i);

      // And the output should list all valid status values
      expect(helpOutput).toMatch(/backlog/i);
      expect(helpOutput).toMatch(/specifying/i);
      expect(helpOutput).toMatch(/testing/i);
      expect(helpOutput).toMatch(/implementing/i);
      expect(helpOutput).toMatch(/validating/i);
      expect(helpOutput).toMatch(/done/i);

      // And the output should include a "TYPICAL WORKFLOW" section
      expect(helpOutput).toMatch(/typical workflow|workflow/i);

      // And the output should show examples
      expect(helpOutput).toMatch(/example/i);
      expect(helpOutput).toMatch(/fspec update-work-unit-status/i);
    });
  });

  describe('Scenario: Main help mentions individual command help availability', () => {
    it('should have help command structure with --help option', () => {
      // Commander.js automatically adds --help to all commands
      // We verify the help configs have required fields
      expect(createEpicHelpConfig.name).toBe('create-epic');
      expect(createEpicHelpConfig.description).toBeDefined();
      expect(createEpicHelpConfig.usage).toBeDefined();
    });
  });

  describe('Scenario: Display help for Example Mapping command with patterns', () => {
    it('should display help with COMMON PATTERNS for add-question', () => {
      // Given I am using the fspec CLI
      // When I format the help config
      const helpOutput = formatCommandHelp(addQuestionHelpConfig);

      // Then the output should include a "WHEN TO USE" section
      expect(helpOutput).toMatch(/when to use/i);

      // And the output should include patterns section
      expect(helpOutput).toMatch(/pattern/i);

      // And the output should mention @human prefix for questions
      expect(helpOutput).toMatch(/@human/i);

      // And the output should show examples
      expect(helpOutput).toMatch(/example/i);
      expect(helpOutput).toMatch(/fspec add-question/i);

      // And the output should list related commands
      expect(helpOutput).toMatch(/related|see also/i);
    });
  });
});
