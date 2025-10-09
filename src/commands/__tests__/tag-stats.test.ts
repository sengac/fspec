import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tagStats } from '../tag-stats';

describe('Feature: Show Tag Usage Statistics', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-tag-stats');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show overall tag statistics', () => {
    it('should show total files, unique tags, and total occurrences', async () => {
      // Given I have 5 feature files with various tags
      const features = [
        { name: 'f1.feature', tags: '@phase1 @cli' },
        { name: 'f2.feature', tags: '@phase1 @parser' },
        { name: 'f3.feature', tags: '@phase2 @cli' },
        { name: 'f4.feature', tags: '@phase2 @formatter' },
        { name: 'f5.feature', tags: '@phase3' },
      ];

      for (const feature of features) {
        await writeFile(
          join(testDir, 'spec/features', feature.name),
          `${feature.tags}\nFeature: Test\n  Scenario: Test\n    Given test\n`
        );
      }

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show total number of feature files
      expect(result.totalFiles).toBe(5);

      // And the output should show total number of unique tags used
      expect(result.uniqueTags).toBe(6);

      // And the output should show total tag occurrences
      expect(result.totalOccurrences).toBe(9);
    });
  });

  describe('Scenario: Show per-category tag statistics', () => {
    it('should group statistics by category with sorted counts', async () => {
      // Given I have feature files using tags from multiple categories
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [
              { name: '@phase1', description: 'Phase 1 features' },
              { name: '@phase2', description: 'Phase 2 features' },
            ],
          },
          {
            name: 'Component Tags',
            description: 'Architectural components',
            required: false,
            tags: [
              { name: '@cli', description: 'CLI component' },
              { name: '@parser', description: 'Parser component' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@phase1 @cli\nFeature: F1\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f2.feature'),
        '@phase1 @parser\nFeature: F2\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f3.feature'),
        '@phase2 @cli\nFeature: F3\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the output should group statistics by category
      expect(result.categories).toBeDefined();
      const phaseCategory = result.categories.find(
        c => c.name === 'Phase Tags'
      );
      const componentCategory = result.categories.find(
        c => c.name === 'Component Tags'
      );

      expect(phaseCategory).toBeDefined();
      expect(componentCategory).toBeDefined();

      // And each category should show tag name and count
      expect(phaseCategory!.tags.length).toBeGreaterThan(0);
      expect(componentCategory!.tags.length).toBeGreaterThan(0);

      // And tags should be sorted by count in descending order within each category
      const phaseTags = phaseCategory!.tags;
      for (let i = 0; i < phaseTags.length - 1; i++) {
        expect(phaseTags[i].count).toBeGreaterThanOrEqual(
          phaseTags[i + 1].count
        );
      }
    });
  });

  describe('Scenario: Show most used tags', () => {
    it('should sort tags by count in descending order', async () => {
      // Given I have feature files where @phase1 is used 4 times and @phase2 is used 2 times
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [
              { name: '@phase1', description: 'Phase 1' },
              { name: '@phase2', description: 'Phase 2' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      for (let i = 1; i <= 4; i++) {
        await writeFile(
          join(testDir, 'spec/features', `phase1-${i}.feature`),
          '@phase1\nFeature: Test\n  Scenario: Test\n    Given test\n'
        );
      }

      for (let i = 1; i <= 2; i++) {
        await writeFile(
          join(testDir, 'spec/features', `phase2-${i}.feature`),
          '@phase2\nFeature: Test\n  Scenario: Test\n    Given test\n'
        );
      }

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      const phaseCategory = result.categories.find(
        c => c.name === 'Phase Tags'
      );

      // Then @phase1 should appear before @phase2 in the output
      const phase1Index = phaseCategory!.tags.findIndex(
        t => t.tag === '@phase1'
      );
      const phase2Index = phaseCategory!.tags.findIndex(
        t => t.tag === '@phase2'
      );
      expect(phase1Index).toBeLessThan(phase2Index);

      // And the count for @phase1 should be 4
      expect(phaseCategory!.tags[phase1Index].count).toBe(4);

      // And the count for @phase2 should be 2
      expect(phaseCategory!.tags[phase2Index].count).toBe(2);
    });
  });

  describe('Scenario: Identify unused registered tags', () => {
    it('should show unused tags section with tags not in feature files', async () => {
      // Given I have tags.json with 10 registered tags
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [
              { name: '@phase1', description: 'Phase 1' },
              { name: '@phase2', description: 'Phase 2' },
              { name: '@phase3', description: 'Phase 3' },
            ],
          },
          {
            name: 'Component Tags',
            description: 'Architectural components',
            required: false,
            tags: [
              { name: '@cli', description: 'CLI' },
              { name: '@parser', description: 'Parser' },
              { name: '@formatter', description: 'Formatter' },
              { name: '@validator', description: 'Validator' },
              { name: '@generator', description: 'Generator' },
              { name: '@file-ops', description: 'File ops' },
              { name: '@integration', description: 'Integration' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      // And only 7 of those tags are used in feature files
      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@phase1 @cli @parser @formatter @validator @generator @file-ops\nFeature: Test\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the output should show a section for unused tags
      expect(result.unusedTags).toBeDefined();

      // And the unused tags section should list 3 tags
      expect(result.unusedTags.length).toBe(3);

      // And the unused tags should be the ones not present in any feature file
      expect(result.unusedTags).toContain('@phase2');
      expect(result.unusedTags).toContain('@phase3');
      expect(result.unusedTags).toContain('@integration');
    });
  });

  describe('Scenario: Show statistics when no feature files exist', () => {
    it('should show 0 for all counts', async () => {
      // Given I have no feature files in spec/features/
      // (testDir is already set up with empty spec/features/)

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show 0 feature files
      expect(result.totalFiles).toBe(0);

      // And the output should show 0 tags used
      expect(result.uniqueTags).toBe(0);
    });
  });

  describe('Scenario: Handle feature files with no tags', () => {
    it('should only count tags from tagged files', async () => {
      // Given I have a feature file with no tags
      await writeFile(
        join(testDir, 'spec/features/no-tags.feature'),
        'Feature: No Tags\n  Scenario: Test\n    Given test\n'
      );

      // And I have a feature file with 3 tags
      await writeFile(
        join(testDir, 'spec/features/with-tags.feature'),
        '@tag1 @tag2 @tag3\nFeature: With Tags\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the statistics should only count the tags from the tagged file
      expect(result.totalFiles).toBe(2);
      expect(result.uniqueTags).toBe(3);
      expect(result.totalOccurrences).toBe(3);
    });
  });

  describe('Scenario: Count tags from unregistered tags correctly', () => {
    it('should group unregistered tags separately with accurate counts', async () => {
      // Given I have feature files using both registered and unregistered tags
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [{ name: '@phase1', description: 'Phase 1' }],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@phase1 @custom-tag\nFeature: F1\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f2.feature'),
        '@custom-tag @another-tag\nFeature: F2\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the output should show all tags found in feature files
      expect(result.uniqueTags).toBe(3);

      // And unregistered tags should be grouped in an "Unregistered" section
      const unregisteredCategory = result.categories.find(
        c => c.name === 'Unregistered'
      );
      expect(unregisteredCategory).toBeDefined();

      // And the count for each unregistered tag should be accurate
      const customTag = unregisteredCategory!.tags.find(
        t => t.tag === '@custom-tag'
      );
      const anotherTag = unregisteredCategory!.tags.find(
        t => t.tag === '@another-tag'
      );
      expect(customTag!.count).toBe(2);
      expect(anotherTag!.count).toBe(1);
    });
  });

  describe('Scenario: Handle tags.json not found', () => {
    it('should show warning and group all tags as unregistered', async () => {
      // Given spec/tags.json does not exist
      // (no tags.json created in testDir)

      // And I have feature files with tags
      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@tag1 @tag2\nFeature: F1\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show warning that tags.json was not found
      expect(result.tagsFileFound).toBe(false);

      // And all tags should be shown in "Unregistered" category
      const unregisteredCategory = result.categories.find(
        c => c.name === 'Unregistered'
      );
      expect(unregisteredCategory).toBeDefined();
      expect(unregisteredCategory!.tags.length).toBe(2);

      // And the statistics should still be accurate
      expect(result.uniqueTags).toBe(2);
      expect(result.totalOccurrences).toBe(2);
    });
  });

  describe('Scenario: Display zero count for categories with no usage', () => {
    it('should show categories with unused tags', async () => {
      // Given I have tags.json with "Testing Tags" category
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Testing Tags',
            description: 'Testing-related tags',
            required: false,
            tags: [
              { name: '@unit-test', description: 'Unit test' },
              { name: '@integration-test', description: 'Integration test' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      // And no feature files use any testing tags
      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@phase1\nFeature: F1\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the output should show "Testing Tags" category
      // (implicitly - unused tags will be in the unusedTags array)
      expect(result.unusedTags).toBeDefined();

      // And all testing tags should show count of 0 in the unused section
      expect(result.unusedTags).toContain('@unit-test');
      expect(result.unusedTags).toContain('@integration-test');
    });
  });

  describe('Scenario: Handle invalid feature files gracefully', () => {
    it('should count valid files and warn about invalid ones', async () => {
      // Given I have 3 valid feature files with tags
      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@tag1\nFeature: F1\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f2.feature'),
        '@tag2\nFeature: F2\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f3.feature'),
        '@tag3\nFeature: F3\n  Scenario: Test\n    Given test\n'
      );

      // And I have 1 feature file with invalid Gherkin syntax
      await writeFile(
        join(testDir, 'spec/features/invalid.feature'),
        'This is not valid Gherkin syntax at all'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the statistics should count tags from the 3 valid files
      expect(result.uniqueTags).toBe(3);
      expect(result.totalOccurrences).toBe(3);

      // And the output should show a warning about the invalid file
      expect(result.invalidFiles).toBeDefined();
      expect(result.invalidFiles.length).toBe(1);
      expect(result.invalidFiles[0]).toContain('invalid.feature');
    });
  });

  describe('Scenario: JSON-backed workflow - read categories from tags.json', () => {
    it('should load tag categories from tags.json and group statistics', async () => {
      // Given I have tags.json with multiple tag categories
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [
              { name: '@phase1', description: 'Phase 1 features' },
              { name: '@phase2', description: 'Phase 2 features' },
              { name: '@phase3', description: 'Phase 3 features' },
            ],
          },
          {
            name: 'Component Tags',
            description: 'Architectural components',
            required: false,
            tags: [
              { name: '@cli', description: 'CLI component' },
              { name: '@parser', description: 'Parser component' },
              { name: '@formatter', description: 'Formatter component' },
            ],
          },
          {
            name: 'Feature Group Tags',
            description: 'Functional areas',
            required: false,
            tags: [
              {
                name: '@feature-management',
                description: 'Feature management',
              },
              { name: '@tag-management', description: 'Tag management' },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec', 'tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      // And I have feature files using tags from different categories
      await writeFile(
        join(testDir, 'spec/features/f1.feature'),
        '@phase1 @cli @feature-management\nFeature: F1\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f2.feature'),
        '@phase2 @parser @tag-management\nFeature: F2\n  Scenario: Test\n    Given test\n'
      );
      await writeFile(
        join(testDir, 'spec/features/f3.feature'),
        '@phase1 @formatter @feature-management\nFeature: F3\n  Scenario: Test\n    Given test\n'
      );

      // When I run `fspec tag-stats`
      const result = await tagStats({ cwd: testDir });

      // Then the command should load tag categories from spec/tags.json
      expect(result.success).toBe(true);
      expect(result.tagsFileFound).toBe(true);

      // And statistics should be grouped by categories from tags.json
      expect(result.categories).toBeDefined();
      const phaseCategory = result.categories.find(
        c => c.name === 'Phase Tags'
      );
      const componentCategory = result.categories.find(
        c => c.name === 'Component Tags'
      );
      const featureGroupCategory = result.categories.find(
        c => c.name === 'Feature Group Tags'
      );

      expect(phaseCategory).toBeDefined();
      expect(componentCategory).toBeDefined();
      expect(featureGroupCategory).toBeDefined();

      // And each category should show accurate tag counts
      expect(phaseCategory!.tags).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ tag: '@phase1', count: 2 }),
          expect.objectContaining({ tag: '@phase2', count: 1 }),
        ])
      );

      // And unused registered tags should be identified correctly
      expect(result.unusedTags).toContain('@phase3');

      // And the command should exit with code 0
      expect(result.success).toBe(true);
    });
  });
});
