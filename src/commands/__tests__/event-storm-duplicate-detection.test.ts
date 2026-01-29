/**
 * Feature: spec/features/event-storm-commands-allow-duplicate-entries-without-validation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { addDomainEvent } from '../add-domain-event';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Event Storm commands allow duplicate entries without validation', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('event-storm-duplicate-detection');

    // Initialize with work units that have event storm capabilities
    const workUnitsData = {
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          type: 'story',
          title: 'Test Story',
          status: 'specifying',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [
            {
              state: 'specifying',
              timestamp: new Date().toISOString(),
            },
          ],
          eventStorm: {
            level: 'process_modeling',
            items: [],
            nextItemId: 0,
          },
        },
        'TEST-002': {
          id: 'TEST-002',
          type: 'story',
          title: 'Test Story 2',
          status: 'specifying',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [
            {
              state: 'specifying',
              timestamp: new Date().toISOString(),
            },
          ],
          eventStorm: {
            level: 'process_modeling',
            items: [],
            nextItemId: 0,
          },
        },
        'TEST-003': {
          id: 'TEST-003',
          type: 'story',
          title: 'Test Story 3',
          status: 'specifying',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [
            {
              state: 'specifying',
              timestamp: new Date().toISOString(),
            },
          ],
          eventStorm: {
            level: 'process_modeling',
            items: [],
            nextItemId: 0,
          },
        },
      },
      states: {
        backlog: [],
        specifying: ['TEST-001', 'TEST-002', 'TEST-003'],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
    };

    await writeJsonTestFile(setup.workUnitsFile, workUnitsData);
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Prevent duplicate domain event', () => {
    it('should throw error when adding duplicate event with exact text match', async () => {
      // @step Given a work unit "TEST-001" exists with Event Storm
      // Work unit already created in beforeEach

      // @step And I add domain event "EventA"
      await addDomainEvent({
        workUnitId: 'TEST-001',
        text: 'EventA',
        cwd: setup.testDir,
      });

      // @step When I try to add domain event "EventA" again
      const result = await addDomainEvent({
        workUnitId: 'TEST-001',
        text: 'EventA',
        cwd: setup.testDir,
      });

      // @step Then an error should be thrown
      // @step And the error should say "Event 'EventA' already exists"
      expect(result.success).toBe(false);
      expect(result.error).toMatch(/Event 'EventA' already exists/);
    });
  });

  describe('Scenario: Prevent duplicate with case-insensitive check', () => {
    it('should throw error when adding duplicate event with different case', async () => {
      // @step Given a work unit "TEST-002" exists with Event Storm
      // Work unit already created in beforeEach

      // @step And I add domain event "EventA"
      await addDomainEvent({
        workUnitId: 'TEST-002',
        text: 'EventA',
        cwd: setup.testDir,
      });

      // @step When I try to add domain event "eventa" (lowercase)
      const result = await addDomainEvent({
        workUnitId: 'TEST-002',
        text: 'eventa',
        cwd: setup.testDir,
      });

      // @step Then an error should be thrown
      // @step And the error should say "Event 'eventa' already exists"
      expect(result.success).toBe(false);
      expect(result.error).toMatch(/Event 'eventa' already exists/);
    });
  });

  describe('Scenario: Allow same text after deletion', () => {
    it('should allow adding event with same text after original is deleted', async () => {
      // @step Given a work unit "TEST-003" exists with Event Storm
      // Work unit already created in beforeEach

      // @step And I add domain event "EventA"
      await addDomainEvent({
        workUnitId: 'TEST-003',
        text: 'EventA',
        cwd: setup.testDir,
      });

      // @step And I delete the event
      const workUnitsData = await readJsonTestFile(setup.workUnitsFile);
      workUnitsData.workUnits['TEST-003'].eventStorm.items[0].deleted = true;
      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I try to add domain event "EventA" again
      const result = await addDomainEvent({
        workUnitId: 'TEST-003',
        text: 'EventA',
        cwd: setup.testDir,
      });

      // @step Then the command should succeed
      // @step And a new event should be created
      expect(result.success).toBe(true);
      expect(result.eventId).toBeDefined();

      // Verify a new event was created
      const updatedData = await readJsonTestFile(setup.workUnitsFile);
      const events = updatedData.workUnits['TEST-003'].eventStorm.items;
      expect(events.length).toBe(2);
      expect(events[0].deleted).toBe(true);
      expect(events[1].text).toBe('EventA');
      expect(events[1].deleted).toBe(false);
    });
  });
});
