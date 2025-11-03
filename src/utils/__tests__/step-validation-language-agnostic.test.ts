/**
 * Feature: spec/features/step-validation-hardcoded-to-javascript-comment-syntax-should-be-language-agnostic.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { extractStepComments } from '../step-validation';

describe('Feature: @step validation hardcoded to JavaScript comment syntax, should be language-agnostic', () => {
  describe('Scenario: Extract @step comment from JavaScript-style line comment', () => {
    it('should extract step comment with correct keyword and text', () => {
      // @step Given a test file contains the line "// @step Given a user is authenticated"
      const testContent = '// @step Given a user is authenticated';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "Given"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('Given');

      // @step And the step text should be "a user is authenticated"
      expect(stepComments[0].text).toBe('a user is authenticated');
    });
  });

  describe('Scenario: Extract @step comment from Python-style line comment', () => {
    it('should extract step comment from # prefix', () => {
      // @step Given a test file contains the line "# @step When I click the button"
      const testContent = '# @step When I click the button';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "When"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('When');

      // @step And the step text should be "I click the button"
      expect(stepComments[0].text).toBe('I click the button');
    });
  });

  describe('Scenario: Extract @step comment from SQL-style line comment', () => {
    it('should extract step comment from -- prefix', () => {
      // @step Given a test file contains the line "-- @step Then I see the result"
      const testContent = '-- @step Then I see the result';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "Then"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('Then');

      // @step And the step text should be "I see the result"
      expect(stepComments[0].text).toBe('I see the result');
    });
  });

  describe('Scenario: Extract @step comment from block comment', () => {
    it('should extract step comment from /* */ block comment', () => {
      // @step Given a test file contains the line "/* @step Given a user is authenticated */"
      const testContent = '/* @step Given a user is authenticated */';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "Given"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('Given');

      // @step And the step text should be "a user is authenticated"
      expect(stepComments[0].text).toBe('a user is authenticated');
    });
  });

  describe('Scenario: Extract @step comment from MATLAB-style line comment', () => {
    it('should extract step comment from % prefix', () => {
      // @step Given a test file contains the line "% @step And the database is updated"
      const testContent = '% @step And the database is updated';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "And"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('And');

      // @step And the step text should be "the database is updated"
      expect(stepComments[0].text).toBe('the database is updated');
    });
  });

  describe('Scenario: Extract @step comment from Visual Basic-style line comment', () => {
    it("should extract step comment from ' prefix", () => {
      // @step Given a test file contains the line "' @step But the error is logged"
      const testContent = "' @step But the error is logged";

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "But"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('But');

      // @step And the step text should be "the error is logged"
      expect(stepComments[0].text).toBe('the error is logged');
    });
  });

  describe('Scenario: Match @step anywhere in the line, not just at the start', () => {
    it('should extract step comment with leading whitespace', () => {
      // @step Given a test file contains the line "  // @step Given I am logged in"
      const testContent = '  // @step Given I am logged in';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "Given"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('Given');

      // @step And the step text should be "I am logged in"
      expect(stepComments[0].text).toBe('I am logged in');
    });
  });

  describe('Scenario: Ignore trailing comment delimiters from block comments', () => {
    it('should extract step text without trailing delimiters', () => {
      // @step Given a test file contains the line "/* @step When I submit the form */ // trailing comment"
      const testContent =
        '/* @step When I submit the form */ // trailing comment';

      // @step When the extractStepComments function processes the file
      const stepComments = extractStepComments(testContent);

      // @step Then it should extract a step comment with keyword "When"
      expect(stepComments).toHaveLength(1);
      expect(stepComments[0].keyword).toBe('When');

      // @step And the step text should be "I submit the form"
      expect(stepComments[0].text).toBe('I submit the form');

      // @step And the step text should not contain "*/"
      expect(stepComments[0].text).not.toContain('*/');
    });
  });
});
