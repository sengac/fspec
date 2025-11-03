/**
 * Feature: spec/features/step-validation-system-reminder-doesn-t-explain-proper-comment-placement-and-deduplication.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import {
  formatValidationError,
  type ValidationResult,
} from '../step-validation';

describe('Feature: Step validation system-reminder clarity', () => {
  describe('Scenario: System-reminder includes placement guidance for step comments', () => {
    it('should include placement guidance in the error message', () => {
      // Given a test file is missing step comments for a scenario
      const validationResult: ValidationResult = {
        valid: false,
        matches: [
          {
            featureStep:
              'When the scanDraftForNextField function processes the draft',
            testComment: null,
            matched: false,
          },
        ],
        missingSteps: [
          'When the scanDraftForNextField function processes the draft',
        ],
        unmatchedComments: [],
      };

      // When the link-coverage command validates step comments
      // And step validation fails
      const errorMessage = formatValidationError(validationResult, 'story');

      // Then the system-reminder should contain "Place step comments NEAR the code that executes each step"
      expect(errorMessage).toContain(
        'Place step comments NEAR the code that executes each step'
      );

      // And the system-reminder should explain which line of code executes the step
      expect(errorMessage).toContain('the line that executes');

      // And the system-reminder should provide a contextual example of proper placement
      expect(errorMessage).toContain('Example:');
      expect(errorMessage).toContain('right before');
    });
  });

  describe('Scenario: System-reminder includes deduplication guidance for step comments', () => {
    it('should include deduplication guidance in the error message', () => {
      // Given a test file has existing Given/When/Then comments
      // And the test file is missing @step comments for a scenario
      const validationResult: ValidationResult = {
        valid: false,
        matches: [
          {
            featureStep: 'Given I have a foundation.json.draft',
            testComment: null,
            matched: false,
          },
        ],
        missingSteps: ['Given I have a foundation.json.draft'],
        unmatchedComments: [],
      };

      // When the link-coverage command validates step comments
      // And step validation fails
      const errorMessage = formatValidationError(validationResult, 'story');

      // Then the system-reminder should contain "If you have duplicate Given/When/Then comments, remove them first"
      expect(errorMessage).toContain('duplicate Given/When/Then comments');
      expect(errorMessage).toContain('remove them first');

      // And the system-reminder should explain that @step comments replace existing step comments
      expect(errorMessage).toContain('@step comments replace');

      // And the system-reminder should warn against creating redundant comments
      expect(errorMessage).toContain('redundant');
    });
  });

  describe('Scenario: System-reminder provides concrete example of step comment placement', () => {
    it('should provide a concrete example showing proper placement', () => {
      // Given step validation fails for a missing step comment
      const validationResult: ValidationResult = {
        valid: false,
        matches: [
          {
            featureStep: 'When I run the finalize command',
            testComment: null,
            matched: false,
          },
        ],
        missingSteps: ['When I run the finalize command'],
        unmatchedComments: [],
      };

      // When the system-reminder is generated
      const errorMessage = formatValidationError(validationResult, 'story');

      // Then the reminder should include an example showing proper placement
      expect(errorMessage).toContain('Example:');

      // And the example should reference the specific code line that executes the step
      expect(errorMessage).toContain('const result = await');
      expect(errorMessage).toContain('// @step');

      // And the example should show the @step comment placed immediately before the executing code
      expect(errorMessage).toContain('right before');
      expect(errorMessage).toContain('line that');
    });
  });
});
