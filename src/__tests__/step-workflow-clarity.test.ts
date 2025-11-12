/**
 * Feature: spec/features/improve-step-workflow-clarity-to-prevent-wrong-test-file-and-missing-comments.feature
 *
 * Tests for improving @step workflow clarity to prevent AI agents from:
 * - Missing @step requirement buried in long system-reminders
 * - Adding @step comments to wrong test files
 * - Creating tests without @step comments then fixing retroactively
 */

import { describe, it, expect } from 'vitest';
import { getStatusChangeReminder } from '../utils/system-reminder';
import {
  formatValidationError,
  type ValidationResult,
} from '../utils/step-validation';

describe('Feature: Improve @step workflow clarity', () => {
  describe('Scenario: Testing state reminder makes @step requirement prominent', () => {
    // @step Given work unit moves to testing state
    // @step When system-reminder is shown
    // @step Then @step requirement should be in FIRST 10 lines of reminder
    it('should show @step requirement in first 10 lines of testing state reminder', async () => {
      const reminder = await getStatusChangeReminder('TEST-001', 'testing');
      expect(reminder).not.toBeNull();

      if (!reminder) {
        throw new Error('Reminder should not be null');
      }

      // Extract the reminder content (strip <system-reminder> tags)
      const reminderContent = reminder
        .replace(/<system-reminder>\n/, '')
        .replace(/\n<\/system-reminder>/, '');

      const lines = reminderContent.split('\n');

      // Check that @step is mentioned in first 10 lines
      const first10Lines = lines.slice(0, 10).join('\n');
      expect(first10Lines).toContain('@step');
    });

    // @step And requirement should emphasize ONE scenario = ONE test mapping
    it('should emphasize ONE scenario = ONE test mapping in reminder', async () => {
      const reminder = await getStatusChangeReminder('TEST-001', 'testing');
      expect(reminder).not.toBeNull();

      if (!reminder) {
        throw new Error('Reminder should not be null');
      }

      // Should mention the ONE-to-ONE mapping concept
      expect(reminder).toContain('ONE');
    });

    // @step And reminder should state @step comments added DURING test writing
    it('should state @step comments added DURING test writing', async () => {
      const reminder = await getStatusChangeReminder('TEST-001', 'testing');
      expect(reminder).not.toBeNull();

      if (!reminder) {
        throw new Error('Reminder should not be null');
      }

      // Should mention that @step comments are required/mandatory
      expect(reminder.toLowerCase()).toContain('mandatory');
    });
  });

  describe('Scenario: Validation error shows which test file is being checked', () => {
    // @step Given test file missing @step comments
    // @step When link-coverage validation fails
    // @step Then error message should show test file path being validated
    it('should show test file path in validation error message', () => {
      const validationResult: ValidationResult = {
        valid: false,
        matches: [],
        missingSteps: [
          'Given I am on the login page',
          'When I enter valid credentials',
        ],
        unmatchedComments: [],
      };

      const errorMessage = formatValidationError(validationResult, 'story');

      // Error should mention test file
      expect(errorMessage).toContain('test file');
    });

    // @step And error should warn about adding @step to CORRECT test file
    it('should warn about adding @step to CORRECT test file', () => {
      const validationResult: ValidationResult = {
        valid: false,
        matches: [],
        missingSteps: ['Given I am on the login page'],
        unmatchedComments: [],
      };

      const errorMessage = formatValidationError(validationResult, 'story');

      // Should emphasize correct placement
      expect(errorMessage).toContain('NEAR');
      expect(errorMessage).toContain('right before');
    });
  });

  describe('Scenario: Reminder guides recreation when tests created without @step comments', () => {
    // @step Given tests were created without @step comments
    // @step When validation error occurs
    // @step Then error should suggest DELETE and RECREATE tests ONLY if created in current work unit
    it('should suggest DELETE and RECREATE for tests created in current work unit', () => {
      const validationResult: ValidationResult = {
        valid: false,
        matches: [],
        missingSteps: ['When I run the finalize command'],
        unmatchedComments: [],
      };

      const errorMessage = formatValidationError(validationResult, 'story');

      // Should suggest deletion and recreation for new tests
      expect(errorMessage).toContain('DELETE');
      expect(errorMessage).toContain('RECREATE');
      expect(errorMessage).toContain('current work unit');
    });

    // @step And error should explain recreation is better than editing for structural issues
    it('should explain recreation is better than editing for structural issues', () => {
      const validationResult: ValidationResult = {
        valid: false,
        matches: [],
        missingSteps: ['When I run the finalize command'],
        unmatchedComments: [],
      };

      const errorMessage = formatValidationError(validationResult, 'story');

      // Should explain why recreation is better
      expect(errorMessage).toContain('recreation');
      expect(errorMessage).toContain('structural');
    });

    // @step And error should state existing tests from other cards should use checkpoint restore instead
    it('should state existing tests from other cards should use checkpoint restore', () => {
      const validationResult: ValidationResult = {
        valid: false,
        matches: [],
        missingSteps: ['Given I am on the login page'],
        unmatchedComments: [],
      };

      const errorMessage = formatValidationError(validationResult, 'story');

      // Should mention checkpoint restore for existing tests
      expect(errorMessage).toContain('checkpoint');
      expect(errorMessage).toContain('restore');
      expect(errorMessage).toContain('other');
    });
  });
});
