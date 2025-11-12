/**
 * Feature: spec/features/enhance-fspec-review-with-dry-and-solid-principles-checking-using-ast-analysis.feature
 *
 * Tests for AST-based structural data gathering in fspec review command.
 * These tests ensure fspec provides data to AI without making semantic judgments.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { review } from '../review';
import * as fs from 'fs/promises';
import * as path from 'path';

describe('Feature: Enhance fspec review with DRY and SOLID principles checking using AST analysis', () => {
  const testWorkDir = path.join(
    process.cwd(),
    'test-fixtures',
    'review-ast-data'
  );

  beforeEach(async () => {
    // Setup test workspace
    await fs.mkdir(testWorkDir, { recursive: true });
  });

  afterEach(async () => {
    // Cleanup test workspace
    await fs.rm(testWorkDir, { recursive: true, force: true });
  });

  describe('Scenario: Gather structural data for implementation files', () => {
    it('should emit system-reminder with structural data when reviewing work unit', async () => {
      // @step Given a work unit AUTH-001 has implementation files with test coverage
      const workUnitId = 'AUTH-001';
      await createWorkUnit(workUnitId, testWorkDir);
      await createImplementationFiles(workUnitId, testWorkDir);
      await createCoverageFiles(workUnitId, testWorkDir);

      // @step When AI runs fspec review AUTH-001
      const result = await review(workUnitId, { cwd: testWorkDir });

      // @step Then a system-reminder should be emitted with structural data
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('CODE STRUCTURE DATA');

      // @step And the system-reminder should include function counts for each file
      expect(result.output).toContain('Functions:');
      expect(result.output).toMatch(/Functions: \d+/);

      // @step And the system-reminder should include class counts for each file
      expect(result.output).toContain('Classes:');
      expect(result.output).toMatch(/Classes: \d+/);
    });
  });

  describe('Scenario: Identify functions with identical names across files', () => {
    it('should list functions with same name in multiple files', async () => {
      // @step Given implementation files have formatOutput() in review.ts and validate.ts
      const workUnitId = 'AUTH-002';
      await createWorkUnit(workUnitId, testWorkDir);
      await createFilesWithDuplicateFunctions(workUnitId, testWorkDir);
      await createCoverageFiles(workUnitId, testWorkDir);

      // @step When fspec review runs AST analysis
      const result = await review(workUnitId, { cwd: testWorkDir });

      // @step Then the system-reminder should show formatOutput() appears in review.ts:156 and validate.ts:234
      expect(result.output).toContain('formatOutput');
      expect(result.output).toContain('review.ts');
      expect(result.output).toContain('validate.ts');
      expect(result.output).toMatch(/formatOutput.*appears in \d+ files/i);
    });
  });

  describe('Scenario: Suggest AST commands for deeper investigation', () => {
    it('should include suggested fspec research commands in system-reminder', async () => {
      // @step Given fspec review has gathered initial structural data
      const workUnitId = 'AUTH-003';
      await createWorkUnit(workUnitId, testWorkDir);
      await createImplementationFiles(workUnitId, testWorkDir);
      await createCoverageFiles(workUnitId, testWorkDir);

      // @step When the system-reminder is generated
      const result = await review(workUnitId, { cwd: testWorkDir });

      // @step Then it should include command: fspec research --tool=ast --operation=list-functions --file=src/commands/review.ts
      expect(result.output).toContain('fspec research');
      expect(result.output).toContain('--tool=ast');
      expect(result.output).toContain('--operation=list-functions');
      expect(result.output).toMatch(
        /fspec research --tool=ast --operation=\w+/
      );
    });
  });

  describe('Scenario: Include guidance questions for AI analysis', () => {
    it('should ask AI to consider DRY/SOLID principles', async () => {
      // @step Given structural data has been gathered
      const workUnitId = 'AUTH-004';
      await createWorkUnit(workUnitId, testWorkDir);
      await createImplementationFiles(workUnitId, testWorkDir);
      await createCoverageFiles(workUnitId, testWorkDir);

      // @step When the system-reminder is formatted
      const result = await review(workUnitId, { cwd: testWorkDir });

      // @step Then it should ask: Are there functions with similar names that might have duplicate logic?
      expect(result.output).toContain('Are there functions with similar names');
      expect(result.output).toContain('duplicate logic');
      expect(result.output).toMatch(/consider.*DRY|SOLID/i);
    });
  });

  describe('Scenario: Present data in neutral format without judgments', () => {
    it('should use neutral language and NOT make quality judgments', async () => {
      // @step Given similar function names have been detected
      const workUnitId = 'AUTH-005';
      await createWorkUnit(workUnitId, testWorkDir);
      await createFilesWithDuplicateFunctions(workUnitId, testWorkDir);
      await createCoverageFiles(workUnitId, testWorkDir);

      // @step When the system-reminder describes the pattern
      const result = await review(workUnitId, { cwd: testWorkDir });

      // @step Then it should say: Patterns detected (NOT violations, just observations)
      expect(result.output).toContain('Patterns detected');
      expect(result.output).toContain('NOT violations');
      expect(result.output).toContain('observations');

      // @step And it should NOT say: violation detected or similarity score calculated
      expect(result.output).not.toContain('violation detected');
      expect(result.output).not.toContain('similarity score');
      expect(result.output).not.toContain('quality score');
      expect(result.output).not.toMatch(/\d+% similar/);
    });
  });
});

// Test helper functions
async function createWorkUnit(workUnitId: string, cwd: string): Promise<void> {
  const workUnitsPath = path.join(cwd, 'spec', 'work-units.json');
  await fs.mkdir(path.dirname(workUnitsPath), { recursive: true });

  const workUnitsData = {
    workUnits: {
      [workUnitId]: {
        id: workUnitId,
        title: 'Test Work Unit',
        type: 'story',
        status: 'implementing',
        prefix: 'AUTH',
        description: 'Test work unit for AST data gathering',
        rules: ['Test rule'],
        examples: ['Test example'],
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
          { state: 'testing', timestamp: new Date().toISOString() },
          { state: 'implementing', timestamp: new Date().toISOString() },
        ],
      },
    },
    prefixes: {},
    epics: {},
  };

  await fs.writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

  // Create agent config to enable system-reminders in tests
  const agentConfigPath = path.join(cwd, 'spec', 'fspec-config.json');
  const agentConfig = {
    agent: 'claude',
  };
  await fs.writeFile(agentConfigPath, JSON.stringify(agentConfig, null, 2));
}

async function createImplementationFiles(
  workUnitId: string,
  cwd: string
): Promise<void> {
  // Create implementation files with functions and classes
  const srcDir = path.join(cwd, 'src', 'commands');
  await fs.mkdir(srcDir, { recursive: true });

  const reviewFile = path.join(srcDir, 'review.ts');
  await fs.writeFile(
    reviewFile,
    `
export class ReviewCommand {
  async execute() {
    return this.formatOutput();
  }

  private formatOutput() {
    return 'Review output';
  }
}

export function validateInput(input: string): boolean {
  return input.length > 0;
}
`.trim()
  );

  const validateFile = path.join(srcDir, 'validate.ts');
  await fs.writeFile(
    validateFile,
    `
export function validateSchema(schema: object): boolean {
  return Object.keys(schema).length > 0;
}
`.trim()
  );
}

async function createFilesWithDuplicateFunctions(
  workUnitId: string,
  cwd: string
): Promise<void> {
  // Create files with formatOutput() in multiple locations
  const srcDir = path.join(cwd, 'src', 'commands');
  await fs.mkdir(srcDir, { recursive: true });

  const reviewFile = path.join(srcDir, 'review.ts');
  await fs.writeFile(
    reviewFile,
    `
export function formatOutput(data: any): string {
  return JSON.stringify(data);
}
`.trim()
  );

  const validateFile = path.join(srcDir, 'validate.ts');
  await fs.writeFile(
    validateFile,
    `
export function formatOutput(data: any): string {
  return data.toString();
}
`.trim()
  );
}

async function createCoverageFiles(
  workUnitId: string,
  cwd: string
): Promise<void> {
  // Create feature file
  const featureFile = path.join(
    cwd,
    'spec',
    'features',
    'test-feature.feature'
  );
  await fs.mkdir(path.dirname(featureFile), { recursive: true });

  await fs.writeFile(
    featureFile,
    `
@${workUnitId}
Feature: Test Feature

  Scenario: Test scenario
    Given a test setup
    When I run a test
    Then I should see results
`.trim()
  );

  // Create coverage file
  const coverageFile = featureFile + '.coverage';
  const coverageData = {
    scenarios: [
      {
        name: 'Test scenario',
        testMappings: [
          {
            file: 'src/__tests__/test.test.ts',
            lines: '10-20',
            implMappings: [
              {
                file: 'src/commands/review.ts',
                lines: [5, 10, 15],
              },
              {
                file: 'src/commands/validate.ts',
                lines: [3, 7],
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

  await fs.writeFile(coverageFile, JSON.stringify(coverageData, null, 2));
}
