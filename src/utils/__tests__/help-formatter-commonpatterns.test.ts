/**
 * Feature: spec/features/commonpatterns-help-formatting.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { formatCommandHelp, type CommandHelpConfig } from '../help-formatter';

describe('Feature: commonPatterns displays [object Object] in help output', () => {
  describe('Scenario: Display formatted patterns for object-style commonPatterns', () => {
    it('should display formatted pattern information with name, example, and description', () => {
      // Given I have a help file using object-style commonPatterns with pattern, example, and description fields
      const config: CommandHelpConfig = {
        name: 'test-command',
        description: 'Test command for pattern formatting',
        commonPatterns: [
          {
            pattern: 'Linting Before Implementation',
            example: 'fspec add-virtual-hook AUTH-001 pre-implementing "npm run lint" --blocking',
            description: 'Ensures code is clean before starting implementation. Prevents messy code from being committed.',
          },
          {
            pattern: 'Type Checking Before Validation',
            example: 'fspec add-virtual-hook AUTH-001 pre-validating "npm run typecheck" --blocking',
            description: 'Catches type errors before moving to validation phase. Strict quality gate.',
          },
        ],
      };

      // When I run the formatter
      const output = formatCommandHelp(config);

      // Then the COMMON PATTERNS section should display formatted pattern information
      expect(output).toContain('COMMON PATTERNS');

      // And each pattern should show the pattern name
      expect(output).toContain('Linting Before Implementation');
      expect(output).toContain('Type Checking Before Validation');

      // And each pattern should show the example command
      expect(output).toContain('fspec add-virtual-hook AUTH-001 pre-implementing "npm run lint" --blocking');
      expect(output).toContain('fspec add-virtual-hook AUTH-001 pre-validating "npm run typecheck" --blocking');

      // And each pattern should show the description
      expect(output).toContain('Ensures code is clean before starting implementation');
      expect(output).toContain('Catches type errors before moving to validation phase');

      // And the output should NOT contain "[object Object]"
      expect(output).not.toContain('[object Object]');
    });
  });

  describe('Scenario: Backward compatibility with string array format', () => {
    it('should display string patterns as bulleted list', () => {
      // Given I have a help file using string[] format for commonPatterns
      const config: CommandHelpConfig = {
        name: 'test-command',
        description: 'Test command for backward compatibility',
        commonPatterns: [
          'Use this pattern when you need simple guidance',
          'Another simple pattern example',
        ],
      };

      // When I run the formatter
      const output = formatCommandHelp(config);

      // Then the COMMON PATTERNS section should display as before
      expect(output).toContain('COMMON PATTERNS');

      // And each pattern should show as a bulleted string
      expect(output).toContain('Use this pattern when you need simple guidance');
      expect(output).toContain('Another simple pattern example');

      // And the formatter should not break existing string[] usage
      expect(output).not.toContain('[object Object]');
    });
  });

  describe('Scenario: All affected commands display COMMON PATTERNS correctly', () => {
    it('should handle object-style patterns without [object Object] errors', () => {
      // Given there are help files using object-style commonPatterns
      const objectStyleConfig: CommandHelpConfig = {
        name: 'add-virtual-hook',
        description: 'Add virtual hook command',
        commonPatterns: [
          {
            pattern: 'Multiple Quality Checks',
            example: 'fspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --blocking',
            description: 'Adds multiple hooks to same event. Both must pass to proceed.',
          },
        ],
      };

      // When I run any affected command with --help flag
      const output = formatCommandHelp(objectStyleConfig);

      // Then the COMMON PATTERNS section should display formatted content
      expect(output).toContain('COMMON PATTERNS');
      expect(output).toContain('Multiple Quality Checks');
      expect(output).toContain('fspec add-virtual-hook AUTH-001 post-implementing');

      // And no command should show "[object Object]" in help output
      expect(output).not.toContain('[object Object]');
    });
  });
});
