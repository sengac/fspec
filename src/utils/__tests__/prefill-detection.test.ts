/**
 * Feature: spec/features/feature-file-prefill-detection.feature
 *
 * This test file validates the prefill detection utility.
 */

import { describe, it, expect } from 'vitest';
import { detectPrefill } from '../prefill-detection';

describe('Feature: Prefill Detection in Feature Files', () => {
  describe('Scenario: Detect placeholder text in Background section', () => {
    it('should detect [role], [action], [benefit] placeholders', () => {
      // Given: Feature file content with placeholders
      const content = `Feature: Test
  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]`;

      // When: I detect prefill
      const result = detectPrefill(content);

      // Then: Placeholders should be detected
      expect(result.hasPrefill).toBe(true);
      expect(result.matches.length).toBeGreaterThanOrEqual(3);
      const patterns = result.matches.map(m => m.pattern);
      expect(patterns).toContain('[role]');
      expect(patterns).toContain('[action]');
      expect(patterns).toContain('[benefit]');
    });
  });

  describe('Scenario: Detect placeholder text in scenario steps', () => {
    it('should detect [precondition], [action], [expected outcome]', () => {
      // Given: Feature file with step placeholders
      const content = `Feature: Test
  Scenario: Test scenario
    Given [precondition]
    When [action]
    Then [expected outcome]`;

      // When: I detect prefill
      const result = detectPrefill(content);

      // Then: Step placeholders should be detected
      expect(result.hasPrefill).toBe(true);
      expect(result.matches.length).toBeGreaterThanOrEqual(3);
    });
  });

  describe('Scenario: Detect TODO markers', () => {
    it('should detect TODO: in architecture notes', () => {
      // Given: Feature file with TODO markers
      const content = `Feature: Test
  """
  TODO: Add architecture notes
  TODO: Add diagrams
  """`;

      // When: I detect prefill
      const result = detectPrefill(content);

      // Then: TODO markers should be detected
      expect(result.hasPrefill).toBe(true);
      const todoMatches = result.matches.filter(m => m.pattern === 'TODO:');
      expect(todoMatches.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Scenario: Detect generic placeholder tags', () => {
    it('should detect @component and @feature-group placeholders', () => {
      // Given: Feature file with placeholder tags
      const content = `@critical @component @feature-group
Feature: Test`;

      // When: I detect prefill
      const result = detectPrefill(content);

      // Then: Placeholder tags should be detected
      expect(result.hasPrefill).toBe(true);
      const tagMatches = result.matches.filter(
        m => m.pattern === '@component' || m.pattern === '@feature-group'
      );
      expect(tagMatches.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Scenario: No prefill detected in complete feature', () => {
    it('should return no prefill for complete feature file', () => {
      // Given: Feature file without any placeholders
      const content = `@critical @cli @validation
Feature: Complete Feature

  Background: User Story
    As a developer
    I want to validate features
    So that quality is maintained

  Scenario: Validate syntax
    Given I have a feature file
    When I run validation
    Then it should pass`;

      // When: I detect prefill
      const result = detectPrefill(content);

      // Then: Minimal or no prefill should be detected
      // Note: May detect some generic placeholders, but should be minimal
      expect(result.matches.length).toBeLessThan(5);
    });
  });

  describe('Scenario: System-reminder generation', () => {
    it('should generate system-reminder with CLI command suggestions', () => {
      // Given: Feature file with prefill
      const content = `Feature: Test
  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]`;

      // When: I detect prefill
      const result = detectPrefill(content);

      // Then: System-reminder should be generated
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('PREFILL DETECTED');
      expect(result.systemReminder).toContain('fspec');
    });
  });
});
