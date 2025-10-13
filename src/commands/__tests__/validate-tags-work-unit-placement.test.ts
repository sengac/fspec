/**
 * Feature: spec/features/remove-work-unit-id-tags-from-generate-scenarios.feature
 *
 * This test file validates the acceptance criteria for work unit ID tag placement.
 * Specifically tests that validate-tags rejects scenario-level work unit ID tags.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { validateTags } from '../validate-tags';

describe('Feature: Remove work unit ID tags from generate-scenarios', () => {
  let testDir: string;
  let specDir: string;
  let featuresDir: string;
  let workUnitsFile: string;
  let tagsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    featuresDir = join(specDir, 'features');
    workUnitsFile = join(specDir, 'work-units.json');
    tagsFile = join(specDir, 'tags.json');

    // Create spec directory structure
    await mkdir(featuresDir, { recursive: true });

    // Create work-units.json
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {
            'AUTH-001': {
              id: 'AUTH-001',
              title: 'User Authentication',
              status: 'done',
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
            },
          },
          states: {
            backlog: [],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
            done: ['AUTH-001'],
            blocked: [],
          },
        },
        null,
        2
      )
    );

    // Create tags.json with required tags
    await writeFile(
      tagsFile,
      JSON.stringify(
        {
          categories: [
            {
              name: 'Phase Tags',
              tags: [{ name: '@phase1', description: 'Phase 1' }],
            },
            {
              name: 'Component Tags',
              tags: [{ name: '@cli', description: 'CLI component' }],
            },
            {
              name: 'Feature Group Tags',
              tags: [{ name: '@validation', description: 'Validation features' }],
            },
          ],
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

  describe('Scenario: Validation rejects scenario-level work unit ID tags', () => {
    it('should fail validation when scenario has work unit ID tag', async () => {
      // Given I have enabled the work unit ID tag validation rule
      // (The rule is always enabled in validate-tags.ts)

      // And I have a feature file with scenario-level work unit ID tag @AUTH-001
      const featureFile = join(featuresDir, 'test.feature');
      await writeFile(
        featureFile,
        `@phase1
@cli
@validation
@AUTH-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test validation
    So that tags are properly validated

  @AUTH-001
  Scenario: Test scenario with work unit ID tag
    Given I have a scenario
    When I run validation
    Then it should fail
`
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/test.feature',
        cwd: testDir,
      });

      // Then the validation should fail
      expect(result.invalidCount).toBe(1);
      expect(result.results[0].valid).toBe(false);

      // And the error message should indicate scenario-level work unit ID tags are not allowed
      const errors = result.results[0].errors;
      const workUnitError = errors.find(e =>
        e.message.includes('Work unit ID tag @AUTH-001 must be at feature level')
      );
      expect(workUnitError).toBeDefined();

      // And the error should show which scenario has the invalid tag
      expect(workUnitError?.message).toContain('feature level, not scenario level');
      expect(workUnitError?.suggestion).toContain('coverage files');
    });

    it('should pass validation when work unit ID tag is at feature level only', async () => {
      // Given I have a feature file with work unit ID tag at feature level
      const featureFile = join(featuresDir, 'test.feature');
      await writeFile(
        featureFile,
        `@phase1
@cli
@validation
@AUTH-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test validation
    So that tags are properly validated

  Scenario: Test scenario without work unit ID tag
    Given I have a scenario
    When I run validation
    Then it should pass
`
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/test.feature',
        cwd: testDir,
      });

      // Then the validation should pass
      expect(result.invalidCount).toBe(0);
      expect(result.results[0].valid).toBe(true);
      expect(result.results[0].errors.length).toBe(0);
    });
  });
});
