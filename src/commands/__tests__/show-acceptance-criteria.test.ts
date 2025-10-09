import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { showAcceptanceCriteria } from '../show-acceptance-criteria';

describe('Feature: Show Acceptance Criteria by Tag', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-show-acs');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show acceptance criteria for single tag', () => {
    it('should show features, backgrounds, and scenarios', async () => {
      // Given I have feature files tagged @phase1
      const content = `@phase1
Feature: Test Feature

  Background: User Story
    As a user
    I want to test
    So that it works

  Scenario: First scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec show-acceptance-criteria --tag=@phase1`
      const result = await showAcceptanceCriteria({
        tags: ['@phase1'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show all feature names
      expect(result.features.length).toBe(1);
      expect(result.features[0].name).toBe('Test Feature');

      // And the output should show background user stories
      expect(result.features[0].background).toBeDefined();
      expect(result.features[0].background).toContain('As a user');

      // And the output should show all scenarios with their steps
      expect(result.features[0].scenarios.length).toBe(1);
      expect(result.features[0].scenarios[0].steps.length).toBe(3);
    });
  });

  describe('Scenario: Show acceptance criteria with multiple tags', () => {
    it('should only show features with all tags', async () => {
      // Given I have features tagged @phase1 @critical
      await writeFile(
        join(testDir, 'spec/features/both.feature'),
        `@phase1 @critical
Feature: Both Tags

  Scenario: Test
    Given test
`
      );

      // And I have features with only @phase1
      await writeFile(
        join(testDir, 'spec/features/one.feature'),
        `@phase1
Feature: One Tag

  Scenario: Test
    Given test
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@phase1 --tag=@critical`
      const result = await showAcceptanceCriteria({
        tags: ['@phase1', '@critical'],
        cwd: testDir,
      });

      // Then the output should only show features with both tags
      expect(result.features.length).toBe(1);
      expect(result.features[0].name).toBe('Both Tags');
    });
  });

  describe('Scenario: Format output as markdown', () => {
    it('should generate valid markdown', async () => {
      // Given I have a feature file "login.feature" tagged @auth
      const content = `@auth
Feature: Login

  Background: User Story
    As a user
    I want to login
    So that I can access my account

  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec show-acceptance-criteria --tag=@auth --format=markdown`
      const result = await showAcceptanceCriteria({
        tags: ['@auth'],
        format: 'markdown',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then the output should be valid markdown
      expect(result.output).toContain('# Login');

      // And the output should include scenarios as H2 headings
      expect(result.output).toContain('## Successful login');

      // And the output should include steps as bullet points
      expect(result.output).toContain('- **Given** I am on the login page');
    });
  });

  describe('Scenario: Format output as JSON', () => {
    it('should return valid JSON structure', async () => {
      // Given I have feature files tagged @critical
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        `@critical
Feature: Test

  Scenario: Test scenario
    Given test step
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@critical --format=json`
      const result = await showAcceptanceCriteria({
        tags: ['@critical'],
        format: 'json',
        cwd: testDir,
      });

      // Then the output should be valid JSON
      expect(result.success).toBe(true);

      // And each feature should have name, background, and scenarios properties
      expect(result.features[0]).toHaveProperty('name');
      expect(result.features[0]).toHaveProperty('scenarios');

      // And the JSON should be parseable by other tools
      const jsonString = JSON.stringify(result.features);
      const parsed = JSON.parse(jsonString);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  describe('Scenario: Show acceptance criteria when no features match', () => {
    it('should show message when no features found', async () => {
      // Given I have feature files without @deprecated tag
      await writeFile(
        join(testDir, 'spec/features/active.feature'),
        `@active
Feature: Active

  Scenario: Test
    Given test
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@deprecated`
      const result = await showAcceptanceCriteria({
        tags: ['@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No features found matching tags: @deprecated"
      expect(result.features.length).toBe(0);
      expect(result.message).toContain('No features found');
      expect(result.message).toContain('@deprecated');
    });
  });

  describe('Scenario: Include feature-level tags in output', () => {
    it('should show feature tags', async () => {
      // Given I have a feature file with tags @phase1 @critical @auth
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        `@phase1 @critical @auth
Feature: Test

  Scenario: Test
    Given test
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@phase1 --format=text`
      const result = await showAcceptanceCriteria({
        tags: ['@phase1'],
        cwd: testDir,
      });

      // Then the output should show the feature tags
      expect(result.features[0].tags).toBeDefined();
      expect(result.features[0].tags).toContain('@phase1');
      expect(result.features[0].tags).toContain('@critical');
      expect(result.features[0].tags).toContain('@auth');
    });
  });

  describe('Scenario: Handle features with no background', () => {
    it('should handle missing background section', async () => {
      // Given I have a feature file without a Background section
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        `@test
Feature: No Background

  Scenario: Test
    Given test
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@test`
      const result = await showAcceptanceCriteria({
        tags: ['@test'],
        cwd: testDir,
      });

      // Then the output should show the feature
      expect(result.features.length).toBe(1);

      // And the background section should be omitted
      expect(result.features[0].background).toBeUndefined();

      // And scenarios should still be displayed
      expect(result.features[0].scenarios.length).toBe(1);
    });
  });

  describe('Scenario: Handle features with no scenarios', () => {
    it('should show feature with no scenarios message', async () => {
      // Given I have a feature file with only a Feature line
      await writeFile(
        join(testDir, 'spec/features/empty.feature'),
        `@empty
Feature: Empty Feature
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@empty`
      const result = await showAcceptanceCriteria({
        tags: ['@empty'],
        cwd: testDir,
      });

      // Then the output should show the feature name
      expect(result.features[0].name).toBe('Empty Feature');

      // And the output should indicate "No scenarios defined"
      expect(result.features[0].scenarios.length).toBe(0);
    });
  });

  describe('Scenario: Export to file with --output option', () => {
    it('should write to specified file', async () => {
      // Given I have features tagged @phase1
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        `@phase1
Feature: Test

  Scenario: Test
    Given test
`
      );

      // When I run `fspec show-acceptance-criteria --tag=@phase1 --format=markdown --output=phase1-acs.md`
      const outputPath = join(testDir, 'phase1-acs.md');
      const result = await showAcceptanceCriteria({
        tags: ['@phase1'],
        format: 'markdown',
        output: outputPath,
        cwd: testDir,
      });

      // Then a file "phase1-acs.md" should be created
      const fileContent = await readFile(outputPath, 'utf-8');
      expect(fileContent).toBeDefined();

      // And the file should contain all acceptance criteria in markdown format
      expect(fileContent).toContain('# Test');

      // And the command output should show "Acceptance criteria written to phase1-acs.md"
      expect(result.message).toContain('written to');
      expect(result.message).toContain('phase1-acs.md');
    });
  });

  describe('Scenario: Show architecture notes from doc strings', () => {
    it('should include architecture notes', async () => {
      // Given I have a feature with architecture notes in triple-quoted doc string
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        `@test
Feature: With Architecture

  """
  Architecture notes:
  - This is an architecture note
  - Critical implementation requirement
  """

  Scenario: Test
    Given test
`
      );

      // When I run `fspec show-acceptance-criteria --format=text`
      const result = await showAcceptanceCriteria({ cwd: testDir });

      // Then the architecture notes should be displayed
      expect(result.features[0].description).toBeDefined();
      expect(result.features[0].description).toContain('Architecture notes');
    });
  });

  describe('Scenario: Count total scenarios shown', () => {
    it('should show count of scenarios', async () => {
      // Given I have features tagged @critical with 15 total scenarios
      for (let i = 1; i <= 5; i++) {
        let content = `@critical\nFeature: Feature ${i}\n\n`;
        for (let j = 1; j <= 3; j++) {
          content += `  Scenario: Scenario ${j}\n    Given test\n\n`;
        }
        await writeFile(
          join(testDir, 'spec/features', `f${i}.feature`),
          content
        );
      }

      // When I run `fspec show-acceptance-criteria --tag=@critical`
      const result = await showAcceptanceCriteria({
        tags: ['@critical'],
        cwd: testDir,
      });

      // Then the output should show "Showing acceptance criteria for 15 scenarios"
      expect(result.totalScenarios).toBe(15);
      expect(result.message).toContain('15 scenarios');
    });
  });
});
