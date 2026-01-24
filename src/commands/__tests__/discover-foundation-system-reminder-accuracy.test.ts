/**
 * Feature: spec/features/discover-foundation-system-reminder-accuracy.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests verify that system reminders instruct the correct command names for
 * foundation field updates.
 *
 * Bug Fix: GitHub Issue #1
 * The discover-foundation command was emitting incorrect system reminder for
 * problemSpace.primaryProblem.title field (said "problemDefinition" instead of "problemTitle").
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { discoverFoundation } from '../discover-foundation';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Fix incorrect command name in discover-foundation system reminder', () => {
  let testDir: string;
  let draftPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = await createTempTestDir(
      'discover-foundation-system-reminder-accuracy'
    );
    draftPath = join(testDir, 'spec/foundation.json.draft');
  });

  afterEach(async () => {
    // Clean up
    await removeTempTestDir(testDir);
  });

  describe('Scenario: System reminder instructs to use problemTitle command for title field', () => {
    it('should instruct fspec update-foundation problemTitle for title field', async () => {
      // Given: I have a draft with problemSpace.primaryProblem.title unfilled
      const draft = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: '[QUESTION: What is the primary problem?]',
            description: 'Some description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When: The discover-foundation command emits system reminder for this field
      const result = await discoverFoundation({
        cwd: testDir,
        draftPath,
        scanOnly: true,
      });

      // Then: The system reminder should instruct "fspec update-foundation problemTitle"
      expect(result.systemReminder).toContain(
        'fspec update-foundation problemTitle'
      );

      // And: The system reminder should NOT instruct "fspec update-foundation problemDefinition"
      expect(result.systemReminder).not.toContain(
        'fspec update-foundation problemDefinition'
      );
    });
  });

  describe('Scenario: System reminder uses correct command for description field', () => {
    it('should instruct fspec update-foundation problemDefinition for description field', async () => {
      // Given: I have a draft with problemSpace.primaryProblem.description unfilled
      const draft = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: '[QUESTION: What is the problem description?]',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When: The discover-foundation command emits system reminder for this field
      const result = await discoverFoundation({
        cwd: testDir,
        draftPath,
        scanOnly: true,
      });

      // Then: The system reminder should instruct "fspec update-foundation problemDefinition"
      expect(result.systemReminder).toContain(
        'fspec update-foundation problemDefinition'
      );

      // And: The system reminder should NOT instruct "fspec update-foundation problemTitle"
      expect(result.systemReminder).not.toContain(
        'fspec update-foundation problemTitle'
      );
    });
  });
});
