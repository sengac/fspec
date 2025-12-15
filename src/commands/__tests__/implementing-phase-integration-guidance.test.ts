/**
 * Feature: spec/features/reword-implementing-phase-guidance-to-prevent-llms-skipping-integration-work.feature
 *
 * Tests for REMIND-015: Reword IMPLEMENTING phase guidance to prevent LLMs skipping integration work
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { getStatusChangeReminder } from '../../utils/system-reminder';

describe('Feature: Reword IMPLEMENTING phase guidance to prevent LLMs skipping integration work', () => {
  beforeEach(() => {
    // Ensure reminders are enabled
    delete process.env.FSPEC_DISABLE_REMINDERS;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: IMPLEMENTING phase shows CREATION + CONNECTION prominently', () => {
    it('should contain IMPLEMENTATION = CREATION + CONNECTION', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should contain "IMPLEMENTATION = CREATION + CONNECTION"
      expect(reminder).toContain('IMPLEMENTATION = CREATION + CONNECTION');
    });
  });

  describe('Scenario: IMPLEMENTING phase includes WHO CALLS THIS heuristic', () => {
    it('should contain WHO CALLS THIS?', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should contain "WHO CALLS THIS?"
      expect(reminder).toContain('WHO CALLS THIS?');
    });
  });

  describe('Scenario: IMPLEMENTING phase lists COMPLETE MEANS checklist', () => {
    it('should contain COMPLETE MEANS checklist with all items', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should contain "COMPLETE MEANS:"
      expect(reminder).toContain('COMPLETE MEANS:');

      // @step And the reminder should contain "connected to the system"
      expect(reminder).toContain('connected to the system');

      // @step And the reminder should contain "Feature works end-to-end"
      expect(reminder).toContain('Feature works end-to-end');
    });
  });

  describe('Scenario: IMPLEMENTING phase emphasizes CREATION + CONNECTION pattern', () => {
    it('should emphasize that code must be connected, not just created', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should contain "CREATION + CONNECTION"
      expect(reminder).toContain('CREATION + CONNECTION');

      // @step And the reminder should warn about code that exists but isn't connected
      expect(reminder).toContain("Code that exists but isn't connected");
    });
  });

  describe('Scenario: IMPLEMENTING phase removes minimization-encouraging phrases', () => {
    it('should not contain minimization-encouraging phrases', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should not contain "ONLY enough"
      expect(reminder).not.toContain('ONLY enough');

      // @step And the reminder should not contain "minimum code"
      expect(reminder).not.toContain('minimum code');

      // @step And the reminder should not contain "minimal code"
      expect(reminder).not.toContain('minimal code');

      // @step And the reminder should not contain "Avoid over-implementation"
      expect(reminder).not.toContain('Avoid over-implementation');
    });
  });

  describe('Scenario: IMPLEMENTING phase emphasizes wire it up message', () => {
    it('should include wire it up guidance', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should include "Wire it up"
      expect(reminder).toContain('Wire it up');

      // @step And the reminder should mention feature works end-to-end
      expect(reminder).toContain('end-to-end');
    });
  });

  describe('Scenario: SPECIFYING phase includes WHO CALLS THIS prompt for integration points', () => {
    it('should contain WHO CALLS THIS? in specifying phase', async () => {
      // @step Given a work unit transitions to SPECIFYING status
      const workUnitId = 'TEST-001';
      const newStatus = 'specifying';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should contain "WHO CALLS THIS?"
      expect(reminder).toContain('WHO CALLS THIS?');

      // @step And the reminder should prompt for integration scenarios
      expect(reminder).toMatch(/integration|wired|connected/i);
    });
  });
});
