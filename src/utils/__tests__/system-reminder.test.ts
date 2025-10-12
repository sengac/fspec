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
      expect(reminder).toContain('Write FAILING tests BEFORE any implementation code');
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

  describe('Scenario: No reminder when moving to done status', () => {
    it('should return null for done status', () => {
      // Given: Work unit UI-001 moving to done status
      const workUnitId = 'UI-001';
      const newStatus = 'done';

      // When: Getting status change reminder
      const reminder = getStatusChangeReminder(workUnitId, newStatus);

      // Then: Should return null (no reminder)
      expect(reminder).toBeNull();
    });
  });

  describe('Scenario: Remind about Fibonacci scale when estimate is missing', () => {
    it('should return reminder about Fibonacci scale when estimate is missing', () => {
      // Given: Work unit UI-001 with no estimate
      const workUnitId = 'UI-001';
      const hasEstimate = false;

      // When: Getting missing estimate reminder
      const reminder = getMissingEstimateReminder(workUnitId, hasEstimate);

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

      // When: Getting missing estimate reminder
      const reminder = getMissingEstimateReminder(workUnitId, hasEstimate);

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
      expect(wrapped).toMatch(/^<system-reminder>\n[\s\S]+\n<\/system-reminder>$/);
    });
  });
});
