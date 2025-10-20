/**
 * Feature: spec/features/implement-interactive-questionnaire.feature
 *
 * This test file validates the acceptance criteria for interactive questionnaire.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import {
  runQuestionnaire,
  type QuestionnaireOptions,
} from '../interactive-questionnaire';

describe('Feature: Implement Interactive Questionnaire', () => {
  describe('Scenario: Display vision question with help text and example', () => {
    it('should display vision question with help text and example answer', () => {
      // Given I run questionnaire in interactive mode
      const options: QuestionnaireOptions = {
        mode: 'interactive',
      };

      // When questionnaire displays vision section question
      const questionnaire = runQuestionnaire(options);
      const display = questionnaire.formatQuestion(0);

      // Then question should show 'What is the core purpose?'
      expect(display).toContain('What is the core purpose?');

      // And question should include HELP text with elevator pitch guidance
      expect(display).toContain('[HELP:');
      expect(display).toContain('elevator pitch');

      // And question should include example answer
      expect(display).toContain('[Example:');
      expect(display).toContain('fspec helps AI agents follow ACDD workflow');
    });
  });

  describe('Scenario: Display prefilled answer with DETECTED tag from code analysis', () => {
    it('should show prefilled answer with DETECTED tag and Keep/Edit/Skip options', () => {
      // Given I run questionnaire with --from-discovery option
      // And code analysis detected primary user as 'Developer using CLI'
      const options: QuestionnaireOptions = {
        mode: 'from-discovery',
        discoveryData: {
          personas: ['Developer using CLI'],
        },
      };

      // When questionnaire displays personas question
      const questionnaire = runQuestionnaire(options);
      const personasQuestionIndex = questionnaire.questions.findIndex(
        q => q.id === 'primary-users'
      );
      const display = questionnaire.formatQuestion(personasQuestionIndex);

      // Then answer should be prefilled with detected value
      expect(display).toContain('Developer using CLI');

      // And answer should show [DETECTED] tag
      expect(display).toContain('[DETECTED]');

      // And user should have Keep/Edit/Skip options
      expect(display).toContain('Keep/Edit/Skip');
    });
  });

  describe('Scenario: Show progress indicator for each question', () => {
    it('should display progress indicator showing current question number', () => {
      // Given I am answering the third question out of 15 total
      const options: QuestionnaireOptions = {
        mode: 'interactive',
      };

      // When questionnaire displays the question
      const questionnaire = runQuestionnaire(options);
      const totalQuestions = questionnaire.questions.length;
      const display = questionnaire.formatQuestion(2); // Third question (index 2)

      // Then UI should show 'Question 3 of 15'
      expect(display).toContain(`Question 3 of ${totalQuestions}`);

      // And progress indicator should update with each question
      const firstQuestionDisplay = questionnaire.formatQuestion(0);
      expect(firstQuestionDisplay).toContain(`Question 1 of ${totalQuestions}`);
    });
  });

  describe('Scenario: Validate non-empty answer before accepting', () => {
    it('should reject empty answers and display validation error', () => {
      // Given I am answering a required question
      const options: QuestionnaireOptions = {
        mode: 'interactive',
      };
      const questionnaire = runQuestionnaire(options);
      const requiredQuestion = questionnaire.questions.find(q => q.required);

      // When I submit an empty answer
      const validation = questionnaire.validate('', requiredQuestion!.id);

      // Then questionnaire should reject the answer
      expect(validation.valid).toBe(false);

      // And questionnaire should display validation error message
      expect(validation.error).toBeDefined();
      expect(validation.error).toContain('requires an answer');

      // And questionnaire should prompt me to answer again
      // (Validation failure prompts re-entry in actual UI)
      expect(validation.error).toContain('Please provide a response');
    });
  });
});
