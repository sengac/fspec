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

      // @step And the reminder should contain "Unit tests pass"
      expect(reminder).toContain('Unit tests pass');

      // @step And the reminder should contain "Call sites connected"
      expect(reminder).toContain('Call sites connected');

      // @step And the reminder should contain "Feature works end-to-end"
      expect(reminder).toContain('Feature works end-to-end');
    });
  });

  describe('Scenario: IMPLEMENTING phase separates STAY IN SCOPE from INTEGRATION IS NOT SCOPE CREEP', () => {
    it('should contain both STAY IN SCOPE and INTEGRATION IS NOT SCOPE CREEP', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the reminder should contain "STAY IN SCOPE"
      expect(reminder).toContain('STAY IN SCOPE');

      // @step And the reminder should contain "INTEGRATION IS NOT SCOPE CREEP"
      expect(reminder).toContain('INTEGRATION IS NOT SCOPE CREEP');
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

  describe('Scenario: IMPLEMENTING phase next steps include integration verification', () => {
    it('should include integration verification in next steps', async () => {
      // @step Given a work unit transitions to IMPLEMENTING status
      const workUnitId = 'TEST-001';
      const newStatus = 'implementing';

      // @step When the status change reminder is generated
      const reminder = await getStatusChangeReminder(workUnitId, newStatus);

      // @step Then the suggested next steps should include wiring up integration points
      expect(reminder).toMatch(/wire.*integration|integration.*wire/i);

      // @step And the suggested next steps should include verifying feature works end-to-end
      expect(reminder).toMatch(/verify.*end-to-end|end-to-end.*verify/i);
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
