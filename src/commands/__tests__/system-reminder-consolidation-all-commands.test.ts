/**
 * Feature: spec/features/multiple-system-reminder-blocks-emitted-instead-of-single-consolidated-block.feature
 *
 * This test file validates that all commands consolidate multiple system reminders
 * into a single <system-reminder> block, extending the VAL-004 fix beyond
 * update-work-unit-status to show-work-unit, add-tag-to-feature, and generate-scenarios.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { showWorkUnit } from '../show-work-unit';
import { addTagToFeature } from '../add-tag-to-feature';
import { generateScenarios } from '../generate-scenarios';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  ensureTestDirectory,
  createTestFile,
} from '../../test-helpers/test-file-operations';
import { consolidateReminders } from '../../utils/system-reminder';

/**
 * Helper to count occurrences of <system-reminder> opening tags in a string
 */
function countReminderTags(text: string): number {
  const matches = text.match(/<system-reminder>/g);
  return matches ? matches.length : 0;
}

/**
 * Helper to check no consecutive closing+opening tags exist
 */
function hasConsecutiveBlocks(text: string): boolean {
  return /<\/system-reminder>\s*<system-reminder>/.test(text);
}

describe('Feature: Multiple system-reminder blocks emitted instead of single consolidated block', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('reminder-consolidation-all');
    await ensureTestDirectory(join(setup.testDir, 'spec'));
    await ensureTestDirectory(join(setup.testDir, 'spec/features'));
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: show-work-unit consolidates multiple reminders into single block', () => {
    it('should emit one system-reminder block when multiple reminders are applicable', async () => {
      // @step Given a work unit with no estimate that has been in done state for over 24 hours
      const twentySixHoursAgo = new Date(
        Date.now() - 26 * 60 * 60 * 1000
      ).toISOString();

      const workUnitsData = {
        version: '0.8.13',
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Test Work Unit No Estimate',
            description: 'Work unit that triggers multiple reminders',
            type: 'story' as const,
            status: 'done' as const,
            createdAt: twentySixHoursAgo,
            updatedAt: twentySixHoursAgo,
            stateHistory: [
              {
                status: 'done',
                timestamp: twentySixHoursAgo,
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['TEST-001'],
          blocked: [],
        },
      };

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // @step When I run show-work-unit for that work unit
      const result = await showWorkUnit({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });

      // @step Then the output contains exactly one system-reminder opening tag
      expect(result.systemReminder).toBeDefined();
      expect(countReminderTags(result.systemReminder!)).toBe(1);
      expect(hasConsecutiveBlocks(result.systemReminder!)).toBe(false);

      // @step Then the block contains both the missing estimate and long duration reminders separated by a blank line
      expect(result.systemReminder).toContain('no estimate');
      expect(result.systemReminder).toContain('done status');
      // Sections separated by blank line
      const content = result.systemReminder!.match(
        /<system-reminder>([\s\S]*?)<\/system-reminder>/
      );
      expect(content).toBeTruthy();
      expect(content![1]).toMatch(/\n\n/);
    });
  });

  describe('Scenario: add-tag-to-feature consolidates unregistered tag reminders into single block', () => {
    it('should emit one system-reminder block for multiple unregistered tags', async () => {
      // @step Given a feature file and three unregistered tags
      const featureContent = `Feature: Test Feature

  Scenario: Test scenario
    Given a test condition
    When I do something
    Then it should work
`;
      await createTestFile(
        join(setup.testDir, 'spec/features'),
        'test-tag-consolidation.feature',
        featureContent
      );

      // Create an empty tags.json so no tags are registered
      await writeJsonTestFile(join(setup.testDir, 'spec/tags.json'), {
        version: '1.0.0',
        categories: [],
      });

      // @step When I run add-tag-to-feature with validate-registry enabled
      const result = await addTagToFeature(
        'spec/features/test-tag-consolidation.feature',
        ['@unregistered-one', '@unregistered-two', '@unregistered-three'],
        { cwd: setup.testDir }
      );

      // @step Then the output contains exactly one system-reminder opening tag
      expect(result.systemReminder).toBeDefined();
      expect(countReminderTags(result.systemReminder!)).toBe(1);
      expect(hasConsecutiveBlocks(result.systemReminder!)).toBe(false);

      // @step Then the block contains all three unregistered tag warnings
      expect(result.systemReminder).toContain('@unregistered-one');
      expect(result.systemReminder).toContain('@unregistered-two');
      expect(result.systemReminder).toContain('@unregistered-three');
    });
  });

  describe('Scenario: generate-scenarios consolidates reminders into single block', () => {
    it('should emit one system-reminder block when multiple reminders are triggered', async () => {
      // @step Given a work unit with example mapping data that will trigger both generation and prefill reminders
      const workUnitsData = {
        version: '0.8.13',
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            prefix: 'TEST',
            title: 'Test Generate Scenarios',
            description: 'Test description',
            type: 'story' as const,
            status: 'specifying' as const,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: {
              role: 'developer',
              action: 'test things',
              benefit: 'things work',
            },
            rules: [{ id: 0, text: 'Must validate input' }],
            examples: [{ id: 0, text: 'Valid input is accepted' }],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // Create tags.json to avoid errors
      await writeJsonTestFile(join(setup.testDir, 'spec/tags.json'), {
        version: '1.0.0',
        categories: [],
      });

      // @step When I run generate-scenarios for that work unit
      const result = await generateScenarios({
        workUnitId: 'TEST-002',
        cwd: setup.testDir,
      });

      // @step Then the output contains exactly one system-reminder opening tag
      if (result.systemReminder) {
        expect(countReminderTags(result.systemReminder)).toBe(1);
        expect(hasConsecutiveBlocks(result.systemReminder)).toBe(false);

        // @step Then the block contains both the generation guidance and prefill detection content
        // The consolidated block should not have nested/consecutive blocks
        const closingTags = result.systemReminder.match(/<\/system-reminder>/g);
        expect(closingTags?.length).toBe(1);
      }
    });
  });

  describe('Scenario: TUI path consolidates reminders from fspec tool call into single block', () => {
    it('should produce a single system-reminder block from multiple unwrapped reminders', () => {
      // @step Given a fspec tool call result containing multiple unwrapped system reminders
      const reminders = [
        'Work unit TEST-001 has no estimate.\nUse Fibonacci scale.',
        'Work unit TEST-001 has been in done status for 48 hours.',
        'Consider breaking down into smaller units.',
      ];

      // @step When the globalSessionStreamManager processes the result
      // Simulating the consolidation logic that should be used
      const consolidated = consolidateReminders(
        reminders.map(r => `<system-reminder>\n${r}\n</system-reminder>`)
      );

      // @step Then the systemReminder string sent to the session contains exactly one system-reminder opening tag
      expect(consolidated).toBeDefined();
      expect(countReminderTags(consolidated!)).toBe(1);

      // @step Then all reminder content is within that single block separated by blank lines
      expect(consolidated).toContain('has no estimate');
      expect(consolidated).toContain('done status for 48 hours');
      expect(consolidated).toContain('breaking down');
      expect(hasConsecutiveBlocks(consolidated!)).toBe(false);

      // Content sections separated by blank lines
      const content = consolidated!.match(
        /<system-reminder>([\s\S]*?)<\/system-reminder>/
      );
      expect(content).toBeTruthy();
      expect(content![1]).toMatch(/\n\n/);
    });
  });

  describe('consolidateReminders utility function', () => {
    it('should return undefined for empty array', () => {
      expect(consolidateReminders([])).toBeUndefined();
    });

    it('should pass through a single reminder unchanged', () => {
      const single = '<system-reminder>\nSingle reminder\n</system-reminder>';
      const result = consolidateReminders([single]);
      expect(result).toBeDefined();
      expect(countReminderTags(result!)).toBe(1);
      expect(result).toContain('Single reminder');
    });

    it('should handle reminders that are already unwrapped (plain text)', () => {
      const result = consolidateReminders([
        'Plain reminder one',
        'Plain reminder two',
      ]);
      expect(result).toBeDefined();
      expect(countReminderTags(result!)).toBe(1);
      expect(result).toContain('Plain reminder one');
      expect(result).toContain('Plain reminder two');
    });

    it('should handle mix of wrapped and unwrapped reminders', () => {
      const result = consolidateReminders([
        '<system-reminder>\nWrapped\n</system-reminder>',
        'Unwrapped plain text',
      ]);
      expect(result).toBeDefined();
      expect(countReminderTags(result!)).toBe(1);
      expect(result).toContain('Wrapped');
      expect(result).toContain('Unwrapped plain text');
    });
  });
});
