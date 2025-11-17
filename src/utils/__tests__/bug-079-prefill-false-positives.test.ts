/**
 * Feature: spec/features/prefill-detection-incorrectly-flags-code-syntax-as-placeholders.feature
 *
 * Tests for BUG-079: Prefill detection false positives on code syntax
 */

import { describe, it, expect } from 'vitest';
import { detectPrefill } from '../prefill-detection';

describe('Feature: Prefill detection incorrectly flags code syntax as placeholders', () => {
  describe('Scenario: Code syntax with brackets should not trigger prefill detection', () => {
    it('should NOT flag array access syntax as placeholder', () => {
      // @step Given I have a feature file with step "Then only work-unit.ts should contain workUnitsData.workUnits with id = object logic"
      const content = `Feature: Test
  Scenario: Test
    Then only work-unit.ts should contain workUnitsData.workUnits[id] = object logic`;

      // @step When I run prefill detection on the feature file
      const result = detectPrefill(content);

      // @step Then prefill detection should NOT flag the step as containing placeholders
      expect(result.hasPrefill).toBe(false);

      // @step And the step should be considered complete
      expect(result.matches.length).toBe(0);
    });
  });

  describe('Scenario: Actual placeholder patterns should trigger prefill detection', () => {
    it('should flag specific placeholder patterns like [role]', () => {
      // @step Given I have a feature file with step "Given I have role placeholder"
      const content = `Feature: Test
  Scenario: Test
    Given I have [role] placeholder`;

      // @step When I run prefill detection on the feature file
      const result = detectPrefill(content);

      // @step Then prefill detection SHOULD flag the step as containing placeholder "role"
      expect(result.hasPrefill).toBe(true);
      const roleMatches = result.matches.filter(m => m.pattern === '[role]');
      expect(roleMatches.length).toBeGreaterThan(0);

      // @step And the step should be considered incomplete
      expect(result.matches.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Array access syntax should not trigger prefill detection', () => {
    it('should NOT flag array index access as placeholder', () => {
      // @step Given I have a feature file with step "When I access array with index in code"
      const content = `Feature: Test
  Scenario: Test
    When I access array[index] in code`;

      // @step When I run prefill detection on the feature file
      const result = detectPrefill(content);

      // @step Then prefill detection should NOT flag the step as containing placeholders
      expect(result.hasPrefill).toBe(false);

      // @step And the step should be considered complete
      expect(result.matches.length).toBe(0);
    });
  });
});
