/**
 * Feature: spec/features/preserve-example-mapping-context-as-comments-in-generated-feature-files.feature
 *
 * This test file validates comment preservation in the Gherkin formatter.
 * Scenarios tested:
 * - Formatter preserves comments through format cycles
 * - Gherkin parser captures comments in AST
 * - Formatter outputs comments from AST to correct positions
 */

import { describe, it, expect } from 'vitest';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import { formatGherkinDocument } from '../gherkin-formatter';

describe('Feature: Preserve example mapping context as comments in generated feature files', () => {
  describe('Scenario: Gherkin parser captures comments in AST', () => {
    it('should parse feature file and capture comments in ast.comments array', () => {
      // Given I have a feature file with # comment lines
      const featureContent = `@test
Feature: Test Feature

  # This is a comment before Background
  Background: User Story
    As a user
    I want something
    So that benefit

  # This is a comment before scenario
  Scenario: Test scenario
    Given precondition
    When action
    Then outcome
`;

      // When the file is parsed by @cucumber/gherkin parser
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      const ast = parser.parse(featureContent);

      // Then the AST should contain ast.comments array
      expect(ast.comments).toBeDefined();
      expect(Array.isArray(ast.comments)).toBe(true);

      // And each comment should have line number and text
      expect(ast.comments!.length).toBeGreaterThan(0);
      ast.comments!.forEach(comment => {
        expect(comment.location).toBeDefined();
        expect(comment.location.line).toBeGreaterThan(0);
        expect(comment.text).toBeDefined();
        expect(typeof comment.text).toBe('string');
      });

      // And comments are not stripped during parsing
      expect(ast.comments!.length).toBe(2); // Two comments in the file
      expect(ast.comments![0].text).toContain('comment before Background');
      expect(ast.comments![1].text).toContain('comment before scenario');
    });
  });

  describe('Scenario: Formatter outputs comments from AST to correct positions', () => {
    it('should include all comments in formatted output at correct line positions', () => {
      // Given I have a Gherkin AST with comments in ast.comments
      const featureContent = `@test
Feature: Test Feature

  # Comment before Background
  Background: User Story
    As a user
    I want something
    So that benefit

  # Comment before scenario
  Scenario: Test scenario
    Given precondition
    When action
    Then outcome
`;

      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      const ast = parser.parse(featureContent);

      // When formatGherkinDocument() is called
      const formatted = formatGherkinDocument(ast);

      // Then the formatted output should include all comments
      expect(formatted).toContain('# Comment before Background');
      expect(formatted).toContain('# Comment before scenario');

      // And comments should be inserted at their original line positions
      const lines = formatted.split('\n');
      const commentBeforeBackgroundIndex = lines.findIndex(line =>
        line.includes('# Comment before Background')
      );
      const backgroundIndex = lines.findIndex(line =>
        line.includes('Background:')
      );
      const commentBeforeScenarioIndex = lines.findIndex(line =>
        line.includes('# Comment before scenario')
      );
      const scenarioIndex = lines.findIndex(line => line.includes('Scenario:'));

      // And comments should appear before the elements they precede
      expect(commentBeforeBackgroundIndex).toBeLessThan(backgroundIndex);
      expect(commentBeforeBackgroundIndex).toBeGreaterThan(-1);
      expect(commentBeforeScenarioIndex).toBeLessThan(scenarioIndex);
      expect(commentBeforeScenarioIndex).toBeGreaterThan(-1);
    });
  });

  describe('Scenario: Formatter preserves comments through format cycles', () => {
    it('should maintain comments after multiple format operations', () => {
      // Given I have a feature file with example mapping comments
      const featureContent = `@test
Feature: Test Feature

  # EXAMPLE MAPPING CONTEXT
  # =======================
  #
  # BUSINESS RULES:
  #   1. Passwords must be 8+ characters
  #   2. Account locks after 5 failed attempts
  #
  # EXAMPLES:
  #   1. User logs in with valid credentials
  #   2. User enters wrong password 5 times
  #
  # =======================

  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;

      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      // When I run "fspec format" on the file (simulate multiple format cycles)
      let ast = parser.parse(featureContent);
      let formatted = formatGherkinDocument(ast);

      // Format again (second cycle)
      ast = parser.parse(formatted);
      formatted = formatGherkinDocument(ast);

      // Format again (third cycle)
      ast = parser.parse(formatted);
      formatted = formatGherkinDocument(ast);

      // Then the formatted file should still contain the comment block
      expect(formatted).toContain('# EXAMPLE MAPPING CONTEXT');
      expect(formatted).toContain('# BUSINESS RULES:');
      expect(formatted).toContain('# EXAMPLES:');

      // And the comments should be in the same location
      const lines = formatted.split('\n');
      const exampleMappingIndex = lines.findIndex(line =>
        line.includes('# EXAMPLE MAPPING CONTEXT')
      );
      const backgroundIndex = lines.findIndex(line =>
        line.includes('Background:')
      );
      expect(exampleMappingIndex).toBeLessThan(backgroundIndex);

      // And no comments should be lost
      const commentLines = lines.filter(line => line.trim().startsWith('#'));
      expect(commentLines.length).toBeGreaterThan(10); // We have many comment lines
    });
  });

  describe('Scenario: Example mapping comments coexist with architecture doc strings', () => {
    it('should preserve both # comments and """ doc strings without conflict', () => {
      // Given I generate scenarios from a work unit
      // And the feature file has "# EXAMPLE MAPPING CONTEXT" comments
      const featureContent = `@test
Feature: Test Feature

  """
  Architecture notes:
  - Uses @cucumber/gherkin for parsing
  - Supports all Gherkin keywords
  """

  # EXAMPLE MAPPING CONTEXT
  # Business rule: passwords 8+ chars

  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Test
    Given precondition
    When action
    Then outcome
`;

      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      const ast = parser.parse(featureContent);

      // When I run "fspec add-architecture" to add technical notes (simulated by format)
      const formatted = formatGherkinDocument(ast);

      // Then the file should contain both # comments AND """ doc string
      expect(formatted).toContain('"""');
      expect(formatted).toContain('Architecture notes:');
      expect(formatted).toContain('# EXAMPLE MAPPING CONTEXT');
      expect(formatted).toContain('# Business rule: passwords 8+ chars');

      // And neither should overwrite the other
      const lines = formatted.split('\n');
      const hasDocString = lines.some(line =>
        line.includes('Architecture notes:')
      );
      const hasComments = lines.some(line =>
        line.includes('# EXAMPLE MAPPING CONTEXT')
      );
      expect(hasDocString).toBe(true);
      expect(hasComments).toBe(true);

      // And format command should preserve both
      const ast2 = parser.parse(formatted);
      const reformatted = formatGherkinDocument(ast2);
      expect(reformatted).toContain('Architecture notes:');
      expect(reformatted).toContain('# EXAMPLE MAPPING CONTEXT');
    });
  });
});
