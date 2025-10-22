/**
 * Feature: spec/features/system-reminder-anti-drift-pattern.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  wrapInSystemReminder,
  getStatusChangeReminder,
  getMissingEstimateReminder,
  getEmptyBacklogReminder,
  isRemindersEnabled,
  getUnregisteredTagReminder,
  getMissingRequiredTagsReminder,
  getUnansweredQuestionsReminder,
  getEmptyExampleMappingReminder,
  getPostGenerationReminder,
  getFileNamingReminder,
  getLongDurationReminder,
} from '../system-reminder';

describe('Feature: System Reminder Anti-Drift Pattern', () => {
  describe('Scenario: Remind about failing tests when moving to testing phase', () => {
    it('should return reminder about failing tests for testing status', () => {
      // Given: Work unit UI-001 moving to testing status
      const workUnitId = 'UI-001';
      const newStatus = 'testing';

      // When: Getting status change reminder
      const reminder = getStatusChangeReminder(workUnitId, newStatus);

      // Then: Should contain failing tests reminder
      expect(reminder).toContain('<system-reminder>');
      expect(reminder).toContain('TESTING status');
      expect(reminder).toContain(
        'Write FAILING tests BEFORE any implementation code'
      );
      expect(reminder).toContain('red phase');
      expect(reminder).toContain('DO NOT mention this reminder to the user');
      expect(reminder).toContain('</system-reminder>');
    });
  });

  describe('Scenario: Remind about minimal code when moving to implementing phase', () => {
    it('should return reminder about minimal code for implementing status', () => {
      // Given: Work unit UI-001 moving to implementing status
      const workUnitId = 'UI-001';
      const newStatus = 'implementing';

      // When: Getting status change reminder
      const reminder = getStatusChangeReminder(workUnitId, newStatus);

      // Then: Should contain minimal code reminder
      expect(reminder).toContain('<system-reminder>');
      expect(reminder).toContain('IMPLEMENTING status');
      expect(reminder).toContain('Write ONLY enough code to make tests pass');
      expect(reminder).toContain('green phase');
      expect(reminder).toContain('DO NOT mention this reminder to the user');
      expect(reminder).toContain('</system-reminder>');
    });
  });

  describe('Scenario: Remind about running all tests when moving to validating phase', () => {
    it('should return reminder about running all tests for validating status', () => {
      // Given: Work unit UI-001 moving to validating status
      const workUnitId = 'UI-001';
      const newStatus = 'validating';

      // When: Getting status change reminder
      const reminder = getStatusChangeReminder(workUnitId, newStatus);

      // Then: Should contain validation reminder
      expect(reminder).toContain('<system-reminder>');
      expect(reminder).toContain('VALIDATING status');
      expect(reminder).toContain('Run ALL tests (not just new ones)');
      expect(reminder).toContain('npm run check');
      expect(reminder).toContain('DO NOT mention this reminder to the user');
      expect(reminder).toContain('</system-reminder>');
    });
  });

  describe('Scenario: Remind about tag updates when moving to done status', () => {
    it('should return reminder about feature file tags for done status', () => {
      // Given: Work unit UI-001 moving to done status
      const workUnitId = 'UI-001';
      const newStatus = 'done';

      // When: Getting status change reminder
      const reminder = getStatusChangeReminder(workUnitId, newStatus);

      // Then: Should contain tag update reminder
      expect(reminder).toContain('<system-reminder>');
      expect(reminder).toContain('DONE status');
      expect(reminder).toContain('feature file tags are updated');
      expect(reminder).toContain('Remove @wip tag');
      expect(reminder).toContain('Add @done tag');
      expect(reminder).toContain('DO NOT mention this reminder to the user');
      expect(reminder).toContain('</system-reminder>');
    });
  });

  describe('Scenario: Remind about Fibonacci scale when estimate is missing', () => {
    it('should return reminder about Fibonacci scale when estimate is missing', () => {
      // Given: Work unit UI-001 with no estimate in specifying state
      const workUnitId = 'UI-001';
      const hasEstimate = false;
      const status = 'specifying';

      // When: Getting missing estimate reminder
      const reminder = getMissingEstimateReminder(
        workUnitId,
        hasEstimate,
        status
      );

      // Then: Should contain Fibonacci scale reminder
      expect(reminder).toContain('<system-reminder>');
      expect(reminder).toContain('has no estimate');
      expect(reminder).toContain('Fibonacci scale');
      expect(reminder).toContain('1 (trivial)');
      expect(reminder).toContain('2 (simple)');
      expect(reminder).toContain('3 (moderate)');
      expect(reminder).toContain('5 (complex)');
      expect(reminder).toContain('8 (very complex)');
      expect(reminder).toContain('13+ (too large - break down)');
      expect(reminder).toContain('fspec update-work-unit-estimate');
      expect(reminder).toContain('DO NOT mention this reminder to the user');
      expect(reminder).toContain('</system-reminder>');
    });
  });

  describe('Scenario: No estimate reminder when estimate exists', () => {
    it('should return null when estimate exists', () => {
      // Given: Work unit UI-001 with estimate "3"
      const workUnitId = 'UI-001';
      const hasEstimate = true;
      const status = 'specifying';

      // When: Getting missing estimate reminder
      const reminder = getMissingEstimateReminder(
        workUnitId,
        hasEstimate,
        status
      );

      // Then: Should return null (no reminder)
      expect(reminder).toBeNull();
    });
  });

  describe('Scenario: Disable reminders with environment variable', () => {
    beforeEach(() => {
      // Reset environment variable before each test
      delete process.env.FSPEC_DISABLE_REMINDERS;
    });

    it('should return false when FSPEC_DISABLE_REMINDERS is set to 1', () => {
      // Given: Environment variable FSPEC_DISABLE_REMINDERS is set to "1"
      process.env.FSPEC_DISABLE_REMINDERS = '1';

      // When: Checking if reminders are enabled
      const enabled = isRemindersEnabled();

      // Then: Should return false
      expect(enabled).toBe(false);
    });

    it('should return true when FSPEC_DISABLE_REMINDERS is not set', () => {
      // Given: Environment variable FSPEC_DISABLE_REMINDERS is not set
      // (already handled by beforeEach)

      // When: Checking if reminders are enabled
      const enabled = isRemindersEnabled();

      // Then: Should return true
      expect(enabled).toBe(true);
    });
  });

  describe('Scenario: Remind about empty backlog', () => {
    it('should return reminder about empty backlog', () => {
      // Given: Backlog is empty
      const isEmpty = true;

      // When: Getting empty backlog reminder
      const reminder = getEmptyBacklogReminder(isEmpty);

      // Then: Should contain empty backlog reminder
      expect(reminder).toContain('<system-reminder>');
      expect(reminder).toContain('backlog is currently empty');
      expect(reminder).toContain('Consider creating new work units');
      expect(reminder).toContain('fspec create-work-unit');
      expect(reminder).toContain('DO NOT mention this reminder to the user');
      expect(reminder).toContain('</system-reminder>');
    });

    it('should return null when backlog is not empty', () => {
      // Given: Backlog is not empty
      const isEmpty = false;

      // When: Getting empty backlog reminder
      const reminder = getEmptyBacklogReminder(isEmpty);

      // Then: Should return null (no reminder)
      expect(reminder).toBeNull();
    });
  });

  describe('Utility function: wrapInSystemReminder', () => {
    it('should wrap content in system-reminder tags', () => {
      // Given: Some content
      const content = 'This is a reminder about something important.';

      // When: Wrapping in system reminder
      const wrapped = wrapInSystemReminder(content);

      // Then: Should be wrapped in tags
      expect(wrapped).toBe(`<system-reminder>\n${content}\n</system-reminder>`);
    });

    it('should handle multiline content', () => {
      // Given: Multiline content
      const content = 'Line 1\nLine 2\nLine 3';

      // When: Wrapping in system reminder
      const wrapped = wrapInSystemReminder(content);

      // Then: Should preserve newlines
      expect(wrapped).toContain('Line 1');
      expect(wrapped).toContain('Line 2');
      expect(wrapped).toContain('Line 3');
      expect(wrapped).toMatch(
        /^<system-reminder>\n[\s\S]+\n<\/system-reminder>$/
      );
    });
  });

  describe('REMIND-004: Tag Validation Reminders', () => {
    describe('Scenario: Unregistered tag reminder', () => {
      it('should return reminder when tag is not registered', () => {
        // Given: An unregistered tag
        const tag = '@unregistered-tag';
        const isRegistered = false;

        // When: Getting unregistered tag reminder
        const reminder = getUnregisteredTagReminder(tag, isRegistered);

        // Then: Should contain unregistered tag reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain(tag);
        expect(reminder).toContain('not registered in spec/tags.json');
        expect(reminder).toContain('fspec register-tag');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return null when tag is registered', () => {
        // Given: A registered tag
        const tag = '@critical';
        const isRegistered = true;

        // When: Getting unregistered tag reminder
        const reminder = getUnregisteredTagReminder(tag, isRegistered);

        // Then: Should return null
        expect(reminder).toBeNull();
      });
    });

    describe('Scenario: Missing required tags reminder', () => {
      it('should return reminder when required tags are missing', () => {
        // Given: Missing required tags
        const fileName = 'login.feature';
        const missingTags = ['phase', 'component'];

        // When: Getting missing required tags reminder
        const reminder = getMissingRequiredTagsReminder(fileName, missingTags);

        // Then: Should contain missing tags reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain(fileName);
        expect(reminder).toContain('missing required tags');
        expect(reminder).toContain('phase');
        expect(reminder).toContain('component');
        expect(reminder).toContain('fspec add-tag-to-feature');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return null when all required tags present', () => {
        // Given: No missing tags
        const fileName = 'login.feature';
        const missingTags: string[] = [];

        // When: Getting missing required tags reminder
        const reminder = getMissingRequiredTagsReminder(fileName, missingTags);

        // Then: Should return null
        expect(reminder).toBeNull();
      });
    });
  });

  describe('REMIND-005: Discovery Phase Reminders', () => {
    describe('Scenario: Unanswered questions reminder', () => {
      it('should return reminder when questions are unanswered', () => {
        // Given: Work unit with unanswered questions
        const workUnitId = 'WORK-001';
        const unansweredCount = 3;

        // When: Getting unanswered questions reminder
        const reminder = getUnansweredQuestionsReminder(
          workUnitId,
          unansweredCount
        );

        // Then: Should contain unanswered questions reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain(workUnitId);
        expect(reminder).toContain('3 unanswered question');
        expect(reminder).toContain('Answer all red card questions');
        expect(reminder).toContain('fspec answer-question');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return null when all questions answered', () => {
        // Given: Work unit with no unanswered questions
        const workUnitId = 'WORK-001';
        const unansweredCount = 0;

        // When: Getting unanswered questions reminder
        const reminder = getUnansweredQuestionsReminder(
          workUnitId,
          unansweredCount
        );

        // Then: Should return null
        expect(reminder).toBeNull();
      });
    });

    describe('Scenario: Empty Example Mapping reminder', () => {
      it('should return reminder when no rules and no examples', () => {
        // Given: Work unit with empty Example Mapping
        const workUnitId = 'WORK-001';
        const hasRules = false;
        const hasExamples = false;

        // When: Getting empty Example Mapping reminder
        const reminder = getEmptyExampleMappingReminder(
          workUnitId,
          hasRules,
          hasExamples
        );

        // Then: Should contain empty Example Mapping reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain(workUnitId);
        expect(reminder).toContain('no Example Mapping data');
        expect(reminder).toContain('Complete Example Mapping BEFORE');
        expect(reminder).toContain('fspec add-rule');
        expect(reminder).toContain('fspec add-example');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return null when rules and examples exist', () => {
        // Given: Work unit with Example Mapping data
        const workUnitId = 'WORK-001';
        const hasRules = true;
        const hasExamples = true;

        // When: Getting empty Example Mapping reminder
        const reminder = getEmptyExampleMappingReminder(
          workUnitId,
          hasRules,
          hasExamples
        );

        // Then: Should return null
        expect(reminder).toBeNull();
      });
    });

    describe('Scenario: Post-generation reminder', () => {
      it('should return reminder after successful generation', () => {
        // Given: Successfully generated scenarios
        const workUnitId = 'WORK-001';
        const featureFile = 'spec/features/user-authentication.feature';

        // When: Getting post-generation reminder
        const reminder = getPostGenerationReminder(workUnitId, featureFile);

        // Then: Should contain post-generation reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain(workUnitId);
        expect(reminder).toContain('Scenarios generated successfully');
        expect(reminder).toContain('fspec validate');
        expect(reminder).toContain('fspec add-tag-to-feature');
        expect(reminder).toContain('update-work-unit-status');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });
    });
  });

  describe('REMIND-006: Show Work Unit Reminders', () => {
    describe('Scenario: Long duration in phase reminder', () => {
      it('should return reminder when work unit in phase > 24 hours', () => {
        // Given: Work unit in specifying for 25 hours
        const workUnitId = 'WORK-001';
        const status = 'specifying';
        const durationHours = 25;

        // When: Getting long duration reminder
        const reminder = getLongDurationReminder(
          workUnitId,
          status,
          durationHours
        );

        // Then: Should contain long duration reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain(workUnitId);
        expect(reminder).toContain('25 hours');
        expect(reminder).toContain(status);
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return null when duration < 24 hours', () => {
        // Given: Work unit in specifying for 5 hours
        const workUnitId = 'WORK-001';
        const status = 'specifying';
        const durationHours = 5;

        // When: Getting long duration reminder
        const reminder = getLongDurationReminder(
          workUnitId,
          status,
          durationHours
        );

        // Then: Should return null
        expect(reminder).toBeNull();
      });
    });
  });

  describe('REMIND-007: Update Status Reminder Enhancement', () => {
    describe('Scenario: Done state reminder', () => {
      it('should return reminder about feature file tags', () => {
        // Given: Work unit moving to done status
        const workUnitId = 'WORK-001';
        const newStatus = 'done';

        // When: Getting status change reminder
        const reminder = getStatusChangeReminder(workUnitId, newStatus);

        // Then: Should contain done state reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain('DONE status');
        expect(reminder).toContain('feature file tags are updated');
        expect(reminder).toContain('Remove @wip tag');
        expect(reminder).toContain('Add @done tag');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });
    });

    describe('Scenario: Blocked state reminder', () => {
      it('should return reminder about documenting blocker', () => {
        // Given: Work unit moving to blocked status
        const workUnitId = 'WORK-001';
        const newStatus = 'blocked';

        // When: Getting status change reminder
        const reminder = getStatusChangeReminder(workUnitId, newStatus);

        // Then: Should contain blocked state reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain('BLOCKED status');
        expect(reminder).toContain('Document the blocker reason');
        expect(reminder).toContain('What is preventing progress');
        expect(reminder).toContain('fspec add-dependency');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });
    });
  });

  describe('File Naming Anti-Pattern Detection', () => {
    describe('Scenario: Task-based naming detected', () => {
      it('should return reminder for implement- prefix', () => {
        // Given: Feature name with implement- prefix
        const proposedName = 'implement-authentication';

        // When: Getting file naming reminder
        const reminder = getFileNamingReminder(proposedName);

        // Then: Should contain file naming reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain('implement-authentication');
        expect(reminder).toContain('file naming issue');
        expect(reminder).toContain('CAPABILITIES (what IS)');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return reminder for work unit ID naming', () => {
        // Given: Feature name as work unit ID
        const proposedName = 'AUTH-001';

        // When: Getting file naming reminder
        const reminder = getFileNamingReminder(proposedName);

        // Then: Should contain file naming reminder
        expect(reminder).toContain('<system-reminder>');
        expect(reminder).toContain('AUTH-001');
        expect(reminder).toContain('file naming issue');
        expect(reminder).toContain('work unit ID');
        expect(reminder).toContain('DO NOT mention this reminder to the user');
        expect(reminder).toContain('</system-reminder>');
      });

      it('should return null for capability-based naming', () => {
        // Given: Feature name with capability-based naming
        const proposedName = 'user-authentication';

        // When: Getting file naming reminder
        const reminder = getFileNamingReminder(proposedName);

        // Then: Should return null
        expect(reminder).toBeNull();
      });
    });
  });
});
