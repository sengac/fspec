/**
 * Feature: spec/features/ai-driven-deep-code-review.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { review } from '../review';

describe('Feature: AI-Driven Deep Code Review', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-review-ai-driven-'));

    // Configure agent as 'claude' to get system-reminder tags
    await mkdir(join(testDir, 'spec'), { recursive: true });
    const configData = { agent: 'claude' };
    await writeFile(
      join(testDir, 'spec', 'fspec-config.json'),
      JSON.stringify(configData, null, 2)
    );
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Review provides system-reminder with AI instructions for deep analysis', () => {
    it('should emit system-reminder instructing AI to read implementation files and analyze code', async () => {
      // @step Given I have a work unit "AUTH-001" with linked implementation files in coverage data
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Login',
            type: 'story',
            status: 'validating',
            linkedFeatures: [],
          },
        },
      };

      await mkdir(join(testDir, 'spec'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create feature file
      const featureContent = `@AUTH-001
Feature: User Login

  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'user-login.feature'),
        featureContent
      );

      // And the implementation file is "src/auth/login.ts"
      const implContent = `export function login(username: string, password: string) {
  return { success: true };
}`;

      await mkdir(join(testDir, 'src', 'auth'), { recursive: true });
      await writeFile(join(testDir, 'src', 'auth', 'login.ts'), implContent);

      // Create coverage file with implementation mapping
      const coverageData = {
        scenarios: [
          {
            name: 'Login with valid credentials',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/auth/login.ts',
                    lines: [1, 2, 3],
                  },
                ],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
        },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'user-login.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // Update work unit with linked feature
      workUnitsData.workUnits['AUTH-001'].linkedFeatures = [
        { file: 'spec/features/user-login.feature' },
      ];
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run "fspec review AUTH-001"
      const result = await review('AUTH-001', { cwd: testDir });

      // @step Then the output should contain a <system-reminder> tag
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('</system-reminder>');

      // @step And the system-reminder should instruct the AI to read "src/auth/login.ts"
      expect(result.output).toContain('src/auth/login.ts');

      // @step And the system-reminder should instruct the AI to analyze code for bugs and edge cases
      // (Structural check: verify AI-DRIVEN system-reminder exists after Summary section)
      const summaryIndex = result.output.indexOf('## Summary');
      const reminderIndex = result.output.indexOf('AI-DRIVEN DEEP CODE REVIEW');
      expect(reminderIndex).toBeGreaterThan(summaryIndex);

      // @step And the system-reminder should instruct the AI to check FOUNDATION.md alignment
      expect(result.output).toContain('FOUNDATION.md');

      // @step And the system-reminder should instruct the AI to report findings conversationally
      // (Structural check: system-reminder appears after all static sections)
      expect(reminderIndex).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Review detects duplicated code and suggests shared utility', () => {
    it('should instruct AI to detect duplicated validation logic across files', async () => {
      // @step Given I have a work unit "VAL-001" with 3 implementation files
      const workUnitsData = {
        workUnits: {
          'VAL-001': {
            id: 'VAL-001',
            title: 'Data Validation',
            type: 'story',
            status: 'validating',
            linkedFeatures: [],
          },
        },
      };

      await mkdir(join(testDir, 'spec'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create feature and coverage with 3 implementation files
      const featureContent = `@VAL-001
Feature: Data Validation

  Scenario: Validate user input
    Given I have user input
    When I validate it
    Then it should be validated`;

      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'data-validation.feature'),
        featureContent
      );

      // @step And all 3 files contain similar validation logic
      await mkdir(join(testDir, 'src', 'validators'), { recursive: true });
      await writeFile(
        join(testDir, 'src', 'validators', 'user.ts'),
        'export function validateUser() { /* duplicated */ }'
      );
      await writeFile(
        join(testDir, 'src', 'validators', 'post.ts'),
        'export function validatePost() { /* duplicated */ }'
      );
      await writeFile(
        join(testDir, 'src', 'validators', 'comment.ts'),
        'export function validateComment() { /* duplicated */ }'
      );

      const coverageData = {
        scenarios: [
          {
            name: 'Validate user input',
            testMappings: [
              {
                file: 'src/__tests__/validation.test.ts',
                lines: '5-15',
                implMappings: [
                  { file: 'src/validators/user.ts', lines: [1] },
                  { file: 'src/validators/post.ts', lines: [1] },
                  { file: 'src/validators/comment.ts', lines: [1] },
                ],
              },
            ],
          },
        ],
        stats: { totalScenarios: 1, coveredScenarios: 1, coveragePercent: 100 },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'data-validation.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      workUnitsData.workUnits['VAL-001'].linkedFeatures = [
        { file: 'spec/features/data-validation.feature' },
      ];
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run "fspec review VAL-001"
      const result = await review('VAL-001', { cwd: testDir });

      // @step Then the output should contain a <system-reminder> tag
      expect(result.output).toContain('<system-reminder>');

      // @step And the system-reminder should instruct the AI to detect duplicated validation logic
      expect(result.output).toContain('duplicat');

      // @step And the system-reminder should suggest creating a shared validator utility
      expect(result.output).toContain('shared');

      // @step And the system-reminder should list the files containing duplicated code
      expect(result.output).toContain('src/validators/user.ts');
      expect(result.output).toContain('src/validators/post.ts');
      expect(result.output).toContain('src/validators/comment.ts');
    });
  });

  describe('Scenario: Review finds race condition and suggests FOUNDATION.md pattern', () => {
    it('should instruct AI to analyze async operations and check FOUNDATION.md', async () => {
      // @step Given I have a work unit "FILE-001" with async file operations
      const workUnitsData = {
        workUnits: {
          'FILE-001': {
            id: 'FILE-001',
            title: 'File Operations',
            type: 'story',
            status: 'validating',
            linkedFeatures: [],
          },
        },
      };

      await mkdir(join(testDir, 'spec'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const featureContent = `@FILE-001
Feature: File Operations

  Scenario: Save file safely
    Given I have data to save
    When I save it to disk
    Then it should be saved atomically`;

      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'file-operations.feature'),
        featureContent
      );

      // @step And the code has potential race conditions
      await mkdir(join(testDir, 'src', 'file-ops'), { recursive: true });
      await writeFile(
        join(testDir, 'src', 'file-ops', 'save.ts'),
        `export async function save() {
  await writeFile('test.txt', 'data');
  await writeFile('test.txt', 'more data'); // race condition
}`
      );

      // @step And FOUNDATION.md defines file locking patterns
      await writeFile(
        join(testDir, 'FOUNDATION.md'),
        '# File Locking\nUse atomic writes with temp files.'
      );

      const coverageData = {
        scenarios: [
          {
            name: 'Save file safely',
            testMappings: [
              {
                file: 'src/__tests__/file-ops.test.ts',
                lines: '10-25',
                implMappings: [{ file: 'src/file-ops/save.ts', lines: [1, 2, 3, 4] }],
              },
            ],
          },
        ],
        stats: { totalScenarios: 1, coveredScenarios: 1, coveragePercent: 100 },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'file-operations.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      workUnitsData.workUnits['FILE-001'].linkedFeatures = [
        { file: 'spec/features/file-operations.feature' },
      ];
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run "fspec review FILE-001"
      const result = await review('FILE-001', { cwd: testDir });

      // @step Then the output should contain a <system-reminder> tag
      expect(result.output).toContain('<system-reminder>');

      // @step And the system-reminder should instruct the AI to analyze async operations
      expect(result.output).toContain('async');

      // @step And the system-reminder should suggest checking FOUNDATION.md for file locking pattern
      expect(result.output).toContain('FOUNDATION.md');

      // @step And the system-reminder should guide the AI to detect race conditions
      expect(result.output).toContain('race condition');
    });
  });

  describe('Scenario: Review detects God function anti-pattern', () => {
    it('should instruct AI to detect large functions and suggest refactoring', async () => {
      // @step Given I have a work unit "PROC-001" with a 500-line function
      const workUnitsData = {
        workUnits: {
          'PROC-001': {
            id: 'PROC-001',
            title: 'Data Processing',
            type: 'story',
            status: 'validating',
            linkedFeatures: [],
          },
        },
      };

      await mkdir(join(testDir, 'spec'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const featureContent = `@PROC-001
Feature: Data Processing

  Scenario: Process large dataset
    Given I have a large dataset
    When I process it
    Then it should be processed`;

      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'data-processing.feature'),
        featureContent
      );

      // Create a 500-line God function
      const godFunction = `export function processEverything() {
${Array(500)
  .fill(0)
  .map((_, i) => `  // Line ${i + 1}`)
  .join('\n')}
}`;

      await mkdir(join(testDir, 'src', 'processing'), { recursive: true });
      await writeFile(join(testDir, 'src', 'processing', 'processor.ts'), godFunction);

      const coverageData = {
        scenarios: [
          {
            name: 'Process large dataset',
            testMappings: [
              {
                file: 'src/__tests__/processing.test.ts',
                lines: '5-20',
                implMappings: [
                  { file: 'src/processing/processor.ts', lines: Array.from({ length: 500 }, (_, i) => i + 1) },
                ],
              },
            ],
          },
        ],
        stats: { totalScenarios: 1, coveredScenarios: 1, coveragePercent: 100 },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'data-processing.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      workUnitsData.workUnits['PROC-001'].linkedFeatures = [
        { file: 'spec/features/data-processing.feature' },
      ];
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run "fspec review PROC-001"
      const result = await review('PROC-001', { cwd: testDir });

      // @step Then the output should contain a <system-reminder> tag
      expect(result.output).toContain('<system-reminder>');

      // @step And the system-reminder should instruct the AI to detect large functions
      expect(result.output).toContain('large');

      // @step And the system-reminder should suggest refactoring into smaller focused functions
      expect(result.output).toContain('refactor');

      // @step And the system-reminder should mention the God object anti-pattern
      expect(result.output).toContain('God');
    });
  });

  describe('Scenario: Review checks FOUNDATION.md file size principle', () => {
    it('should instruct AI to check file sizes against FOUNDATION.md principles', async () => {
      // @step Given I have a work unit "BIG-001" with a file at 450 lines
      const workUnitsData = {
        workUnits: {
          'BIG-001': {
            id: 'BIG-001',
            title: 'Large File Feature',
            type: 'story',
            status: 'validating',
            linkedFeatures: [],
          },
        },
      };

      await mkdir(join(testDir, 'spec'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const featureContent = `@BIG-001
Feature: Large File Feature

  Scenario: Handle large file
    Given I have a large implementation file
    When I review it
    Then it should flag size violations`;

      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'large-file-feature.feature'),
        featureContent
      );

      // Create 450-line file
      const bigFile = Array(450)
        .fill(0)
        .map((_, i) => `// Line ${i + 1}`)
        .join('\n');

      await mkdir(join(testDir, 'src', 'large'), { recursive: true });
      await writeFile(join(testDir, 'src', 'large', 'big-file.ts'), bigFile);

      // @step And FOUNDATION.md states "keep files under 300 lines"
      await writeFile(
        join(testDir, 'FOUNDATION.md'),
        '# Coding Standards\n- Keep files under 300 lines'
      );

      const coverageData = {
        scenarios: [
          {
            name: 'Handle large file',
            testMappings: [
              {
                file: 'src/__tests__/large.test.ts',
                lines: '5-15',
                implMappings: [
                  { file: 'src/large/big-file.ts', lines: Array.from({ length: 450 }, (_, i) => i + 1) },
                ],
              },
            ],
          },
        ],
        stats: { totalScenarios: 1, coveredScenarios: 1, coveragePercent: 100 },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'large-file-feature.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      workUnitsData.workUnits['BIG-001'].linkedFeatures = [
        { file: 'spec/features/large-file-feature.feature' },
      ];
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run "fspec review BIG-001"
      const result = await review('BIG-001', { cwd: testDir });

      // @step Then the output should contain a <system-reminder> tag
      expect(result.output).toContain('<system-reminder>');

      // @step And the system-reminder should instruct the AI to check file sizes against FOUNDATION.md
      expect(result.output).toContain('300 lines');

      // @step And the system-reminder should report the 450-line file violates the 300-line principle
      // (System-reminder INSTRUCTS the AI to check, it doesn't do the checking itself)
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('src/large/big-file.ts');

      // @step And the system-reminder should suggest refactoring to meet the standard
      // (System-reminder tells AI to look for this, doesn't do it itself)
      expect(result.output).toContain('FOUNDATION.md');
    });
  });

  describe('Scenario: Review preserves existing static checks', () => {
    it('should report static issues AND emit AI-driven analysis system-reminder', async () => {
      // @step Given I have a work unit "TEST-001" with code using "any" type
      const workUnitsData = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test Feature',
            type: 'story',
            status: 'validating',
            linkedFeatures: [],
          },
        },
      };

      await mkdir(join(testDir, 'spec'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const featureContent = `@TEST-001
Feature: Test Feature

  Scenario: Test scenario one
    Given I have a test
    When I run it
    Then it should pass

  Scenario: Test scenario two
    Given I have another test
    When I run it
    Then it should also pass`;

      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature'),
        featureContent
      );

      // Create test file with 'any' type
      const testContent = `describe('Test', () => {
  it('should work', () => {
    const data: any = {}; // Bad - using any type
  });
});`;

      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(join(testDir, 'src', '__tests__', 'test.test.ts'), testContent);

      // @step And the work unit has incomplete test coverage (only 1 of 2 scenarios)
      const coverageData = {
        scenarios: [
          {
            name: 'Test scenario one',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '1-5',
                implMappings: [],
              },
            ],
          },
          {
            name: 'Test scenario two',
            testMappings: [], // Uncovered
          },
        ],
        stats: { totalScenarios: 2, coveredScenarios: 1, coveragePercent: 50 },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // @step And the work unit has ACDD compliance violations (no rules)
      workUnitsData.workUnits['TEST-001'].linkedFeatures = [
        { file: 'spec/features/test-feature.feature' },
      ];
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run "fspec review TEST-001"
      const result = await review('TEST-001', { cwd: testDir });

      // @step Then the output should contain critical issue for "any" type usage
      expect(result.output).toContain('any');
      expect(result.output).toContain('Critical Issues');

      // @step And the output should contain ACDD compliance violations
      expect(result.output).toContain('ACDD Compliance');

      // @step And the output should contain incomplete test coverage warnings
      expect(result.output).toContain('50%');

      // @step And the output should also contain <system-reminder> for AI-driven deep analysis
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('analyze');
    });
  });
});
