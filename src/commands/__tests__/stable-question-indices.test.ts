/**
 * Feature: spec/features/stable-question-indices-for-concurrent-answers.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * Tests the fix for race condition in answer-question when run concurrently.
 * Questions use stable indices with 'selected' flag instead of array removal.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { addQuestion } from '../example-mapping';
import { answerQuestion } from '../answer-question';
import { showWorkUnit } from '../work-unit';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Stable Question Indices for Concurrent Answers', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });
    await mkdir(featuresDir, { recursive: true });

    // Initialize work units file
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {},
          states: {
            backlog: [],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
            blocked: [],
          },
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add questions as objects with selected flag', () => {
    it('should add question as object with selected: false', async () => {
      // Given I have a work unit in specifying status
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'Test work unit',
        status: 'specifying',
        questions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec add-question WORK-001 "@human: Should we use option A?"`
      await addQuestion('WORK-001', '@human: Should we use option A?', {
        cwd: testDir,
      });

      // Then the question should be added as {text: "@human: Should we use option A?", selected: false}
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions).toHaveLength(1);

      const question = updatedWorkUnits.workUnits['WORK-001'].questions[0];
      expect(question).toHaveProperty(
        'text',
        '@human: Should we use option A?'
      );
      expect(question).toHaveProperty('selected', false);
    });
  });

  describe('Scenario: Answer question marks it as selected without removing', () => {
    it('should mark question as selected and preserve all indices', async () => {
      // Given I have a work unit with 3 questions at indices 0, 1, 2
      // And all questions have selected: false
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'Test work unit',
        status: 'specifying',
        questions: [
          { text: '@human: Question 0?', selected: false },
          { text: '@human: Question 1?', selected: false },
          { text: '@human: Question 2?', selected: false },
        ],
        rules: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec answer-question WORK-001 1 --answer "Use option B" --add-to rule`
      await answerQuestion({
        workUnitId: 'WORK-001',
        index: 1,
        answer: 'Use option B',
        addTo: 'rule',
        cwd: testDir,
      });

      // Then question at index 1 should have selected: true
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[1].selected).toBe(
        true
      );

      // And question at index 1 should have answer: "Use option B"
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[1].answer).toBe(
        'Use option B'
      );

      // And questions at indices 0 and 2 should still have selected: false
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[0].selected).toBe(
        false
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[2].selected).toBe(
        false
      );

      // And all 3 indices should remain in the array (0, 1, 2)
      expect(updatedWorkUnits.workUnits['WORK-001'].questions).toHaveLength(3);
    });
  });

  describe('Scenario: Concurrent answer commands with stable indices', () => {
    it('should handle parallel answers without data loss', async () => {
      // Given I have a work unit with 3 questions at indices 0, 1, 2
      // And all questions have selected: false
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'Test work unit',
        status: 'specifying',
        questions: [
          { text: '@human: Question 0?', selected: false },
          { text: '@human: Question 1?', selected: false },
          { text: '@human: Question 2?', selected: false },
        ],
        rules: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run 3 answer-question commands in parallel
      await Promise.all([
        answerQuestion({
          workUnitId: 'WORK-001',
          index: 0,
          answer: 'Answer 0',
          addTo: 'rule',
          cwd: testDir,
        }),
        answerQuestion({
          workUnitId: 'WORK-001',
          index: 1,
          answer: 'Answer 1',
          addTo: 'rule',
          cwd: testDir,
        }),
        answerQuestion({
          workUnitId: 'WORK-001',
          index: 2,
          answer: 'Answer 2',
          addTo: 'rule',
          cwd: testDir,
        }),
      ]);

      // Then all 3 questions should have selected: true
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[0].selected).toBe(
        true
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[1].selected).toBe(
        true
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[2].selected).toBe(
        true
      );

      // And all 3 answers should be saved in their respective questions
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[0].answer).toBe(
        'Answer 0'
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[1].answer).toBe(
        'Answer 1'
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[2].answer).toBe(
        'Answer 2'
      );

      // And all 3 rules should be added to the work unit rules array
      expect(updatedWorkUnits.workUnits['WORK-001'].rules).toHaveLength(3);
      expect(
        updatedWorkUnits.workUnits['WORK-001'].rules.some(
          r => r.text === 'Answer 0'
        )
      ).toBe(true);
      expect(
        updatedWorkUnits.workUnits['WORK-001'].rules.some(
          r => r.text === 'Answer 1'
        )
      ).toBe(true);
      expect(
        updatedWorkUnits.workUnits['WORK-001'].rules.some(
          r => r.text === 'Answer 2'
        )
      ).toBe(true);

      // And no data loss should occur (all questions still present)
      expect(updatedWorkUnits.workUnits['WORK-001'].questions).toHaveLength(3);
    });
  });

  describe('Scenario: Display only unselected questions', () => {
    it('should show only questions with selected: false', async () => {
      // Given I have a work unit with 5 questions
      // And questions at indices 1 and 3 have selected: true
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'Test work unit',
        status: 'specifying',
        questions: [
          { text: '@human: Question 0?', selected: false },
          { text: '@human: Question 1?', selected: true, answer: 'Answer 1' },
          { text: '@human: Question 2?', selected: false },
          { text: '@human: Question 3?', selected: true, answer: 'Answer 3' },
          { text: '@human: Question 4?', selected: false },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec show-work-unit WORK-001`
      const output = await showWorkUnit('WORK-001', { cwd: testDir });

      // Then the output should display only questions at indices 0, 2, and 4
      expect(output).toContain('Question 0?');
      expect(output).toContain('Question 2?');
      expect(output).toContain('Question 4?');

      // And selected questions should not appear in the questions list
      expect(output).not.toContain('Question 1?');
      expect(output).not.toContain('Question 3?');
    });
  });

  describe('Scenario: Validate unanswered questions before testing phase', () => {
    it('should block transition with unselected questions', async () => {
      // Given I have a work unit in specifying status
      // And the work unit has 3 questions
      // And question at index 1 has selected: true
      // And questions at indices 0 and 2 have selected: false
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'Test work unit',
        status: 'specifying',
        questions: [
          { text: '@human: Question 0?', selected: false },
          { text: '@human: Question 1?', selected: true, answer: 'Answer 1' },
          { text: '@human: Question 2?', selected: false },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec update-work-unit-status WORK-001 testing`
      // Then the command should fail with error about unanswered questions
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'WORK-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow('Unanswered questions prevent state transition');

      // And the error should list questions at indices 0 and 2
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'WORK-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow('Question 0?');

      await expect(
        updateWorkUnitStatus({
          workUnitId: 'WORK-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow('Question 2?');
    });
  });

  describe('Scenario: Answer same question twice is idempotent', () => {
    it('should allow re-answering with last write wins', async () => {
      // Given I have a work unit with a question at index 0
      // And question 0 has selected: false
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'Test work unit',
        status: 'specifying',
        questions: [{ text: '@human: Question?', selected: false }],
        rules: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec answer-question WORK-001 0 --answer "Answer A" --add-to rule`
      await answerQuestion({
        workUnitId: 'WORK-001',
        index: 0,
        answer: 'Answer A',
        addTo: 'rule',
        cwd: testDir,
      });

      // And I run `fspec answer-question WORK-001 0 --answer "Answer B" --add-to rule`
      await answerQuestion({
        workUnitId: 'WORK-001',
        index: 0,
        answer: 'Answer B',
        addTo: 'rule',
        cwd: testDir,
      });

      // Then question 0 should have selected: true
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[0].selected).toBe(
        true
      );

      // And question 0 should have answer: "Answer B" (last write wins)
      expect(updatedWorkUnits.workUnits['WORK-001'].questions[0].answer).toBe(
        'Answer B'
      );

      // And both rules "Answer A" and "Answer B" should be added
      expect(updatedWorkUnits.workUnits['WORK-001'].rules).toHaveLength(2);
      expect(
        updatedWorkUnits.workUnits['WORK-001'].rules.some(
          r => r.text === 'Answer A'
        )
      ).toBe(true);
      expect(
        updatedWorkUnits.workUnits['WORK-001'].rules.some(
          r => r.text === 'Answer B'
        )
      ).toBe(true);
    });
  });
});
