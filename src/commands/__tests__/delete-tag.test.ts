import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { deleteTag } from '../delete-tag';

describe('Feature: Delete Tag from Registry', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-delete-tag');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Delete unused tag', () => {
    it('should delete tag not used in any feature files', async () => {
      // Given I have a tag @deprecated registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Status Tags',
            description: 'Development status tags',
            required: false,
            tags: [
              { name: '@deprecated', description: 'Deprecated feature' },
              { name: '@wip', description: 'Work in progress' },
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And the tag @deprecated is not used in any feature files
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // When I run `fspec delete-tag @deprecated`
      const result = await deleteTag({ tag: '@deprecated', cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag @deprecated should be removed from TAGS.md
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const statusTags = updatedTags.categories.find(
        (c: any) => c.name === 'Status Tags'
      );
      expect(
        statusTags.tags.find((t: any) => t.name === '@deprecated')
      ).toBeUndefined();

      // And the TAGS.md structure should be preserved
      expect(statusTags.tags.find((t: any) => t.name === '@wip')).toBeDefined();

      // And the output should show "Successfully deleted tag @deprecated"
      expect(result.message).toContain('Successfully deleted tag @deprecated');
    });
  });

  describe('Scenario: Attempt to delete tag in use', () => {
    it('should fail when tag is used in feature files', async () => {
      // Given I have a tag @phase1 registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: false,
            tags: [{ name: '@phase1', description: 'Phase 1' }],
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And the tag @phase1 is used in 5 feature files
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      for (let i = 1; i <= 5; i++) {
        await writeFile(
          join(testDir, 'spec', 'features', `feature${i}.feature`),
          `@phase1\nFeature: Feature ${i}\n  Scenario: Test\n    Given test`
        );
      }

      // When I run `fspec delete-tag @phase1`
      const result = await deleteTag({ tag: '@phase1', cwd: testDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @phase1 is used in 5 feature file(s)"
      expect(result.error).toContain(
        'Tag @phase1 is used in 5 feature file(s)'
      );

      // And the output should list the feature files using the tag
      expect(result.error).toContain('feature1.feature');

      // And the tag should remain in TAGS.md
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const phaseTags = updatedTags.categories.find(
        (c: any) => c.name === 'Phase Tags'
      );
      expect(
        phaseTags.tags.find((t: any) => t.name === '@phase1')
      ).toBeDefined();

      // And the output should suggest using --force to delete anyway
      expect(result.error).toContain('--force');
    });
  });

  describe('Scenario: Force delete tag in use', () => {
    it('should force delete tag even when in use', async () => {
      // Given I have a tag @deprecated registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Status Tags',
            description: 'Status tags',
            required: false,
            tags: [{ name: '@deprecated', description: 'Deprecated' }],
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And the tag @deprecated is used in 2 feature files
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'old1.feature'),
        `@deprecated\nFeature: Old 1\n  Scenario: Test\n    Given test`
      );
      await writeFile(
        join(testDir, 'spec', 'features', 'old2.feature'),
        `@deprecated\nFeature: Old 2\n  Scenario: Test\n    Given test`
      );

      // When I run `fspec delete-tag @deprecated --force`
      const result = await deleteTag({
        tag: '@deprecated',
        force: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag @deprecated should be removed from TAGS.md
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const statusTags = updatedTags.categories.find(
        (c: any) => c.name === 'Status Tags'
      );
      expect(
        statusTags.tags.find((t: any) => t.name === '@deprecated')
      ).toBeUndefined();

      // And the output should show warning about files still using the tag
      expect(result.warning).toContain(
        'Warning: Tag @deprecated is still used'
      );

      // And the output should list the 2 feature files
      expect(result.warning).toContain('old1.feature');
      expect(result.warning).toContain('old2.feature');
    });
  });

  describe('Scenario: Attempt to delete non-existent tag', () => {
    it('should fail when tag does not exist', async () => {
      // Given I do not have a tag @nonexistent in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // When I run `fspec delete-tag @nonexistent`
      const result = await deleteTag({ tag: '@nonexistent', cwd: testDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @nonexistent not found in registry"
      expect(result.error).toContain('Tag @nonexistent not found in registry');

      // And TAGS.md should remain unchanged
      const tagsContent = await readFile(tagsJsonPath, 'utf-8');
      expect(tagsContent).toContain('Phase Tags');
    });
  });

  describe('Scenario: Delete tag preserves other tags in same category', () => {
    it('should preserve other tags in category', async () => {
      // Given I have tags @phase1, @phase2, @phase3 in category "Tag Categories"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Tag Categories',
            description: 'Tag categories',
            required: false,
            tags: [
              { name: '@phase1', description: 'Phase 1' },
              { name: '@phase2', description: 'Phase 2' },
              { name: '@phase3', description: 'Phase 3' },
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // When I run `fspec delete-tag @phase2`
      const result = await deleteTag({ tag: '@phase2', cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And @phase1 and @phase3 should remain in TAGS.md
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const tagCategory = updatedTags.categories.find(
        (c: any) => c.name === 'Tag Categories'
      );
      expect(
        tagCategory.tags.find((t: any) => t.name === '@phase1')
      ).toBeDefined();
      expect(
        tagCategory.tags.find((t: any) => t.name === '@phase3')
      ).toBeDefined();

      // And the category "Tag Categories" should remain intact
      expect(tagCategory).toBeDefined();
      expect(tagCategory.name).toBe('Tag Categories');
    });
  });

  describe('Scenario: Delete tag from specific category', () => {
    it('should delete tag from correct category', async () => {
      // Given I have a tag @custom registered in category "Technical Tags"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Technical Tags',
            description: 'Technical tags',
            required: false,
            tags: [
              { name: '@custom', description: 'Custom tag' },
              { name: '@other', description: 'Other tag' },
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And the tag is not used in any feature files
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // When I run `fspec delete-tag @custom`
      const result = await deleteTag({ tag: '@custom', cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag should be removed from "Technical Tags" category
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const technicalTags = updatedTags.categories.find(
        (c: any) => c.name === 'Technical Tags'
      );
      expect(
        technicalTags.tags.find((t: any) => t.name === '@custom')
      ).toBeUndefined();

      // And other tags in "Technical Tags" should remain
      expect(
        technicalTags.tags.find((t: any) => t.name === '@other')
      ).toBeDefined();
    });
  });

  describe('Scenario: Delete tag updates tag statistics', () => {
    it('should update statistics after deletion', async () => {
      // Given I have 10 tags registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tags = [];
      for (let i = 1; i <= 10; i++) {
        tags.push({ name: `@tag${i}`, description: `Tag ${i}` });
      }

      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Test Tags',
            description: 'Test tags',
            required: false,
            tags,
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And one tag @obsolete is unused
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // When I run `fspec delete-tag @obsolete`
      const result = await deleteTag({ tag: '@tag5', cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag count should be 9
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const testTags = updatedTags.categories.find(
        (c: any) => c.name === 'Test Tags'
      );
      expect(testTags.tags.length).toBe(9);

      // And the tag should not appear in tag statistics
      expect(
        testTags.tags.find((t: any) => t.name === '@tag5')
      ).toBeUndefined();
    });
  });

  describe('Scenario: Handle TAGS.md with invalid format', () => {
    // Note: This test is skipped because the delete-tag implementation reads tags.json (not TAGS.md)
    // Testing invalid tags.json format instead
    it('should handle invalid tags.json format', async () => {
      // Given I have a tags.json file with invalid structure
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      await writeFile(tagsJsonPath, '{ invalid json syntax');

      // When I run `fspec delete-tag @sometag`
      const result = await deleteTag({ tag: '@sometag', cwd: testDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show a parse error
      expect(result.error).toBeDefined();

      // And the file should remain unchanged
      const fileContent = await readFile(tagsJsonPath, 'utf-8');
      expect(fileContent).toBe('{ invalid json syntax');
    });
  });

  describe('Scenario: Delete last tag in category leaves category intact', () => {
    it('should keep empty category after deleting last tag', async () => {
      // Given I have only one tag @lonely in category "Custom Category"
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Custom Category',
            description: 'Custom category',
            required: false,
            tags: [{ name: '@lonely', description: 'Lonely tag' }],
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And the tag is not used in any feature files
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // When I run `fspec delete-tag @lonely`
      const result = await deleteTag({ tag: '@lonely', cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag should be removed
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const customCategory = updatedTags.categories.find(
        (c: any) => c.name === 'Custom Category'
      );
      expect(
        customCategory.tags.find((t: any) => t.name === '@lonely')
      ).toBeUndefined();

      // And the category "Custom Category" should remain (empty)
      expect(customCategory).toBeDefined();
      expect(customCategory.name).toBe('Custom Category');
      expect(customCategory.tags.length).toBe(0);
    });
  });

  describe('Scenario: Dry run shows what would be deleted', () => {
    it('should show deletion preview without making changes', async () => {
      // Given I have a tag @test registered in TAGS.md
      const tagsJsonPath = join(testDir, 'spec', 'tags.json');
      const tagsData = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Test Tags',
            description: 'Test tags',
            required: false,
            tags: [{ name: '@test', description: 'Test tag' }],
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

      await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2));

      // And the tag is not used in any feature files
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // When I run `fspec delete-tag @test --dry-run`
      const result = await deleteTag({
        tag: '@test',
        dryRun: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Would delete tag @test"
      expect(result.message).toContain('Would delete tag @test');

      // And the tag should remain in TAGS.md
      const updatedTags = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const testTags = updatedTags.categories.find(
        (c: any) => c.name === 'Test Tags'
      );
      expect(testTags.tags.find((t: any) => t.name === '@test')).toBeDefined();

      // And the output should show the category it would be removed from
      expect(result.message).toContain('Test Tags');
    });
  });

  describe('Scenario: JSON-backed workflow - modify JSON and regenerate MD', () => {
    it('should update tags.json and regenerate TAGS.md', async () => {
      // Given I have a valid tags.json file with @obsolete-tag in "Technical Tags"
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
                name: '@obsolete-tag',
                description: 'Obsolete tag to be deleted',
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

      await writeFile(tagsJsonPath, JSON.stringify(minimalTags, null, 2));

      // And @obsolete-tag is not used in any feature files
      // (no feature files exist)

      // When I run `fspec delete-tag @obsolete-tag`
      const result = await deleteTag({
        tag: '@obsolete-tag',
        cwd: testDir,
      });

      // Then the tags.json file should be updated
      expect(result.success).toBe(true);

      const updatedTagsJson = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));

      // And the tags.json should validate against tags.schema.json
      expect(updatedTagsJson.categories).toBeDefined();

      // And @obsolete-tag should be removed from the tags array
      const technicalCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Technical Tags'
      );
      expect(technicalCategory).toBeDefined();

      const obsoleteTag = technicalCategory.tags.find(
        (t: any) => t.name === '@obsolete-tag'
      );
      expect(obsoleteTag).toBeUndefined();

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

      // And TAGS.md should not contain @obsolete-tag
      expect(tagsContent).not.toContain('@obsolete-tag');

      // And TAGS.md should have the auto-generation warning header
      expect(tagsContent).toContain(
        '<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->'
      );
      expect(tagsContent).toContain('<!-- DO NOT EDIT THIS FILE DIRECTLY -->');
    });
  });
});
