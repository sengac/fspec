/**
 * Feature: spec/features/enhance-fspec-review-with-architecture-alignment-and-ast-verification.feature
 *
 * This test file validates the enhanced review functionality with architecture alignment
 * and AST verification before transitioning from specifying to testing phase.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Enhance fspec review with architecture alignment and AST verification', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-review-arch-alignment');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
    await mkdir(join(testDir, 'spec', 'attachments'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Block transition when AST research is missing', () => {
    it('should fail with error code 1 when AST research attachments are missing', async () => {
      // @step Given I have a work unit in specifying state with completed feature file
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['WORK-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Test feature',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Test work unit',
            children: [],
            rules: ['Rule 1', 'Rule 2'],
            examples: ['Example 1'],
            questions: [],
            architectureNotes: ['Implementation: Use existing pattern'],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // Create a completed feature file
      const featureContent = `@WORK-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test the feature
    So that I can validate the review

  Scenario: Test scenario
    Given I have a test
    When I run the test
    Then it should pass
`;

      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature'),
        featureContent
      );

      // @step And the work unit has no AST research attachments
      // (No attachments created - attachments array is undefined/empty)

      // @step When I run 'fspec update-work-unit-status WORK-001 testing'
      const result = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'testing',
        cwd: testDir,
      });

      // @step Then the command should fail with error code 1
      expect(result.success).toBe(false);

      // @step And the error message should contain 'Cannot transition to testing - no AST research performed during discovery'
      expect(result.error).toContain('Cannot transition to testing');
      expect(result.error).toContain(
        'no AST research performed during discovery'
      );
      expect(result.error).toContain('fspec research --tool=ast');
      expect(result.error).toContain('fspec add-attachment');

      // @step And the work unit status should remain 'specifying'
      expect(result.newStatus).toBe('specifying');
    });
  });

  describe('Scenario: AI detects reinvention via AST data in system-reminder', () => {
    it('should emit system-reminder with AST data for AI analysis when objective checks pass', async () => {
      // @step Given I have a work unit with AST research attachments showing 'validateFeature' exists in 2 files
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['WORK-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Test feature',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Test work unit',
            children: [],
            rules: ['Rule 1'],
            examples: ['Example 1'],
            questions: [],
            architectureNotes: ['Implementation: Create validation utility'],
            attachments: ['spec/attachments/WORK-001/ast-research.json'],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // @step And the work unit has architectural notes and feature file
      const featureContent = `@WORK-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test the feature
    So that I can validate the review

  Scenario: Test scenario
    Given I have a test
    When I run the test
    Then it should pass
`;

      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature'),
        featureContent
      );

      // Create AST research attachment
      await mkdir(join(testDir, 'spec', 'attachments', 'WORK-001'), {
        recursive: true,
      });
      const astData = {
        functions: [
          { name: 'validateFeature', file: 'src/utils/validation.ts' },
          { name: 'validateFeature', file: 'src/commands/validate.ts' },
        ],
      };
      await writeFile(
        join(testDir, 'spec', 'attachments', 'WORK-001', 'ast-research.json'),
        JSON.stringify(astData, null, 2)
      );

      // @step When I run 'fspec update-work-unit-status WORK-001 testing'
      const result = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'testing',
        cwd: testDir,
      });

      // @step Then the command should succeed with status code 0
      expect(result.success).toBe(true);

      // @step And a system-reminder should be emitted with AST structural data
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');

      // @step And the system-reminder should show AST research attachments
      expect(result.systemReminder).toContain('AST RESEARCH ATTACHMENTS');
      expect(result.systemReminder).toContain('ast-research.json');

      // @step And the AI should revert to specifying state after analyzing the data
      // (This step is AI behavior - not tested in this unit test, but the system-reminder
      // provides the data for AI to make the decision)
    });
  });

  describe('Scenario: Pass review with proper architecture documentation and AST research', () => {
    it('should allow transition when all objective checks pass and documentation is complete', async () => {
      // @step Given I have a work unit with architectural note 'Refactoring: Extract validation logic to shared utility'
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['WORK-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Extract validation utility',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Refactoring work unit',
            children: [],
            rules: ['DRY principle: Extract shared validation'],
            examples: ['3 files have duplicate validation code'],
            questions: [],
            architectureNotes: [
              'Refactoring: Extract validation logic to shared utility because current code has 3 copies',
            ],
            attachments: ['spec/attachments/WORK-001/ast-research.json'],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // @step And the work unit has AST research attachments showing 3 copies of validation pattern
      await mkdir(join(testDir, 'spec', 'attachments', 'WORK-001'), {
        recursive: true,
      });
      const astData = {
        duplicatePatterns: [
          { file: 'src/commands/create.ts', lines: [45, 60] },
          { file: 'src/commands/update.ts', lines: [23, 38] },
          { file: 'src/commands/delete.ts', lines: [12, 27] },
        ],
      };
      await writeFile(
        join(testDir, 'spec', 'attachments', 'WORK-001', 'ast-research.json'),
        JSON.stringify(astData, null, 2)
      );

      // @step And the work unit has a completed feature file with no prefill placeholders
      const featureContent = `@WORK-001
Feature: Extract Validation Utility

  Background: User Story
    As a developer
    I want to extract validation logic to shared utility
    So that I follow DRY principle and reduce code duplication

  Scenario: Create shared validation utility
    Given I have identified 3 files with duplicate validation
    When I extract the validation logic
    Then all 3 files should use the shared utility
`;

      await writeFile(
        join(testDir, 'spec', 'features', 'extract-validation-utility.feature'),
        featureContent
      );

      // @step When I run 'fspec update-work-unit-status WORK-001 testing'
      const result = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'testing',
        cwd: testDir,
      });

      // @step Then the command should succeed with status code 0
      expect(result.success).toBe(true);

      // @step And the work unit status should be 'testing'
      expect(result.newStatus).toBe('testing');

      // @step And the review validation should pass all objective ACDD checks
      // (Validation passed - no errors, status successfully changed)
      expect(result.error).toBeUndefined();
    });
  });
});
