import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { registerTagJsonBacked } from '../register-tag-json-backed';

describe('Feature: Register Tag in JSON-Backed Registry', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-register-tag-json-backed');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Register new tag in Phase Tags category', () => {
    it('should add tag to tags.json and regenerate TAGS.md', async () => {
      // Given I have a valid file "spec/tags.json"
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tracking',
            required: true,
            tags: [
              { name: '@phase1', description: 'Phase 1: Foundation' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      // When I run register-tag command
      const result = await registerTagJsonBacked({
        tagName: '@phase7',
        category: 'Phase Tags',
        description: 'Phase 7: JSON-Backed Documentation',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And "spec/tags.json" should contain the new tag
      const updatedContent = await readFile(tagsFile, 'utf-8');
      const updatedData = JSON.parse(updatedContent);
      const phaseCategory = updatedData.categories.find((c: { name: string }) => c.name === 'Phase Tags');
      expect(phaseCategory).toBeDefined();
      expect(phaseCategory.tags).toHaveLength(2);
      expect(phaseCategory.tags[1].name).toBe('@phase7');
      expect(phaseCategory.tags[1].description).toBe('Phase 7: JSON-Backed Documentation');

      // And "spec/TAGS.md" should be regenerated
      const tagsMd = join(testDir, 'spec', 'TAGS.md');
      const mdContent = await readFile(tagsMd, 'utf-8');
      expect(mdContent).toContain('@phase7');
      expect(mdContent).toContain('Phase 7: JSON-Backed Documentation');
    });
  });

  describe('Scenario: Register tag with usage information', () => {
    it('should include usage field in tag definition', async () => {
      // Given I have a valid file "spec/tags.json"
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Technical Tags',
            description: 'Technical implementation tags',
            required: false,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      // When I run register-tag with usage parameter
      const result = await registerTagJsonBacked({
        tagName: '@json-backed',
        category: 'Technical Tags',
        description: 'JSON-backed documentation features',
        useCases: 'Features using JSON schemas and generators',
        cwd: testDir,
      });

      // Then "spec/tags.json" should contain the tag with useCases field
      expect(result.success).toBe(true);
      const content = await readFile(tagsFile, 'utf-8');
      const data = JSON.parse(content);
      const techCategory = data.categories.find((c: { name: string }) => c.name === 'Technical Tags');
      expect(techCategory.tags[0].useCases).toBe('Features using JSON schemas and generators');
    });
  });

  describe('Scenario: Fail if category does not exist', () => {
    it('should error when category does not exist in tags.json', async () => {
      // Given I have a valid file "spec/tags.json"
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tracking',
            required: true,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      const originalContent = await readFile(tagsFile, 'utf-8');

      // When I run register-tag with nonexistent category
      // Then the command should exit with code 1
      await expect(
        registerTagJsonBacked({
          tagName: '@new-tag',
          category: 'Nonexistent Category',
          description: 'Description',
          cwd: testDir,
        })
      ).rejects.toThrow("Category 'Nonexistent Category' not found");

      // And "spec/tags.json" should not be modified
      const unchangedContent = await readFile(tagsFile, 'utf-8');
      expect(unchangedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Fail if tag already exists', () => {
    it('should prevent duplicate tag registration', async () => {
      // Given I have a valid file "spec/tags.json"
      // And the tag "@phase1" already exists
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tracking',
            required: true,
            tags: [
              { name: '@phase1', description: 'Phase 1: Foundation' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      const originalContent = await readFile(tagsFile, 'utf-8');

      // When I run register-tag with existing tag
      // Then the command should exit with code 1
      await expect(
        registerTagJsonBacked({
          tagName: '@phase1',
          category: 'Phase Tags',
          description: 'Duplicate tag',
          cwd: testDir,
        })
      ).rejects.toThrow('Tag @phase1 already exists');

      // And "spec/tags.json" should not be modified
      const unchangedContent = await readFile(tagsFile, 'utf-8');
      expect(unchangedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Fail if tag name is invalid format', () => {
    it('should validate tag name format before registration', async () => {
      // Given I have a valid file "spec/tags.json"
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tracking',
            required: true,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      // When I run register-tag with invalid tag name (missing @)
      // Then the command should exit with code 1
      await expect(
        registerTagJsonBacked({
          tagName: 'phase1',
          category: 'Phase Tags',
          description: 'Missing @ prefix',
          cwd: testDir,
        })
      ).rejects.toThrow('Invalid tag name: must start with @ and contain only lowercase letters, numbers, and hyphens');

      // When I run register-tag with invalid tag name (uppercase)
      await expect(
        registerTagJsonBacked({
          tagName: '@Phase1',
          category: 'Phase Tags',
          description: 'Uppercase letters',
          cwd: testDir,
        })
      ).rejects.toThrow('Invalid tag name: must start with @ and contain only lowercase letters, numbers, and hyphens');
    });
  });

  describe('Scenario: Rollback if markdown generation fails', () => {
    it('should rollback JSON changes if MD generation fails', async () => {
      // Given I have a valid file "spec/tags.json"
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tracking',
            required: true,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      const originalContent = await readFile(tagsFile, 'utf-8');

      // When I run register-tag and markdown generation fails
      // Then the command should exit with code 1
      await expect(
        registerTagJsonBacked({
          tagName: '@new-tag',
          category: 'Phase Tags',
          description: 'Description',
          cwd: testDir,
          forceRegenerationFailure: true,
        })
      ).rejects.toThrow('Failed to regenerate TAGS.md');

      // And "spec/tags.json" should be rolled back to previous state
      const rolledBackContent = await readFile(tagsFile, 'utf-8');
      expect(rolledBackContent).toBe(originalContent);
    });
  });

  describe('Scenario: Update statistics after registering tag', () => {
    it('should update tag statistics in tags.json', async () => {
      // Given I have a valid file "spec/tags.json" with current statistics
      const tagsFile = join(testDir, 'spec', 'tags.json');
      const oldTimestamp = '2025-01-01T00:00:00Z';
      const initialTags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Technical Tags',
            description: 'Technical tags',
            required: false,
            tags: [],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: {
            title: 'Ordering',
            order: [],
            example: '',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: {
          title: 'Common Queries',
          examples: [],
        },
        statistics: {
          lastUpdated: oldTimestamp,
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [],
          commands: [],
        },
        references: [],
      };
      await writeFile(tagsFile, JSON.stringify(initialTags, null, 2));

      // When I run register-tag
      const result = await registerTagJsonBacked({
        tagName: '@new-tag',
        category: 'Technical Tags',
        description: 'New technical tag',
        cwd: testDir,
      });

      // Then "spec/tags.json" statistics should be updated
      expect(result.success).toBe(true);
      const content = await readFile(tagsFile, 'utf-8');
      const data = JSON.parse(content);

      // And the lastUpdated timestamp should be current
      expect(data.statistics.lastUpdated).not.toBe(oldTimestamp);
      const updatedTime = new Date(data.statistics.lastUpdated);
      const now = new Date();
      const timeDiff = now.getTime() - updatedTime.getTime();
      expect(timeDiff).toBeLessThan(5000); // Within 5 seconds
    });
  });
});
