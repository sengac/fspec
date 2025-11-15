/**
 * Feature: spec/features/multiple-consecutive-system-reminder-blocks-in-update-work-unit-status.feature
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Multiple consecutive system-reminder blocks in update-work-unit-status', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec/features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Multiple reminders combined into single block', () => {
    it('should combine multiple reminders into single system-reminder block', async () => {
      // @step Given multiple system reminders are applicable for a status transition
      const workUnitsData = {
        version: '0.8.13',
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Test Work Unit',
            description: 'Test description',
            type: 'story' as const,
            status: 'backlog' as const,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['TEST-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step And each reminder is individually wrapped in system-reminder tags
      // (This is handled internally by helper functions in update-work-unit-status)

      // @step When the update-work-unit-status command processes the reminders
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step Then all reminder content should be in a single system-reminder block
      expect(result.systemReminder).toBeDefined();
      const reminderMatches =
        result.systemReminder?.match(/<system-reminder>/g);
      expect(reminderMatches?.length).toBe(1);

      // @step And there should be no consecutive system-reminder blocks in the output
      expect(result.systemReminder).not.toMatch(
        /<\/system-reminder>\s*<system-reminder>/
      );
    });
  });

  describe('Scenario: Status transition to done with multiple applicable reminders', () => {
    it('should combine status, cleanup, and review reminders into single block', async () => {
      // @step Given a work unit is transitioning to done status
      const workUnitsData = {
        version: '0.8.13',
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            prefix: 'TEST',
            title: 'Test Work Unit',
            description: 'Test description',
            type: 'story' as const,
            status: 'validating' as const,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            // @step And the virtual hooks cleanup reminder is applicable
            virtualHooks: [
              {
                name: 'test-hook',
                event: 'post-implementing' as const,
                command: 'echo test',
                blocking: false,
                gitContext: false,
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['TEST-002'],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create feature file to link
      const featureContent = `@TEST-002
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test
    So that it works

  Scenario: Test scenario
    Given a test condition
    When I do something
    Then it should work
`;

      await writeFile(
        join(testDir, 'spec/features/test-feature.feature'),
        featureContent
      );

      // @step And the status change reminder is applicable
      // @step And the quality check review reminder is applicable
      // (Both are automatically triggered when transitioning to done)

      // @step When the status update is executed
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-002',
        status: 'done',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step Then a single system-reminder block should contain all three reminders
      expect(result.systemReminder).toBeDefined();
      const reminderMatches =
        result.systemReminder?.match(/<system-reminder>/g);
      expect(reminderMatches?.length).toBe(1);

      // Verify it contains done status message
      expect(result.systemReminder).toContain('DONE status');

      // Verify it contains virtual hooks cleanup message
      expect(result.systemReminder).toContain('virtual hook');

      // Verify it contains quality check review message
      expect(result.systemReminder).toContain('QUALITY CHECK OPPORTUNITY');

      // @step And the reminders should be separated by blank lines within the block
      const reminderContent = result.systemReminder?.match(
        /<system-reminder>([\s\S]*?)<\/system-reminder>/
      );
      expect(reminderContent).toBeTruthy();
      if (reminderContent) {
        // Should have blank lines between different reminder sections
        expect(reminderContent[1]).toMatch(/\n\n/);
      }
    });
  });
});
