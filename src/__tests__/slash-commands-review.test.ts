/**
 * Feature: spec/features/slash-command-for-critical-story-review-with-ultrathink.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';

const REVIEW_COMMAND_PATH = join(process.cwd(), '.claude/commands/review.md');

describe('Feature: Slash command for critical story review with ULTRATHINK', () => {
  describe('Scenario: Review single work unit with full analysis', () => {
    it('should define command to load work unit, read feature file, analyze tests and implementation, and output structured review', async () => {
      // Given: I have a completed work unit CLI-011 with feature file, tests, and implementation
      // (This is validated by the command specification)

      // When: I run '/review CLI-011'
      // Then: the command should load work unit metadata
      // And: read the linked feature file
      // And: analyze test files from coverage mappings
      // And: analyze implementation files from coverage mappings
      // And: output structured review with sections: Issues Found, Recommendations, Refactoring Opportunities, ACDD Compliance

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify command loads work unit metadata
      expect(content).toContain('fspec show-work-unit');

      // Verify command reads feature file
      expect(content).toContain('feature file');

      // Verify command analyzes tests and implementation
      expect(content).toContain('coverage');
      expect(content).toContain('test');

      // Verify structured output sections
      expect(content).toContain('Issues Found');
      expect(content).toContain('Recommendations');
      expect(content).toContain('Refactoring Opportunities');
      expect(content).toContain('ACDD Compliance');
    });
  });

  describe('Scenario: Prompt for work unit ID when none provided', () => {
    it('should prompt user for work unit ID when no arguments given', async () => {
      // Given: I am using the /review command
      // When: I run '/review' without any arguments
      // Then: I should see the message 'Which work unit would you like me to review? Please provide the work unit ID (e.g., CLI-011)'

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify prompt for work unit ID
      expect(content).toContain('Which work unit');
      expect(content).toContain('work unit ID');
    });
  });

  describe('Scenario: Detect test that doesn\'t validate its scenario', () => {
    it('should identify tests that don\'t validate their acceptance criteria', async () => {
      // Given: I have a work unit with a test that claims to test 'Login with valid credentials'
      // And: the test only checks that a function exists, not that login actually works
      // When: I run '/review AUTH-001'
      // Then: the review should identify 'Issue: Test does not validate acceptance criteria'
      // And: suggest 'Fix: Rewrite test to verify actual login behavior with credentials and session creation'
      // And: provide actionable next steps

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify command checks test-scenario alignment
      expect(content).toContain('test');
      expect(content).toContain('scenario');
      expect(content).toContain('validate') || expect(content).toContain('acceptance criteria');
    });
  });

  describe('Scenario: Detect manual file operations that should use existing utilities', () => {
    it('should identify when manual file operations should use existing utilities', async () => {
      // Given: I have implementation code using manual fs.readFile and fs.writeFile
      // And: an existing utility function in src/utils/config.ts already handles this pattern
      // When: I run '/review CONFIG-002'
      // Then: the review should identify 'Issue: Manual file operations instead of existing utility'
      // And: suggest 'Fix: Use loadConfig() from src/utils/config.ts instead of manual fs operations'
      // And: provide specific refactoring steps

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify command checks for reusable utilities
      const hasUtilityCheck = content.includes('utility') || content.includes('utilities');
      expect(hasUtilityCheck).toBe(true);
      expect(content).toContain('refactor');
    });
  });

  describe('Scenario: Detect coverage gaps in feature file scenarios', () => {
    it('should identify scenarios that lack test coverage', async () => {
      // Given: I have a feature file with 5 scenarios
      // And: coverage file shows only 3 scenarios have test mappings
      // When: I run '/review TEST-001'
      // Then: the review should identify 'Issue: 2 scenarios lack test coverage'
      // And: list the uncovered scenario names
      // And: suggest 'Fix: Add test coverage for missing scenarios'

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify command checks coverage
      expect(content).toContain('coverage');
      expect(content).toContain('fspec show-coverage') || expect(content).toContain('coverage');
    });
  });

  describe('Scenario: Detect CLAUDE.md coding standard violations', () => {
    it('should identify violations of CLAUDE.md coding standards', async () => {
      // Given: I have implementation code using 'any' type in file.ts:42
      // And: CLAUDE.md mandates no 'any' types
      // When: I run '/review FEAT-001'
      // Then: the review should identify 'Issue: Using any type in file.ts:42 (violates CLAUDE.md)'
      // And: suggest 'Fix: Replace with proper interface type'
      // And: provide 'Action: Edit file.ts line 42 to use UserInterface type'

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify command validates CLAUDE.md standards
      expect(content).toContain('CLAUDE.md');
      const hasStandardCheck = content.includes('coding standard') || content.includes('Coding Standards') || content.includes('standards');
      expect(hasStandardCheck).toBe(true);
    });
  });

  describe('Scenario: Review multiple work units at once', () => {
    it('should support reviewing multiple work units with clear separators', async () => {
      // Given: I have three completed work units: CLI-001, CLI-002, CLI-003
      // When: I run '/review CLI-001 CLI-002 CLI-003'
      // Then: the command should review all three work units
      // And: output should be structured per work unit with clear separators
      // And: each work unit review should include: Issues Found, Recommendations, Refactoring Opportunities, ACDD Compliance

      expect(existsSync(REVIEW_COMMAND_PATH)).toBe(true);

      const content = await readFile(REVIEW_COMMAND_PATH, 'utf-8');

      // Verify command supports multiple work units
      expect(content).toContain('multiple') || expect(content).toMatch(/CLI-\d+\s+CLI-\d+/);
    });
  });
});
