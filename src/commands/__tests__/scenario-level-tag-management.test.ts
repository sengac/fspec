import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { addTagToScenario } from '../add-tag-to-scenario';
import { removeTagFromScenario } from '../remove-tag-from-scenario';
import { listScenarioTags } from '../list-scenario-tags';

describe('Feature: Scenario-Level Tag Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-scenario-tag-management');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

    // Create minimal tags.json for validation tests
    const minimalTags = {
      $schema: '../src/schemas/tags.schema.json',
      categories: [
        {
          name: 'Phase Tags',
          description: 'Phase tags',
          required: true,
          tags: [{ name: '@phase1', description: 'Phase 1' }],
        },
        {
          name: 'Component Tags',
          description: 'Component tags',
          required: true,
          tags: [{ name: '@authentication', description: 'Authentication' }],
        },
        {
          name: 'Priority Tags',
          description: 'Priority tags',
          required: false,
          tags: [
            { name: '@critical', description: 'Critical priority' },
            { name: '@high', description: 'High priority' },
          ],
        },
        {
          name: 'Test Type Tags',
          description: 'Test type tags',
          required: false,
          tags: [
            { name: '@smoke', description: 'Smoke test' },
            { name: '@regression', description: 'Regression test' },
          ],
        },
        {
          name: 'Technical Tags',
          description: 'Technical tags',
          required: false,
          tags: [
            { name: '@custom-tag', description: 'Custom tag' },
            { name: '@api', description: 'API features' },
          ],
        },
      ],
      combinationExamples: [],
      usageGuidelines: {
        requiredCombinations: { title: '', requirements: [], minimumExample: '' },
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
      join(testDir, 'spec', 'tags.json'),
      JSON.stringify(minimalTags, null, 2)
    );
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add single tag to scenario', () => {
    it('should add tag and show success message', async () => {
      // Given I have a feature file with scenario "Login with valid credentials"
      // And the scenario has no tags
      const featureContent = `@phase1
Feature: User Login

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @smoke`
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Login with valid credentials',
        ['@smoke'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should have tag @smoke
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('Scenario: Login with valid credentials');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);

      // And the output should show "Added @smoke to scenario 'Login with valid credentials'"
      expect(result.message).toContain('Added @smoke');
      expect(result.message).toContain('Login with valid credentials');
    });
  });

  describe('Scenario: Add multiple tags to scenario', () => {
    it('should add multiple tags and show count', async () => {
      // Given I have a feature file with scenario "Login with valid credentials"
      // And the scenario has tag @smoke
      const featureContent = `@phase1
Feature: User Login

  @smoke
  Scenario: Login with valid credentials
    Given I am on the login page
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run add multiple tags
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Login with valid credentials',
        ['@critical', '@regression'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should have tags @smoke @critical @regression
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('@critical');
      expect(updatedContent).toContain('@regression');

      // And the output should show "Added @critical, @regression"
      expect(result.message).toContain('Added @critical, @regression');
    });
  });

  describe('Scenario: Prevent adding duplicate tag to scenario', () => {
    it('should error when tag already exists', async () => {
      // Given I have a scenario with tags @smoke @critical
      const featureContent = `Feature: Login

  @smoke
  @critical
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run add duplicate tag
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Login',
        ['@smoke'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @smoke already exists on this scenario"
      expect(result.error).toContain('@smoke');
      expect(result.error).toContain('already exists');
    });
  });

  describe('Scenario: Validate tag format when adding to scenario', () => {
    it('should reject invalid tag format', async () => {
      // Given I have a scenario with tag @smoke
      const featureContent = `Feature: Login

  @smoke
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I try to add invalid tag
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Login',
        ['InvalidTag'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And error should mention invalid format
      expect(result.error).toContain('Invalid tag format');
      expect(result.error).toContain('@');
      expect(result.error).toContain('lowercase');
    });
  });

  describe('Scenario: Remove single tag from scenario', () => {
    it('should remove tag and show success message', async () => {
      // Given I have a scenario with tags @smoke @critical @regression
      const featureContent = `Feature: Login

  @smoke
  @critical
  @regression
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I remove @critical
      const result = await removeTagFromScenario(
        'spec/features/login.feature',
        'Login',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should have tags @smoke @regression
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('@regression');
      expect(updatedContent).not.toContain('@critical');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);

      // And the output should show "Removed @critical from scenario 'Login'"
      expect(result.message).toContain('Removed @critical');
      expect(result.message).toContain('Login');
    });
  });

  describe('Scenario: Remove multiple tags from scenario', () => {
    it('should remove multiple tags and show count', async () => {
      // Given I have a scenario with tags @smoke @critical @regression @wip
      const featureContent = `Feature: Login

  @smoke
  @critical
  @regression
  @wip
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I remove multiple tags
      const result = await removeTagFromScenario(
        'spec/features/login.feature',
        'Login',
        ['@critical', '@wip'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should have tags @smoke @regression
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('@regression');
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).not.toContain('@wip');

      // And the output should show "Removed @critical, @wip"
      expect(result.message).toContain('Removed @critical, @wip');
    });
  });

  describe('Scenario: Attempt to remove non-existent tag from scenario', () => {
    it('should error when tag does not exist', async () => {
      // Given I have a scenario with tags @smoke @regression
      const featureContent = `Feature: Login

  @smoke
  @regression
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I try to remove non-existent tag
      const result = await removeTagFromScenario(
        'spec/features/login.feature',
        'Login',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @critical not found on this scenario"
      expect(result.error).toContain('@critical');
      expect(result.error).toContain('not found');
    });
  });

  describe('Scenario: List all scenario-level tags', () => {
    it('should list all tags', async () => {
      // Given I have a scenario with tags @smoke @critical @regression @api
      const featureContent = `Feature: Login

  @smoke
  @critical
  @regression
  @api
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I list tags
      const result = await listScenarioTags(
        'spec/features/login.feature',
        'Login',
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show all tags
      expect(result.tags).toContain('@smoke');
      expect(result.tags).toContain('@critical');
      expect(result.tags).toContain('@regression');
      expect(result.tags).toContain('@api');
      expect(result.tags.length).toBe(4);
    });
  });

  describe('Scenario: List tags on scenario with no tags', () => {
    it('should show "No tags found" message', async () => {
      // Given I have a scenario with no tags
      const featureContent = `Feature: Login

  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I list tags
      const result = await listScenarioTags(
        'spec/features/login.feature',
        'Login',
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No tags found on this scenario"
      expect(result.message).toContain('No tags found');
      expect(result.tags.length).toBe(0);
    });
  });

  describe('Scenario: Preserve feature-level tags when modifying scenario tags', () => {
    it('should preserve feature tags unchanged', async () => {
      // Given I have a feature with tags @phase1 @authentication
      // And a scenario "Login" with tag @smoke
      const featureContent = `@phase1
@authentication
Feature: User Login

  @smoke
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I add tag to scenario
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Login',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should have tags @smoke @critical
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('@critical');

      // And the feature should still have tags @phase1 @authentication
      expect(updatedContent).toContain('@phase1');
      expect(updatedContent).toContain('@authentication');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Preserve other scenarios when modifying one scenario\'s tags', () => {
    it('should preserve other scenario tags unchanged', async () => {
      // Given I have a feature with two scenarios
      const featureContent = `Feature: Authentication

  @smoke
  Scenario: Login
    Given test

  @regression
  Scenario: Logout
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I add tag to "Login" scenario
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Login',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );

      // And scenario "Login" should have tags @smoke @critical
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('@critical');

      // And scenario "Logout" should still have tag @regression
      expect(updatedContent).toContain('@regression');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Handle scenario not found error', () => {
    it('should show error when scenario does not exist', async () => {
      // Given I have a feature file
      const featureContent = `Feature: Login

  Scenario: Existing
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I try to add tag to non-existent scenario
      const result = await addTagToScenario(
        'spec/features/login.feature',
        'Nonexistent',
        ['@smoke'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show scenario not found
      expect(result.error).toContain('Nonexistent');
      expect(result.error).toContain('not found');
    });
  });

  describe('Scenario: Handle file not found error', () => {
    it('should show error when file does not exist', async () => {
      // Given the file does not exist

      // When I try to add tag
      const result = await addTagToScenario(
        'spec/features/nonexistent.feature',
        'Login',
        ['@smoke'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show file not found
      expect(result.error).toContain('not found');
      expect(result.error).toContain('nonexistent.feature');
    });
  });

  describe('Scenario: Remove all tags from scenario', () => {
    it('should remove all tags and leave clean scenario', async () => {
      // Given I have a scenario with tags @smoke @critical
      const featureContent = `Feature: Login

  @smoke
  @critical
  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I remove all tags
      const result = await removeTagFromScenario(
        'spec/features/login.feature',
        'Login',
        ['@smoke', '@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should have no tags
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@smoke');
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).toContain('Scenario: Login');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);
    });
  });
});
