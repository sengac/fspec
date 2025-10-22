/**
 * Test suite for: spec/features/remove-work-unit-id-tags-from-generate-scenarios.feature
 * Scenario: Validation rejects scenario-level work unit ID tags
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { validateTags } from '../validate-tags';

describe('Feature: Remove work unit ID tags from generate-scenarios', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Validation rejects scenario-level work unit ID tags', () => {
    it('should fail validation when scenario has work unit ID tag', async () => {
      // Given I have enabled the work unit ID tag validation rule
      // (validation is always enabled)

      // And I have a feature file with scenario-level work unit ID tag @AUTH-001
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      const featureContent = `@critical
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test validation
    So that work unit ID tags are only at feature level

  @AUTH-001
  Scenario: Invalid scenario with work unit ID tag
    Given I have a scenario
    When I run validation
    Then it should fail
`;

      await writeFile(
        join(testDir, 'spec/features/test-feature.feature'),
        featureContent
      );

      // Create minimal tags.json
      const tagsData = {
        categories: [
          {
            name: 'Phase Tags',
            tags: [{ name: '@critical', description: 'Phase 1' }],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: '',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: '',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: { title: '', order: [], example: '' },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: { title: '', examples: [] },
        statistics: {
          lastUpdated: new Date().toISOString(),
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: '',
        },
        validation: { rules: [], commands: [] },
        references: [],
      };

      await writeFile(
        join(testDir, 'spec/tags.json'),
        JSON.stringify(tagsData, null, 2)
      );

      // When I run `fspec validate-tags`
      const result = await validateTags({ cwd: testDir });

      // Then the validation should fail
      expect(result.invalidCount).toBeGreaterThan(0);

      // And the error message should indicate scenario-level work unit ID tags are not allowed
      const invalidResults = result.results.filter(r => !r.valid);
      expect(invalidResults.length).toBeGreaterThan(0);

      const hasWorkUnitError = invalidResults.some(r =>
        r.errors.some(
          err =>
            err.message.toLowerCase().includes('work unit') ||
            err.message.toLowerCase().includes('scenario') ||
            err.message.includes('AUTH-001')
        )
      );
      expect(hasWorkUnitError).toBe(true);

      // And the error should show which scenario has the invalid tag
      const hasScenarioReference = invalidResults.some(
        r =>
          r.file.includes('test-feature.feature') &&
          r.errors.some(
            err =>
              err.message.includes('Invalid scenario with work unit ID tag') ||
              err.message.includes('AUTH-001')
          )
      );
      expect(hasScenarioReference).toBe(true);
    });

    it('should pass validation when work unit ID tags are only at feature level', async () => {
      // Given I have a feature file with work unit ID tag at feature level only
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      const featureContent = `@critical
@AUTH-002
Feature: Valid Feature

  Background: User Story
    As a developer
    I want to test validation
    So that work unit ID tags work correctly

  Scenario: Valid scenario without work unit ID tag
    Given I have a scenario
    When I run validation
    Then it should pass
`;

      await writeFile(
        join(testDir, 'spec/features/valid-feature.feature'),
        featureContent
      );

      // Create minimal tags.json
      const tagsData = {
        categories: [
          {
            name: 'Phase Tags',
            tags: [{ name: '@critical', description: 'Phase 1' }],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: '',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: '',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: { title: '', order: [], example: '' },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: { title: '', examples: [] },
        statistics: {
          lastUpdated: new Date().toISOString(),
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: '',
        },
        validation: { rules: [], commands: [] },
        references: [],
      };

      await writeFile(
        join(testDir, 'spec/tags.json'),
        JSON.stringify(tagsData, null, 2)
      );

      // When I run `fspec validate-tags`
      const result = await validateTags({ cwd: testDir });

      // Then the validation should pass (or only have warnings about unregistered tags)
      // Work unit ID tags at feature level are allowed
      const invalidResults = result.results.filter(r => !r.valid);

      const hasWorkUnitScenarioError = invalidResults.some(r =>
        r.errors.some(
          err =>
            (err.message.toLowerCase().includes('work unit') &&
              err.message.toLowerCase().includes('scenario')) ||
            (err.message.includes('AUTH-002') &&
              err.message.toLowerCase().includes('scenario'))
        )
      );
      expect(hasWorkUnitScenarioError).toBe(false);
    });
  });
});
