/**
 * Feature: spec/features/feature-file-prefill-detection.feature
 *
 * This test file validates the hybrid step extraction utility.
 */

import { describe, it, expect } from 'vitest';
import { extractStepsFromExample } from '../step-extraction';

describe('Feature: Hybrid Step Extraction from Examples', () => {
  describe('Scenario: Extract steps from action-oriented examples', () => {
    it('should extract Given/When/Then from "User runs command"', () => {
      // Given: Action-oriented example text
      const example = 'User runs fspec validate command';

      // When: I extract steps
      const steps = extractStepsFromExample(example);

      // Then: Steps should be intelligently extracted
      expect(steps.usedPrefill).toBe(false);
      expect(steps.given).toContain('User');
      expect(steps.when).toContain('runs');
      expect(steps.when).toContain('fspec validate command');
    });

    it('should extract from "LLM creates feature file"', () => {
      // Given: Action example with creates verb
      const example = 'LLM creates feature file with placeholders';

      // When: I extract steps
      const steps = extractStepsFromExample(example);

      // Then: Actor and action should be extracted
      expect(steps.usedPrefill).toBe(false);
      expect(steps.given).toContain('LLM');
      expect(steps.when).toContain('creates');
    });
  });

  describe('Scenario: Extract steps from condition-based examples', () => {
    it('should extract from "Feature file has valid syntax"', () => {
      // Given: Condition-based example
      const example = 'Feature file has valid syntax';

      // When: I extract steps
      const steps = extractStepsFromExample(example);

      // Then: Condition should be captured
      expect(steps.usedPrefill).toBe(false);
      expect(steps.given).toContain('Feature file');
      expect(steps.given).toContain('has');
      expect(steps.given).toContain('valid syntax');
    });
  });

  describe('Scenario: Extract steps from error examples', () => {
    it('should extract from "Command fails with error"', () => {
      // Given: Error-based example
      const example = 'Command fails with invalid syntax error';

      // When: I extract steps
      const steps = extractStepsFromExample(example);

      // Then: Error condition should be captured
      expect(steps.usedPrefill).toBe(false);
      expect(steps.when).toContain('Command');
      expect(steps.then).toContain('fails');
    });
  });

  describe('Scenario: Fall back to prefill for unparseable examples', () => {
    it('should use prefill for ambiguous text', () => {
      // Given: Ambiguous example text
      const example = 'Something happens somehow';

      // When: I extract steps
      const steps = extractStepsFromExample(example);

      // Then: Should fall back to prefill
      expect(steps.usedPrefill).toBe(true);
      expect(steps.given).toBe('[precondition]');
      expect(steps.when).toBe('[action]');
      expect(steps.then).toBe('[expected outcome]');
    });
  });

  describe('Scenario: Extract from explicit Given/When/Then', () => {
    it('should parse explicit GWT structure in example', () => {
      // Given: Example with explicit GWT
      const example =
        'given user is authenticated when user clicks logout then session ends';

      // When: I extract steps
      const steps = extractStepsFromExample(example);

      // Then: Should parse explicit structure
      expect(steps.usedPrefill).toBe(false);
      expect(steps.given?.toLowerCase()).toContain('user is authenticated');
      expect(steps.when?.toLowerCase()).toContain('user clicks logout');
      expect(steps.then?.toLowerCase()).toContain('session ends');
    });
  });
});
