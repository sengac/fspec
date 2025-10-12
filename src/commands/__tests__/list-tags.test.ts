import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { listTags } from '../list-tags';

describe('Feature: List Registered Tags from Registry', () => {
  let testDir: string;
  let originalCwd: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    originalCwd = process.cwd();
    process.chdir(testDir);

    // Create tags.json with sample tags
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });

    const tagsJson = {
      $schema: '../src/schemas/tags.schema.json',
      categories: [
        {
          name: 'Phase Tags',
          description: 'Development phase tags',
          required: false,
          tags: [
            { name: '@phase1', description: 'Phase 1: Core features' },
            { name: '@phase2', description: 'Phase 2: Advanced features' },
            { name: '@phase3', description: 'Phase 3: Future features' },
          ],
        },
        {
          name: 'Component Tags',
          description: 'Component tags',
          required: false,
          tags: [
            { name: '@cli', description: 'CLI component' },
            { name: '@parser', description: 'Parser component' },
            { name: '@validator', description: 'Validator component' },
          ],
        },
        {
          name: 'Feature Group Tags',
          description: 'Feature group tags',
          required: false,
          tags: [
            { name: '@feature-management', description: 'Feature management' },
            { name: '@tag-management', description: 'Tag management' },
            { name: '@validation', description: 'Validation features' },
          ],
        },
        {
          name: 'Technical Tags',
          description: 'Technical tags',
          required: false,
          tags: [{ name: '@gherkin', description: 'Gherkin spec compliance' }],
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
      join(specDir, 'tags.json'),
      JSON.stringify(tagsJson, null, 2)
    );
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List all registered tags', () => {
    it('should display tags grouped by category', async () => {
      // Given I have a tags.json file with tags in multiple categories
      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display tags grouped by category
      expect(result.categories).toHaveLength(4);
      expect(result.categories.map(c => c.name)).toContain('Phase Tags');
      expect(result.categories.map(c => c.name)).toContain('Component Tags');
      expect(result.categories.map(c => c.name)).toContain(
        'Feature Group Tags'
      );

      // And each tag should be displayed with its description
      const phaseTags = result.categories.find(c => c.name === 'Phase Tags');
      expect(phaseTags?.tags).toHaveLength(3);
      expect(phaseTags?.tags[0]).toEqual({
        tag: '@phase1',
        description: 'Phase 1: Core features',
      });
    });
  });

  describe('Scenario: Filter tags by category', () => {
    it('should only show tags from specified category', async () => {
      // Given I have a tags.json file with tags in multiple categories
      // When I run `fspec list-tags --category="Phase Tags"`
      const result = await listTags({ category: 'Phase Tags', cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should only show tags from "Phase Tags" category
      expect(result.categories).toHaveLength(1);
      expect(result.categories[0].name).toBe('Phase Tags');

      // And the output should contain "@phase1", "@phase2", "@phase3"
      const tags = result.categories[0].tags.map(t => t.tag);
      expect(tags).toContain('@phase1');
      expect(tags).toContain('@phase2');
      expect(tags).toContain('@phase3');

      // And the output should not contain tags from other categories
      expect(tags).not.toContain('@cli');
      expect(tags).not.toContain('@validation');
    });
  });

  describe('Scenario: Auto-create tags.json when missing', () => {
    it('should auto-create tags.json with default structure when missing', async () => {
      // Given no tags.json file exists in spec/
      await rm(join(testDir, 'spec', 'tags.json'));

      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then it should succeed
      expect(result.success).toBe(true);

      // And tags.json should be auto-created with default categories
      expect(result.categories).toHaveLength(9); // 9 default categories
      expect(result.categories.map(c => c.name)).toContain('Phase Tags');
      expect(result.categories.map(c => c.name)).toContain('Component Tags');
      expect(result.categories.map(c => c.name)).toContain(
        'Feature Group Tags'
      );
    });
  });

  describe('Scenario: Display tag count per category', () => {
    it('should show count for each category', async () => {
      // Given I have a tags.json file with tags
      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the output should show the count for each category
      const phaseTags = result.categories.find(c => c.name === 'Phase Tags');
      expect(phaseTags?.tags).toHaveLength(3);

      const componentTags = result.categories.find(
        c => c.name === 'Component Tags'
      );
      expect(componentTags?.tags).toHaveLength(3);

      const featureGroupTags = result.categories.find(
        c => c.name === 'Feature Group Tags'
      );
      expect(featureGroupTags?.tags).toHaveLength(3);
    });
  });

  describe('Scenario: List tags in alphabetical order within category', () => {
    it('should display tags alphabetically', async () => {
      // Given I have Phase Tags: "@phase3", "@phase1", "@phase2"
      // When I run `fspec list-tags --category="Phase Tags"`
      const result = await listTags({ category: 'Phase Tags', cwd: testDir });

      // Then the tags should be displayed in alphabetical order
      const tags = result.categories[0].tags.map(t => t.tag);
      expect(tags).toEqual(['@phase1', '@phase2', '@phase3']);
    });
  });

  describe('Scenario: Handle invalid category name', () => {
    it('should error with available categories', async () => {
      // Given I have a tags.json file
      // When I run `fspec list-tags --category="Invalid Category"`
      await expect(
        listTags({ category: 'Invalid Category', cwd: testDir })
      ).rejects.toThrow();

      // Then the error should contain available categories
      try {
        await listTags({ category: 'Invalid Category', cwd: testDir });
      } catch (error: any) {
        expect(error.message).toContain('Category not found');
        expect(error.message).toContain('Invalid Category');
      }
    });
  });

  describe('Scenario: Show all categories even if empty', () => {
    it('should show empty categories with 0 tags', async () => {
      // Given I have a tags.json file with only Phase Tags populated
      const minimalTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [{ name: '@phase1', description: 'Phase 1' }],
          },
          {
            name: 'Component Tags',
            description: 'Component tags',
            required: false,
            tags: [],
          },
          {
            name: 'Feature Group Tags',
            description: 'Feature group tags',
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

      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(minimalTags, null, 2)
      );

      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the output should show all category headers
      expect(result.categories.map(c => c.name)).toContain('Phase Tags');
      expect(result.categories.map(c => c.name)).toContain('Component Tags');
      expect(result.categories.map(c => c.name)).toContain(
        'Feature Group Tags'
      );

      // And empty categories should show 0 tags
      const componentTags = result.categories.find(
        c => c.name === 'Component Tags'
      );
      expect(componentTags?.tags).toHaveLength(0);
    });
  });

  describe('Scenario: Display tag descriptions with wrapping', () => {
    it('should display long descriptions without truncation', async () => {
      // Given I have a tag with a long description
      const longDescTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Technical Tags',
            description: 'Technical tags',
            required: false,
            tags: [
              {
                name: '@long-desc',
                description:
                  'This is a very long description that explains the purpose and usage of tag',
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

      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(longDescTags, null, 2)
      );

      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the description should be displayed without truncation
      const technicalTags = result.categories.find(
        c => c.name === 'Technical Tags'
      );
      const longDescTag = technicalTags?.tags.find(t => t.tag === '@long-desc');
      expect(longDescTag?.description).toBe(
        'This is a very long description that explains the purpose and usage of tag'
      );
    });
  });

  describe('Scenario: AI agent workflow - discover available tags', () => {
    it('should support tag discovery workflow', async () => {
      // Given I am an AI agent working on a new feature specification
      // And I need to know which tags to use
      // When I run `fspec list-tags --category="Feature Group Tags"`
      const categoryResult = await listTags({
        category: 'Feature Group Tags',
        cwd: testDir,
      });

      // Then I should see all available feature group tags
      expect(categoryResult.categories[0].tags).toHaveLength(3);
      const tags = categoryResult.categories[0].tags.map(t => t.tag);
      expect(tags).toContain('@feature-management');
      expect(tags).toContain('@tag-management');
      expect(tags).toContain('@validation');

      // And when I run `fspec list-tags`
      const allResult = await listTags({ cwd: testDir });

      // Then I can see the complete tag vocabulary organized by category
      expect(allResult.categories.length).toBeGreaterThan(1);
      expect(allResult.success).toBe(true);
    });
  });

  describe('Scenario: Compare with validate-tags integration', () => {
    it('should show tags that validate-tags will accept', async () => {
      // Given I have tags registered in tags.json
      // When I run `fspec list-tags` and see tag "@custom-tag"
      const result = await listTags({ cwd: testDir });

      // Then these tags should be recognized by validate-tags
      const allTags = result.categories.flatMap(c => c.tags.map(t => t.tag));
      expect(allTags).toContain('@phase1');
      expect(allTags).toContain('@cli');
      expect(allTags).toContain('@validation');

      // And the tag ecosystem remains consistent
      expect(allTags.every(tag => tag.startsWith('@'))).toBe(true);
    });
  });

  describe('Scenario: JSON-backed workflow - read from source of truth', () => {
    it('should load tags from tags.json', async () => {
      // Given I have a valid tags.json file with multiple categories
      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the command should load tags from spec/tags.json
      expect(result.success).toBe(true);

      // And tags should be displayed grouped by category
      expect(result.categories.length).toBeGreaterThan(0);

      // And each category should show its tag count
      result.categories.forEach(category => {
        expect(category.tags).toBeDefined();
        expect(Array.isArray(category.tags)).toBe(true);
      });

      // And tags should be sorted alphabetically within categories
      const phaseTags = result.categories.find(c => c.name === 'Phase Tags');
      const tagNames = phaseTags?.tags.map(t => t.tag) || [];
      const sortedTagNames = [...tagNames].sort();
      expect(tagNames).toEqual(sortedTagNames);

      // And the command should exit with code 0
      expect(result.success).toBe(true);
    });
  });
});
