/**
 * Feature: spec/features/review-command-with-done-status-integration.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { review } from '../review';

describe('Feature: Review command with done status integration', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-review');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
    await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
    await mkdir(join(testDir, 'src', 'commands'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Review completed work unit with comprehensive analysis', () => {
    it('should display Issues Found, ACDD Compliance, Coverage Analysis, and Summary sections', async () => {
      // Given I have a completed work unit CLI-011 in done status
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'CLI-011': {
            id: 'CLI-011',
            title: 'Slash command for critical story review',
            type: 'story',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Test work unit',
            children: [],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
              { state: 'testing', timestamp: new Date().toISOString() },
              { state: 'implementing', timestamp: new Date().toISOString() },
              { state: 'validating', timestamp: new Date().toISOString() },
              { state: 'done', timestamp: new Date().toISOString() },
            ],
            estimate: 3,
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // And the work unit has linked feature files and coverage data
      const featureContent = `@CLI-011
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test
    So that I can verify

  Scenario: Test scenario
    Given test
    When test
    Then test
`;

      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature'),
        featureContent
      );

      const coverageContent = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/commands/test.ts',
                    lines: [10, 11, 12],
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
        join(testDir, 'spec', 'features', 'test-feature.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // When I run 'fspec review CLI-011'
      const result = await review('CLI-011', { cwd: testDir });

      // Then the output should show Issues Found section with critical issues and warnings
      expect(result.output).toContain('Issues Found');
      expect(result.output).toMatch(/Critical Issues|Warnings/);

      // And the output should show ACDD Compliance section
      expect(result.output).toContain('ACDD Compliance');

      // And the output should show Coverage Analysis section
      expect(result.output).toContain('Coverage Analysis');

      // And the output should show Summary with priority actions
      expect(result.output).toContain('Summary');
      expect(result.output).toMatch(/Priority Actions|Overall Assessment/);
    });
  });

  describe('Scenario: Review in-progress work unit shows current state', () => {
    it('should show current workflow progress and suggest next steps', async () => {
      // Given I have a work unit SPEC-001 in testing status
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'SPEC-001': {
            id: 'SPEC-001',
            title: 'In-progress feature',
            type: 'story',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            description: 'Test work unit in progress',
            children: [],
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
              { state: 'testing', timestamp: new Date().toISOString() },
            ],
            estimate: 5,
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // And the work unit has incomplete test coverage
      const featureContent = `@SPEC-001
Feature: Incomplete Feature

  Background: User Story
    As a developer
    I want to test
    So that I can verify

  Scenario: Covered scenario
    Given test

  Scenario: Uncovered scenario
    Given test
`;

      await writeFile(
        join(testDir, 'spec', 'features', 'incomplete-feature.feature'),
        featureContent
      );

      const coverageContent = {
        scenarios: [
          {
            name: 'Covered scenario',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-15',
                implMappings: [],
              },
            ],
          },
          {
            name: 'Uncovered scenario',
            testMappings: [],
          },
        ],
        stats: {
          totalScenarios: 2,
          coveredScenarios: 1,
          coveragePercent: 50,
        },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'incomplete-feature.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // When I run 'fspec review SPEC-001'
      const result = await review('SPEC-001', { cwd: testDir });

      // Then the output should show current workflow progress
      expect(result.output).toContain('testing');
      expect(result.output).toMatch(/Status|Current State/);

      // And the output should identify missing test coverage
      expect(result.output).toContain('Uncovered scenario');
      expect(result.output).toMatch(/50%|Coverage/);

      // And the output should suggest next steps for completion
      expect(result.output).toMatch(/Next Steps|Recommendations|Priority Actions/);
    });
  });
});
