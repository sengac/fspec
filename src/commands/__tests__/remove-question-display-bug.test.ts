/**
 * Feature: spec/features/remove-question-command-shows-object-object-instead-of-question-text.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { removeQuestion } from '../remove-question';
import type { WorkUnitsData } from '../../types';

describe('Feature: remove-question command shows "[object Object]" instead of question text', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Remove question displays actual question text', () => {
    it('should return the actual question text, not [object Object]', async () => {
      // Given I have a work unit "TEST-001" with a question "Should we support OAuth?"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test Work Unit',
            status: 'specifying',
            type: 'story',
            questions: [{ text: 'Should we support OAuth?', selected: false }],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-question TEST-001 0"
      const result = await removeQuestion({
        workUnitId: 'TEST-001',
        index: 0,
        cwd: testDir,
      });

      // Then the success message should display "Removed question: Should we support OAuth?"
      expect(result.removedQuestion).toBe('Should we support OAuth?');

      // And the message should not contain "[object Object]"
      expect(result.removedQuestion).not.toContain('[object Object]');
    });
  });

  describe('Scenario: Remove question with special characters displays correctly', () => {
    it('should return the actual question text with special characters', async () => {
      // Given I have a work unit "TEST-002" with a question "@human: What happens?"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            title: 'Test Work Unit',
            status: 'specifying',
            type: 'story',
            questions: [{ text: '@human: What happens?', selected: false }],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
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
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-question TEST-002 0"
      const result = await removeQuestion({
        workUnitId: 'TEST-002',
        index: 0,
        cwd: testDir,
      });

      // Then the success message should display "Removed question: @human: What happens?"
      expect(result.removedQuestion).toBe('@human: What happens?');

      // And the message should not contain "[object Object]"
      expect(result.removedQuestion).not.toContain('[object Object]');
    });
  });
});
