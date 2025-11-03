/**
 * Feature: spec/features/generate-scenarios-counts-deleted-questions-as-unanswered.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateScenarios } from '../generate-scenarios';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: generate-scenarios counts deleted questions as unanswered', () => {
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

  describe('Scenario: Deleted questions should not be counted as unanswered', () => {
    it('should not count deleted questions as unanswered and allow generation', async () => {
      // Given a work unit has 3 deleted questions with deleted=true and selected=false
      // And the work unit has 5 answered questions with deleted=false and selected=true
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-001'] = {
        id: 'TEST-001',
        title: 'Test Work Unit',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'AI agent',
          action: 'generate scenarios',
          benefit: 'I can proceed',
        },
        questions: [
          // 3 deleted questions (should be ignored)
          {
            id: 0,
            text: '@human: Deleted question 1?',
            deleted: true,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          {
            id: 1,
            text: '@human: Deleted question 2?',
            deleted: true,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          {
            id: 2,
            text: '@human: Deleted question 3?',
            deleted: true,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          // 5 answered questions (selected=true)
          {
            id: 3,
            text: '@human: Answered question 1?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 4,
            text: '@human: Answered question 2?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 5,
            text: '@human: Answered question 3?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 6,
            text: '@human: Answered question 4?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 7,
            text: '@human: Answered question 5?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
        ],
        examples: [{ id: 0, text: 'Example 1', deleted: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('TEST-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios TEST-001"
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: testDir,
        ignorePossibleDuplicates: true,
      });

      // Then the validation should count 0 unanswered questions
      // And the command should succeed and generate scenarios
      expect(result.success).toBe(true);
      expect(result.featureFile).toContain('test-work-unit.feature');

      // And the command should not throw "unanswered questions found" error
      // (No error thrown = test passes)
    });
  });

  describe('Scenario: Unanswered non-deleted questions should block generation', () => {
    it('should count unanswered non-deleted questions and fail with error', async () => {
      // Given a work unit has 2 unanswered questions with deleted=false and selected=false
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-002'] = {
        id: 'TEST-002',
        title: 'Test Work Unit 2',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'AI agent',
          action: 'generate scenarios',
          benefit: 'I can proceed',
        },
        questions: [
          {
            id: 0,
            text: '@human: Unanswered question 1?',
            deleted: false,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          {
            id: 1,
            text: '@human: Unanswered question 2?',
            deleted: false,
            selected: false,
            createdAt: new Date().toISOString(),
          },
        ],
        examples: [{ id: 0, text: 'Example 1', deleted: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('TEST-002');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios TEST-002"
      // Then the validation should count 2 unanswered questions
      // And the command should fail with "2 unanswered questions found" error
      await expect(
        generateScenarios({
          workUnitId: 'TEST-002',
          cwd: testDir,
          ignorePossibleDuplicates: true,
        })
      ).rejects.toThrow('2 unanswered questions found');

      // And the command should not generate scenarios
      // (Verified by the rejection above)
    });
  });

  describe('Scenario: Work unit with no questions should allow generation', () => {
    it('should allow generation when work unit has no questions', async () => {
      // Given a work unit has no questions
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-003'] = {
        id: 'TEST-003',
        title: 'Test Work Unit 3',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'AI agent',
          action: 'generate scenarios',
          benefit: 'I can proceed',
        },
        questions: [], // No questions
        examples: [{ id: 0, text: 'Example 1', deleted: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('TEST-003');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios TEST-003"
      const result = await generateScenarios({
        workUnitId: 'TEST-003',
        cwd: testDir,
        ignorePossibleDuplicates: true,
      });

      // Then the validation should count 0 unanswered questions
      // And the command should succeed and generate scenarios
      expect(result.success).toBe(true);
      expect(result.featureFile).toContain('test-work-unit-3.feature');
    });
  });

  describe('Scenario: Deleted questions should not block status transition in update-work-unit-status', () => {
    it('should not count deleted questions and allow status transition', async () => {
      // @step Given a work unit in "specifying" status has 3 deleted questions with deleted=true and selected=false
      // @step And the work unit has 5 answered questions with deleted=false and selected=true
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-004'] = {
        id: 'TEST-004',
        title: 'Test Work Unit 4',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'AI agent',
          action: 'transition to testing',
          benefit: 'I can proceed',
        },
        questions: [
          // 3 deleted questions (should be ignored)
          {
            id: 0,
            text: '@human: Deleted question 1?',
            deleted: true,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          {
            id: 1,
            text: '@human: Deleted question 2?',
            deleted: true,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          {
            id: 2,
            text: '@human: Deleted question 3?',
            deleted: true,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          // 5 answered questions (selected=true)
          {
            id: 3,
            text: '@human: Answered question 1?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 4,
            text: '@human: Answered question 2?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 5,
            text: '@human: Answered question 3?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 6,
            text: '@human: Answered question 4?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
          {
            id: 7,
            text: '@human: Answered question 5?',
            deleted: false,
            selected: true,
            answer: 'Yes',
            createdAt: new Date().toISOString(),
          },
        ],
        examples: [{ id: 0, text: 'Example 1', deleted: false }],
        stateHistory: [
          { state: 'specifying', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('TEST-004');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Create minimal feature file for TEST-004
      const featureContent = `@TEST-004
Feature: Test Feature 4

  Background: User Story
    As a AI agent
    I want to transition to testing
    So that I can proceed

  Scenario: Test scenario
    Given a precondition
    When an action
    Then a result
`;
      await writeFile(
        join(featuresDir, 'test-feature-4.feature'),
        featureContent
      );

      // @step When I run "fspec update-work-unit-status TEST-004 testing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-004',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step Then the validation should count 0 unanswered questions
      // @step And the command should succeed and update status to "testing"
      expect(result.success).toBe(true);

      // @step And the command should not throw "unanswered questions" error
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['TEST-004'].status).toBe('testing');
    });
  });

  describe('Scenario: Unanswered non-deleted questions should block status transition', () => {
    it('should count unanswered non-deleted questions and block transition', async () => {
      // @step Given a work unit in "specifying" status has 2 unanswered questions with deleted=false and selected=false
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-005'] = {
        id: 'TEST-005',
        title: 'Test Work Unit 5',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'AI agent',
          action: 'transition to testing',
          benefit: 'I can proceed',
        },
        questions: [
          {
            id: 0,
            text: '@human: Unanswered question 1?',
            deleted: false,
            selected: false,
            createdAt: new Date().toISOString(),
          },
          {
            id: 1,
            text: '@human: Unanswered question 2?',
            deleted: false,
            selected: false,
            createdAt: new Date().toISOString(),
          },
        ],
        examples: [{ id: 0, text: 'Example 1', deleted: false }],
        stateHistory: [
          { state: 'specifying', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('TEST-005');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Create minimal feature file for TEST-005
      const featureContent = `@TEST-005
Feature: Test Feature 5

  Background: User Story
    As a AI agent
    I want to transition to testing
    So that I can proceed

  Scenario: Test scenario
    Given a precondition
    When an action
    Then a result
`;
      await writeFile(
        join(featuresDir, 'test-feature-5.feature'),
        featureContent
      );

      // @step When I run "fspec update-work-unit-status TEST-005 testing"
      // @step Then the validation should count 2 unanswered questions
      // @step And the command should fail with "Unanswered questions prevent state transition" error
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-005',
          status: 'testing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/Unanswered questions prevent state transition/);

      // @step And the status should remain "specifying"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['TEST-005'].status).toBe('specifying');
    });
  });
});
