/**
 * Feature: spec/features/answered-questions-display-as-true-instead-of-actual-answer-text.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { answerQuestion } from '../answer-question';
import { generateScenarios } from '../generate-scenarios';
import type { WorkUnitsData } from '../../types';

describe('Feature: Answered questions display as true instead of actual answer text', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');

    await mkdir(specDir, { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Store answer text in question.answer field', () => {
    it('should store answer text, not just boolean flag', async () => {
      // @step Given a work unit with a question "@human: When should data be cached?"
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
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
            questions: [
              {
                id: 0,
                text: '@human: When should data be cached?',
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
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

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step When I answer the question with "Cache for 24 hours with file-based persistence"
      const result = await answerQuestion({
        workUnitId: 'TEST-001',
        index: 0,
        answer: 'Cache for 24 hours with file-based persistence',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read updated data
      const updatedData: WorkUnitsData = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      const question = updatedData.workUnits['TEST-001'].questions[0];

      // @step Then the question.answered field should be true
      expect(question.answered).toBe(true);

      // @step And the question.answer field should contain "Cache for 24 hours with file-based persistence"
      expect(question.answer).toBe(
        'Cache for 24 hours with file-based persistence'
      );

      // @step And the question.answer should NOT be the boolean true
      expect(question.answer).not.toBe(true);
      expect(typeof question.answer).toBe('string');
    });
  });

  describe('Scenario: Display answer text in feature file comments', () => {
    it('should show answer text in feature file, not boolean true', async () => {
      // @step Given a work unit with an answered question
      // @step And the answer text is "Data should be cached immediately on first access"
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
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
            questions: [
              {
                id: 0,
                text: '@human: When should data be cached?',
                deleted: false,
                selected: true,
                answered: true,
                answer: 'Data should be cached immediately on first access',
                createdAt: new Date().toISOString(),
              },
            ],
            rules: [
              {
                id: 0,
                text: 'System must cache data',
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
            examples: [
              {
                id: 0,
                text: 'User accesses data and it is cached',
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
            nextQuestionId: 1,
            nextRuleId: 1,
            nextExampleId: 1,
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

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));
      await mkdir(join(specDir, 'features'), { recursive: true });

      // @step When I generate scenarios
      const result = await generateScenarios({
        workUnitId: 'TEST-002',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read generated feature file
      const featureFile = join(specDir, 'features', 'test-story-2.feature');
      const featureContent = await readFile(featureFile, 'utf-8');

      // @step Then the feature file should contain "A: Data should be cached immediately on first access"
      expect(featureContent).toContain(
        'A: Data should be cached immediately on first access'
      );

      // @step And the feature file should NOT contain "A: true"
      expect(featureContent).not.toContain('A: true');
    });
  });

  describe('Scenario: Preserve multiline answer text', () => {
    it('should store and display multiline answers correctly', async () => {
      // @step Given a work unit with a question
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
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
            questions: [
              {
                id: 0,
                text: '@human: What is the caching strategy?',
                deleted: false,
                selected: false,
                createdAt: new Date().toISOString(),
              },
            ],
            rules: [
              {
                id: 0,
                text: 'Placeholder rule',
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
            examples: [
              {
                id: 0,
                text: 'Placeholder example',
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
            nextQuestionId: 1,
            nextRuleId: 1,
            nextExampleId: 1,
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-003'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step When I answer with multiline text:
      const multilineAnswer = `Caching strategy:
1. Cache on first access
2. Use file-based persistence
3. TTL of 24 hours`;

      const result = await answerQuestion({
        workUnitId: 'TEST-003',
        index: 0,
        answer: multilineAnswer,
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read updated data
      const updatedData: WorkUnitsData = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      const question = updatedData.workUnits['TEST-003'].questions[0];

      // @step Then the answer text should be stored exactly as provided
      expect(question.answer).toBe(multilineAnswer);
      expect(question.answer).toContain('Caching strategy:');
      expect(question.answer).toContain('1. Cache on first access');
      expect(question.answer).toContain('2. Use file-based persistence');
      expect(question.answer).toContain('3. TTL of 24 hours');

      // @step And feature file comments should show all lines of the answer
      // Generate scenarios to verify feature file generation
      await mkdir(join(specDir, 'features'), { recursive: true });
      await generateScenarios({
        workUnitId: 'TEST-003',
        cwd: testDir,
      });

      const featureFile = join(specDir, 'features', 'test-story-3.feature');
      const featureContent = await readFile(featureFile, 'utf-8');

      // Verify all lines appear in feature file
      expect(featureContent).toContain('Caching strategy:');
      expect(featureContent).toContain('1. Cache on first access');
      expect(featureContent).toContain('2. Use file-based persistence');
      expect(featureContent).toContain('3. TTL of 24 hours');
    });
  });
});
