import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { updateTag } from '../update-tag';

describe('Feature: Update Tag in Registry', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-update-tag');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Update tag description only', () => {
    it('should update only description and preserve category', async () => {
      // Given I have a tag @phase1 registered in TAGS.md with description "Phase 1 features"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: false,
            tags: [
              {
                name: '@phase1',
                description: 'Phase 1 features',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @phase1 --description="Phase 1 - Core validation and feature management"`
      const result = await updateTag({
        tag: '@phase1',
        description: 'Phase 1 - Core validation and feature management',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Successfully updated @phase1"
      expect(result.message).toContain('Successfully updated @phase1');

      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      // And the tag @phase1 description should be updated in TAGS.md
      const phaseCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Phase Tags'
      );
      const phase1Tag = phaseCategory.tags.find(
        (t: any) => t.name === '@phase1'
      );
      expect(phase1Tag.description).toBe('Phase 1 - Core validation and feature management');

      // And the tag @phase1 category should remain unchanged
      expect(phaseCategory.name).toBe('Phase Tags');
    });
  });

  describe('Scenario: Update tag category only', () => {
    it('should move tag to new category', async () => {
      // Given I have a tag @deprecated registered in category "Status Tags"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Status Tags',
            description: 'Status tags',
            required: false,
            tags: [
              {
                name: '@deprecated',
                description: 'Deprecated feature',
              },
            ],
          },
          {
            name: 'Tag Categories',
            description: 'Tag categories',
            required: false,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @deprecated --category="Tag Categories"`
      const result = await updateTag({
        tag: '@deprecated',
        category: 'Tag Categories',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      // And the tag @deprecated should be moved to category "Tag Categories"
      const tagCategoriesCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Tag Categories'
      );
      expect(tagCategoriesCategory).toBeDefined();
      const deprecatedTag = tagCategoriesCategory.tags.find(
        (t: any) => t.name === '@deprecated'
      );
      expect(deprecatedTag).toBeDefined();

      // And the tag @deprecated description should remain unchanged
      expect(deprecatedTag.description).toBe('Deprecated feature');

      // And the tag should be removed from Status Tags
      const statusCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Status Tags'
      );
      expect(statusCategory.tags).toHaveLength(0);
    });
  });

  describe('Scenario: Update both category and description', () => {
    it('should update both fields simultaneously', async () => {
      // Given I have a tag @phase1 registered with category "Tag Categories" and description "Phase 1"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Tag Categories',
            description: 'Tag categories',
            required: false,
            tags: [
              {
                name: '@phase1',
                description: 'Phase 1',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @phase1 --category="Tag Categories" --description="Phase 1 - Core validation and feature management"`
      const result = await updateTag({
        tag: '@phase1',
        category: 'Tag Categories',
        description: 'Phase 1 - Core validation and feature management',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      // And the tag @phase1 category should be "Tag Categories"
      const tagCategoriesCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Tag Categories'
      );
      expect(tagCategoriesCategory).toBeDefined();

      // And the tag @phase1 description should be "Phase 1 - Core validation and feature management"
      const phase1Tag = tagCategoriesCategory.tags.find(
        (t: any) => t.name === '@phase1'
      );
      expect(phase1Tag).toBeDefined();
      expect(phase1Tag.description).toBe('Phase 1 - Core validation and feature management');
    });
  });

  describe('Scenario: Attempt to update non-existent tag', () => {
    it('should fail when tag does not exist', async () => {
      // Given I do not have a tag @nonexistent in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: false,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @nonexistent --description="New description"`
      const result = await updateTag({
        tag: '@nonexistent',
        description: 'New description',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @nonexistent not found in registry"
      expect(result.error).toContain('Tag @nonexistent not found in registry');
    });
  });

  describe('Scenario: Update tag with invalid category', () => {
    it('should fail with non-existent category', async () => {
      // Given I have a tag @phase1 registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: false,
            tags: [
              {
                name: '@phase1',
                description: 'Phase 1',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @phase1 --category="Invalid Category"`
      const result = await updateTag({
        tag: '@phase1',
        category: 'Invalid Category',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Invalid category: Invalid Category"
      expect(result.error).toContain('Invalid category: Invalid Category');

      // And the output should list available categories
      expect(result.error).toContain('Available categories:');
      expect(result.error).toContain('Phase Tags');
    });
  });

  describe('Scenario: Update tag without any changes', () => {
    it('should handle no-op update gracefully', async () => {
      // Given I have a tag @phase1 registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: false,
            tags: [
              {
                name: '@phase1',
                description: 'Phase 1',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @phase1` without --category or --description
      const result = await updateTag({
        tag: '@phase1',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "No updates specified. Use --category and/or --description"
      expect(result.error).toContain('No updates specified. Use --category and/or --description');
    });
  });

  describe('Scenario: Update tag preserves other tags', () => {
    it('should preserve other tags in registry', async () => {
      // Given I have multiple tags in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: false,
            tags: [
              {
                name: '@phase1',
                description: 'Phase 1',
              },
              {
                name: '@phase2',
                description: 'Phase 2',
              },
            ],
          },
          {
            name: 'Technical Tags',
            description: 'Technical tags',
            required: false,
            tags: [
              {
                name: '@test-tag',
                description: 'Test tag',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @phase1 --description="Updated description"`
      const result = await updateTag({
        tag: '@phase1',
        description: 'Updated description',
        cwd: testDir,
      });

      // Then only @phase1 should be modified
      expect(result.success).toBe(true);

      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      const phaseCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Phase Tags'
      );
      const phase1Tag = phaseCategory.tags.find(
        (t: any) => t.name === '@phase1'
      );
      expect(phase1Tag.description).toBe('Updated description');

      // And all other tags should remain unchanged
      const phase2Tag = phaseCategory.tags.find(
        (t: any) => t.name === '@phase2'
      );
      expect(phase2Tag.description).toBe('Phase 2');

      const technicalCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Technical Tags'
      );
      const testTag = technicalCategory.tags.find(
        (t: any) => t.name === '@test-tag'
      );
      expect(testTag.description).toBe('Test tag');

      // And the TAGS.md structure should be preserved
      expect(updatedTagsJson.categories).toHaveLength(2);
    });
  });

  describe('Scenario: Update tag handles special characters in description', () => {
    it('should handle special characters correctly', async () => {
      // Given I have a tag @auth registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Feature Tags',
            description: 'Feature tags',
            required: false,
            tags: [
              {
                name: '@auth',
                description: 'Authentication',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @auth --description="Authentication & authorization with OAuth2.0"`
      const result = await updateTag({
        tag: '@auth',
        description: 'Authentication & authorization with OAuth2.0',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      const featureCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Feature Tags'
      );
      const authTag = featureCategory.tags.find(
        (t: any) => t.name === '@auth'
      );

      // And the description should contain "&" and "2.0"
      expect(authTag.description).toBe('Authentication & authorization with OAuth2.0');
      expect(authTag.description).toContain('&');
      expect(authTag.description).toContain('2.0');

      // And the markdown should be properly escaped
      const tagsMdContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsMdContent).toContain('Authentication & authorization with OAuth2.0');
    });
  });

  describe('Scenario: JSON-backed workflow - modify JSON and regenerate MD', () => {
    it('should update tags.json and regenerate TAGS.md', async () => {
      // Given I have a valid tags.json file with @test-tag in "Technical Tags"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Technical Tags',
            description: 'Technical tags',
            required: false,
            tags: [
              {
                name: '@test-tag',
                description: 'Original test tag description',
              },
              {
                name: '@other-tag',
                description: 'Other tag description',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: '', requirements: [], minimumExample: '' },
          recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // When I run `fspec update-tag @test-tag --description="Updated test tag description"`
      const result = await updateTag({
        tag: '@test-tag',
        description: 'Updated test tag description',
        cwd: testDir,
      });

      // Then the tags.json file should be updated
      expect(result.success).toBe(true);

      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      // And the tags.json should validate against tags.schema.json
      expect(updatedTagsJson.categories).toBeDefined();

      // And the @test-tag description in tags.json should be "Updated test tag description"
      const technicalCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Technical Tags'
      );
      expect(technicalCategory).toBeDefined();

      const testTag = technicalCategory.tags.find(
        (t: any) => t.name === '@test-tag'
      );
      expect(testTag).toBeDefined();
      expect(testTag.description).toBe('Updated test tag description');

      // And the @test-tag should remain in "Technical Tags" category
      expect(technicalCategory.tags).toContainEqual({
        name: '@test-tag',
        description: 'Updated test tag description',
      });

      // And other tags in tags.json should be preserved
      const otherTag = technicalCategory.tags.find(
        (t: any) => t.name === '@other-tag'
      );
      expect(otherTag).toBeDefined();
      expect(otherTag.description).toBe('Other tag description');

      // And TAGS.md should be regenerated from tags.json
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );

      // And TAGS.md should contain the updated description
      expect(tagsContent).toContain('Updated test tag description');

      // And TAGS.md should have the auto-generation warning header
      expect(tagsContent).toContain(
        '<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->'
      );
      expect(tagsContent).toContain('<!-- DO NOT EDIT THIS FILE DIRECTLY -->');
    });
  });
});
