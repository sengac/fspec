import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { registerTag } from '../register-tag';

describe('Feature: Register New Tag in Tag Registry', () => {
  let testDir: string;
  let originalCwd: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    originalCwd = process.cwd();
    process.chdir(testDir);

    // Create minimal tags.json for testing
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });

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
          tags: [{ name: '@cli', description: 'CLI' }],
        },
        {
          name: 'Feature Group Tags',
          description: 'Feature group tags',
          required: true,
          tags: [{ name: '@validation', description: 'Validation' }],
        },
        {
          name: 'Technical Tags',
          description: 'Technical tags',
          required: false,
          tags: [{ name: '@gherkin', description: 'Gherkin spec' }],
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

    await writeFile(
      join(specDir, 'tags.json'),
      JSON.stringify(minimalTags, null, 2)
    );
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Register a new tag in an existing category', () => {
    it('should add tag to Technical Tags table in alphabetical order', async () => {
      // Given I have a TAGS.md file with standard categories
      // When I run `fspec register-tag @api "Technical Tags" "API integration features"`
      const result = await registerTag(
        '@api',
        'Technical Tags',
        'API integration features',
        { cwd: testDir }
      );

      // Then the tag should be added to the Technical Tags table in TAGS.md
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('`@api`');
      expect(tagsContent).toContain('API integration features');

      // And the tag should be inserted in alphabetical order (after @api, before @gherkin)
      const apiIndex = tagsContent.indexOf('@api');
      const gherkinIndex = tagsContent.indexOf('@gherkin');
      expect(apiIndex).toBeLessThan(gherkinIndex);

      // And the command should confirm the registration with success message
      expect(result.success).toBe(true);
      expect(result.message).toContain('@api');
    });
  });

  describe('Scenario: Prevent duplicate tag registration', () => {
    it('should error when tag already exists', async () => {
      // Given I have tags.json with tag @cli registered
      // When I run `fspec register-tag @cli "Component Tags" "CLI component"`
      await expect(
        registerTag('@cli', 'Component Tags', 'CLI component', { cwd: testDir })
      ).rejects.toThrow();

      // Then the error message should indicate @cli is already registered
      try {
        await registerTag('@cli', 'Component Tags', 'CLI component', {
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toContain('@cli');
        expect(error.message).toContain('already registered');
      }

      // And tags.json should remain unchanged
      const tagsJson = JSON.parse(
        await readFile(join(testDir, 'spec', 'tags.json'), 'utf-8')
      );
      const componentCategory = tagsJson.categories.find(
        (c: any) => c.name === 'Component Tags'
      );
      const cliTags = componentCategory.tags.filter(
        (t: any) => t.name === '@cli'
      );
      expect(cliTags.length).toBe(1); // Only the original occurrence
    });
  });

  describe('Scenario: Validate tag naming convention', () => {
    it('should reject invalid tag format', async () => {
      // Given I have a TAGS.md file
      // When I run `fspec register-tag InvalidTag "Technical Tags" "Invalid format"`
      await expect(
        registerTag('InvalidTag', 'Technical Tags', 'Invalid format', {
          cwd: testDir,
        })
      ).rejects.toThrow();

      // Then the error message should indicate invalid tag format
      try {
        await registerTag('InvalidTag', 'Technical Tags', 'Invalid format', {
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toContain('Invalid tag format');
        expect(error.message).toContain('@lowercase-with-hyphens');
      }
    });
  });

  describe("Scenario: Create tags.json if it doesn't exist", () => {
    it('should create tags.json with standard structure and add tag', async () => {
      // Given no tags.json file exists in spec/
      await rm(join(testDir, 'spec', 'tags.json'));

      // When I run `fspec register-tag @custom "Technical Tags" "Custom feature"`
      const result = await registerTag(
        '@custom',
        'Technical Tags',
        'Custom feature',
        { cwd: testDir }
      );

      // Then a new tags.json file should be created
      const tagsJson = JSON.parse(
        await readFile(join(testDir, 'spec', 'tags.json'), 'utf-8')
      );
      expect(tagsJson.categories).toBeDefined();

      // And TAGS.md should be generated with standard structure
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('<!-- THIS FILE IS AUTO-GENERATED');
      expect(tagsContent).toContain('## Phase Tags');
      expect(tagsContent).toContain('## Component Tags');
      expect(tagsContent).toContain('## Technical Tags');

      // And the tag @custom should be added to Technical Tags section
      expect(tagsContent).toContain('`@custom`');
      expect(tagsContent).toContain('Custom feature');

      // And the file should include all standard category sections
      expect(tagsContent).toContain('## Feature Group Tags');

      // And the result should indicate success
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Register tag with uppercase in input (auto-convert)', () => {
    it('should convert to lowercase and register', async () => {
      // Given I have a TAGS.md file
      // When I run `fspec register-tag @API-Integration "Technical Tags" "API features"`
      const result = await registerTag(
        '@API-Integration',
        'Technical Tags',
        'API features',
        { cwd: testDir }
      );

      // Then the tag should be registered as @api-integration (lowercase)
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('`@api-integration`');
      expect(tagsContent).not.toContain('@API-Integration');

      // And the command should confirm the lowercase conversion
      expect(result.message).toContain('@api-integration');
      expect(result.converted).toBe(true);
    });
  });

  describe('Scenario: Handle invalid category name', () => {
    it('should error with list of valid categories', async () => {
      // Given I have a TAGS.md file
      // When I run `fspec register-tag @custom "NonExistent Category" "Description"`
      await expect(
        registerTag('@custom', 'NonExistent Category', 'Description', {
          cwd: testDir,
        })
      ).rejects.toThrow();

      // Then the error message should list valid categories
      try {
        await registerTag('@custom', 'NonExistent Category', 'Description', {
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toContain('Invalid category');
        expect(error.message).toMatch(
          /Phase Tags|Component Tags|Technical Tags/
        );
      }
    });
  });

  describe('Scenario: Preserve JSON structure and regenerate TAGS.md', () => {
    it('should maintain JSON structure and regenerate MD', async () => {
      // Given I have tags.json with existing tags
      const originalJson = JSON.parse(
        await readFile(join(testDir, 'spec', 'tags.json'), 'utf-8')
      );

      // When I run `fspec register-tag @new-tag "Technical Tags" "New feature"`
      await registerTag('@new-tag', 'Technical Tags', 'New feature', {
        cwd: testDir,
      });

      // Then tags.json should have the new tag
      const newJson = JSON.parse(
        await readFile(join(testDir, 'spec', 'tags.json'), 'utf-8')
      );
      const technicalCat = newJson.categories.find(
        (c: any) => c.name === 'Technical Tags'
      );
      expect(technicalCat.tags.find((t: any) => t.name === '@new-tag')).toBeDefined();

      // And TAGS.md should be regenerated with all tags
      const newContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(newContent).toContain('## Phase Tags');
      expect(newContent).toContain('@phase1');
      expect(newContent).toContain('`@new-tag`');

      // And existing tags should remain present
      expect(newContent).toContain('@gherkin');
    });
  });

  describe('Scenario: Register tag with long description', () => {
    it('should handle long descriptions properly', async () => {
      // Given I have a TAGS.md file
      const longDesc =
        'This is a very long description that explains the purpose of the tag in detail';

      // When I run `fspec register-tag @long-desc "Technical Tags" "<long description>"`
      const result = await registerTag(
        '@long-desc',
        'Technical Tags',
        longDesc,
        { cwd: testDir }
      );

      // Then the tag should be registered with the full description
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('`@long-desc`');
      expect(tagsContent).toContain(longDesc);

      // And the table should remain properly formatted
      expect(tagsContent).toMatch(/\|\s*`@long-desc`\s*\|.*\|/);
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: AI agent workflow - discover, register, validate', () => {
    it('should support complete workflow', async () => {
      // Given I am an AI agent working on a new feature type
      // And I identify a need for tag @websocket
      // When I run `fspec register-tag @websocket "Technical Tags" "WebSocket communication features"`
      const result = await registerTag(
        '@websocket',
        'Technical Tags',
        'WebSocket communication features',
        { cwd: testDir }
      );

      // Then the tag should be successfully registered in TAGS.md
      expect(result.success).toBe(true);

      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('`@websocket`');

      // And when I run validate-tags on features using @websocket
      // Then validation should pass for the newly registered tag
      // (This would be tested via integration with validate-tags, so just verify tag is in TAGS.md)
      expect(tagsContent).toContain('WebSocket communication features');
    });
  });

  describe('Scenario: Register tag in all major categories', () => {
    it('should register tags in different categories', async () => {
      // Given I have a TAGS.md file
      // When I run `fspec register-tag @phase4 "Phase Tags" "Phase 4: Future features"`
      await registerTag('@phase4', 'Phase Tags', 'Phase 4: Future features', {
        cwd: testDir,
      });

      // Then the tag should be added to Phase Tags section
      let tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('`@phase4`');

      // When I run `fspec register-tag @new-component "Component Tags" "New component"`
      await registerTag('@new-component', 'Component Tags', 'New component', {
        cwd: testDir,
      });

      // Then the tag should be added to Component Tags section
      tagsContent = await readFile(join(testDir, 'spec', 'TAGS.md'), 'utf-8');
      expect(tagsContent).toContain('`@new-component`');

      // When I run `fspec register-tag @new-group "Feature Group Tags" "New feature group"`
      await registerTag(
        '@new-group',
        'Feature Group Tags',
        'New feature group',
        { cwd: testDir }
      );

      // Then the tag should be added to Feature Group Tags section
      tagsContent = await readFile(join(testDir, 'spec', 'TAGS.md'), 'utf-8');
      expect(tagsContent).toContain('`@new-group`');

      // And all registrations should maintain proper formatting
      expect(tagsContent).toMatch(/## Phase Tags/);
      expect(tagsContent).toMatch(/## Component Tags/);
      expect(tagsContent).toMatch(/## Feature Group Tags/);
    });
  });

  describe('Scenario: JSON-backed workflow - modify JSON and regenerate MD', () => {
    it('should update tags.json and regenerate TAGS.md', async () => {
      // Given I have a valid tags.json file
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
                name: '@gherkin',
                description: 'Gherkin spec',
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

      // When I run `fspec register-tag @new-tag "Technical Tags" "New technical tag"`
      const result = await registerTag(
        '@new-tag',
        'Technical Tags',
        'New technical tag',
        { cwd: testDir }
      );

      // Then the tags.json file should be updated with the new tag
      const updatedTagsJson = JSON.parse(
        await readFile(tagsJsonPath, 'utf-8')
      );

      // And the tags.json should validate against tags.schema.json
      expect(updatedTagsJson.categories).toBeDefined();

      // And the new tag should be in the "Technical Tags" category array
      const technicalCategory = updatedTagsJson.categories.find(
        (c: any) => c.name === 'Technical Tags'
      );
      expect(technicalCategory).toBeDefined();
      const newTag = technicalCategory.tags.find(
        (t: any) => t.name === '@new-tag'
      );
      expect(newTag).toBeDefined();
      expect(newTag.description).toBe('New technical tag');

      // And the tag should be in alphabetical order within the category
      const tagNames = technicalCategory.tags.map((t: any) => t.name);
      expect(tagNames).toContain('@new-tag');
      expect(tagNames).toContain('@gherkin');
      // @gherkin comes before @new-tag alphabetically
      expect(tagNames.indexOf('@gherkin')).toBeLessThan(
        tagNames.indexOf('@new-tag')
      );

      // And TAGS.md should be regenerated from tags.json
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );

      // And TAGS.md should contain the new tag in the Technical Tags table
      expect(tagsContent).toContain('`@new-tag`');
      expect(tagsContent).toContain('New technical tag');

      // And TAGS.md should have the auto-generation warning header
      expect(tagsContent).toContain(
        '<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->'
      );
      expect(tagsContent).toContain('<!-- DO NOT EDIT THIS FILE DIRECTLY -->');

      expect(result.success).toBe(true);
    });
  });
});
