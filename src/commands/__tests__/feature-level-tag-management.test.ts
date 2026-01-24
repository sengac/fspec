import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { addTagToFeature } from '../add-tag-to-feature';
import { removeTagFromFeature } from '../remove-tag-from-feature';
import { listFeatureTags } from '../list-feature-tags';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Feature-Level Tag Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('tag-management');
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

    // Create minimal tags.json for validation tests
    const minimalTags = {
      $schema: '../src/schemas/tags.schema.json',
      categories: [
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
          name: 'Component Tags',
          description: 'Component tags',
          required: true,
          tags: [
            { name: '@cli', description: 'CLI' },
            { name: '@authentication', description: 'Authentication' },
          ],
        },
        {
          name: 'Feature Group Tags',
          description: 'Feature group tags',
          required: true,
          tags: [
            { name: '@feature-management', description: 'Feature management' },
            { name: '@validation', description: 'Validation' },
          ],
        },
        {
          name: 'Technical Tags',
          description: 'Technical tags',
          required: false,
          tags: [
            { name: '@custom-tag', description: 'Custom tag' },
            { name: '@api', description: 'API features' },
            { name: '@security', description: 'Security features' },
          ],
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
      join(testDir, 'spec', 'tags.json'),
      JSON.stringify(minimalTags, null, 2)
    );
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Add single tag to feature file', () => {
    it('should add tag and show success message', async () => {
      // Given I have a feature file "spec/features/login.feature"
      // And the feature has tags @authentication @validation
      const featureContent = `@authentication
@validation
Feature: User Login

  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Login
    Given I am on the login page
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tags @authentication @validation @critical
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@critical');
      expect(updatedContent).toContain('@authentication');
      expect(updatedContent).toContain('@validation');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);

      // And the output should show "Added @critical to spec/features/login.feature"
      expect(result.message).toContain('Added @critical');
      expect(result.message).toContain('spec/features/login.feature');
    });
  });

  describe('Scenario: Add multiple tags to feature file', () => {
    it('should add multiple tags and show count', async () => {
      // Given I have a feature file "spec/features/login.feature"
      // And the feature has tags @authentication @validation
      const featureContent = `@authentication
@validation
Feature: User Login

  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @critical @security`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@critical', '@security'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tags @authentication @validation @critical @security
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@critical');
      expect(updatedContent).toContain('@authentication');
      expect(updatedContent).toContain('@validation');
      expect(updatedContent).toContain('@security');

      // And the output should show "Added @critical, @security to spec/features/login.feature"
      expect(result.message).toContain('Added @critical, @security');
    });
  });

  describe('Scenario: Prevent adding duplicate tag', () => {
    it('should error when tag already exists', async () => {
      // Given I have a feature file with tags @critical @critical
      const featureContent = `@cli
@critical
@critical
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @critical already exists on this feature"
      expect(result.error).toContain('@critical');
      expect(result.error).toContain('already exists');

      // And the feature tags should remain unchanged
      const unchangedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(unchangedContent).toBe(featureContent);
    });
  });

  describe('Scenario: Validate tag format when adding', () => {
    it('should reject invalid tag format', async () => {
      // Given I have a feature file with tags @critical
      const featureContent = `@cli
@critical
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature InvalidTag`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['InvalidTag'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Invalid tag format. Tags must start with @"
      expect(result.error).toContain('Invalid tag format');
      expect(result.error).toContain('Tags must start with @');

      // And the feature tags should remain unchanged
      const unchangedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(unchangedContent).toBe(featureContent);
    });
  });

  describe('Scenario: Add tag with registry validation', () => {
    it('should succeed when tag is registered', async () => {
      // Given I have a feature file with tags @critical
      // And the tag @custom-tag is registered in spec/tags.json
      const featureContent = `@cli
@critical
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @custom-tag --validate-registry`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@custom-tag'],
        { cwd: testDir, validateRegistry: true }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tags @critical @custom-tag
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@critical');
      expect(updatedContent).toContain('@custom-tag');
    });
  });

  describe('Scenario: Prevent adding unregistered tag with validation enabled', () => {
    it('should error when tag not in registry', async () => {
      // Given I have a feature file with tags @critical
      // And the tag @unregistered is NOT in spec/tags.json
      const featureContent = `@cli
@critical
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @unregistered --validate-registry`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@unregistered'],
        { cwd: testDir, validateRegistry: true }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @unregistered is not registered in spec/tags.json"
      expect(result.error).toContain('@unregistered');
      expect(result.error).toContain('not registered');
      expect(result.error).toContain('tags.json');

      // And the feature tags should remain unchanged
      const unchangedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(unchangedContent).toBe(featureContent);
    });
  });

  describe('Scenario: Remove single tag from feature file', () => {
    it('should remove tag and show success message', async () => {
      // Given I have a feature file with tags @critical @authentication @validation
      const featureContent = `@critical
@authentication
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec remove-tag-from-feature spec/features/login.feature @critical`
      const result = await removeTagFromFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tags @authentication @validation (without @critical)
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).toContain('@authentication');
      expect(updatedContent).toContain('@validation');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);

      // And the output should show "Removed @critical from spec/features/login.feature"
      expect(result.message).toContain('Removed @critical');
      expect(result.message).toContain('spec/features/login.feature');
    });
  });

  describe('Scenario: Remove multiple tags from feature file', () => {
    it('should remove multiple tags and show count', async () => {
      // Given I have a feature file with tags @critical @authentication @validation @wip
      const featureContent = `@critical
@authentication
@validation
@wip
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec remove-tag-from-feature spec/features/login.feature @critical @wip`
      const result = await removeTagFromFeature(
        'spec/features/login.feature',
        ['@critical', '@wip'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tags @authentication @validation (without @critical or @wip)
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).not.toContain('@wip');
      expect(updatedContent).toContain('@authentication');
      expect(updatedContent).toContain('@validation');

      // And the output should show "Removed @critical, @wip from spec/features/login.feature"
      expect(result.message).toContain('Removed @critical, @wip');
    });
  });

  describe('Scenario: Attempt to remove non-existent tag', () => {
    it('should error when tag does not exist', async () => {
      // Given I have a feature file with tags @authentication @validation
      const featureContent = `@authentication
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec remove-tag-from-feature spec/features/login.feature @critical`
      const result = await removeTagFromFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @critical not found on this feature"
      expect(result.error).toContain('@critical');
      expect(result.error).toContain('not found');

      // And the feature tags should remain unchanged
      const unchangedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(unchangedContent).toBe(featureContent);
    });
  });

  describe('Scenario: List all feature-level tags', () => {
    it('should list all tags', async () => {
      // Given I have a feature file with tags @critical @authentication @validation @api
      const featureContent = `@critical
@authentication
@validation
@api
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec list-feature-tags spec/features/login.feature`
      const result = await listFeatureTags('spec/features/login.feature', {
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show all tags
      expect(result.tags).toContain('@critical');
      expect(result.tags).toContain('@authentication');
      expect(result.tags).toContain('@validation');
      expect(result.tags).toContain('@api');
      expect(result.tags.length).toBe(4);
    });
  });

  describe('Scenario: List tags on feature with no tags', () => {
    it('should show "No tags found" message', async () => {
      // Given I have a feature file with no tags
      const featureContent = `Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec list-feature-tags spec/features/login.feature`
      const result = await listFeatureTags('spec/features/login.feature', {
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No tags found on this feature"
      expect(result.message).toContain('No tags found');
      expect(result.tags.length).toBe(0);
    });
  });

  describe('Scenario: Preserve scenario-level tags when modifying feature tags', () => {
    it('should preserve scenario tags unchanged', async () => {
      // Given I have a feature file with tags at feature level
      // And scenarios with tags @smoke and @regression at scenario level
      const featureContent = `@cli
@validation
Feature: Login

  @smoke
  Scenario: Quick test
    Given test

  @regression
  Scenario: Full test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tags @cli @validation @critical
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@cli');
      expect(updatedContent).toContain('@validation');
      expect(updatedContent).toContain('@critical');

      // And the scenario tags should remain @smoke and @regression
      expect(updatedContent).toContain('@smoke');
      expect(updatedContent).toContain('@regression');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Add tag to feature file without existing tags', () => {
    it('should create tags section with first tag', async () => {
      // Given I have a feature file with no tags
      const featureContent = `@cli
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have tag @critical
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@critical');
      expect(updatedContent).toContain('Feature: Login');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Remove all tags from feature file', () => {
    it('should remove all tags and leave clean feature', async () => {
      // Given I have a feature file with tags @critical @critical
      const featureContent = `@cli
@critical
@critical
@validation
Feature: Login

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec remove-tag-from-feature spec/features/login.feature @critical @critical`
      const result = await removeTagFromFeature(
        'spec/features/login.feature',
        ['@critical', '@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature should have no tags
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).toContain('Feature: Login');

      // And the file should remain valid Gherkin
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Handle file not found error', () => {
    it('should show error when file does not exist', async () => {
      // Given the file "spec/features/nonexistent.feature" does not exist
      // (file does not exist)

      // When I run `fspec add-tag-to-feature spec/features/nonexistent.feature @critical`
      const result = await addTagToFeature(
        'spec/features/nonexistent.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "File not found: spec/features/nonexistent.feature"
      expect(result.error).toContain('not found');
      expect(result.error).toContain('nonexistent.feature');
    });
  });

  describe('Scenario: Preserve file formatting after tag modification', () => {
    it('should maintain formatting including doc strings', async () => {
      // Given I have a properly formatted feature file
      const featureContent = `@cli
@validation
Feature: Login
  """
  Architecture notes:
  - Uses authentication system
  - Validates credentials
  """

  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
      const result = await addTagToFeature(
        'spec/features/login.feature',
        ['@critical'],
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );

      // And the file formatting should be preserved
      expect(updatedContent).toContain('Feature: Login');

      // And indentation should remain consistent
      expect(updatedContent).toContain('  Scenario: Test');
      expect(updatedContent).toContain('    Given test');

      // And doc strings should remain intact
      expect(updatedContent).toContain('"""');
      expect(updatedContent).toContain('Architecture notes:');
      expect(updatedContent).toContain('- Uses authentication system');
    });
  });
});
