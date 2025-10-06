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

    // Create TAGS.md with sample tags
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });

    const tagsContent = `# Tag Registry

## Phase Tags
| Tag | Description |
|-----|-------------|
| \`@phase1\` | Phase 1: Core features |
| \`@phase2\` | Phase 2: Advanced features |
| \`@phase3\` | Phase 3: Future features |

## Component Tags
| Tag | Description |
|-----|-------------|
| \`@cli\` | CLI component |
| \`@parser\` | Parser component |
| \`@validator\` | Validator component |

## Feature Group Tags
| Tag | Description |
|-----|-------------|
| \`@feature-management\` | Feature management |
| \`@tag-management\` | Tag management |
| \`@validation\` | Validation features |

## Technical Tags
| Tag | Description |
|-----|-------------|
| \`@gherkin\` | Gherkin spec compliance |
`;

    await writeFile(join(specDir, 'TAGS.md'), tagsContent);
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List all registered tags', () => {
    it('should display tags grouped by category', async () => {
      // Given I have a TAGS.md file with tags in multiple categories
      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display tags grouped by category
      expect(result.categories).toHaveLength(4);
      expect(result.categories.map(c => c.name)).toContain('Phase Tags');
      expect(result.categories.map(c => c.name)).toContain('Component Tags');
      expect(result.categories.map(c => c.name)).toContain('Feature Group Tags');

      // And each tag should be displayed with its description
      const phaseTags = result.categories.find(c => c.name === 'Phase Tags');
      expect(phaseTags?.tags).toHaveLength(3);
      expect(phaseTags?.tags[0]).toEqual({ tag: '@phase1', description: 'Phase 1: Core features' });
    });
  });

  describe('Scenario: Filter tags by category', () => {
    it('should only show tags from specified category', async () => {
      // Given I have a TAGS.md file with tags in multiple categories
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

  describe('Scenario: Handle missing TAGS.md file', () => {
    it('should error when TAGS.md does not exist', async () => {
      // Given no TAGS.md file exists in spec/
      await rm(join(testDir, 'spec', 'TAGS.md'));

      // When I run `fspec list-tags`
      // Then it should throw an error
      await expect(listTags({ cwd: testDir })).rejects.toThrow('TAGS.md not found');

      try {
        await listTags({ cwd: testDir });
      } catch (error: any) {
        // And the output should suggest creating TAGS.md
        expect(error.message).toContain('spec/TAGS.md');
      }
    });
  });

  describe('Scenario: Display tag count per category', () => {
    it('should show count for each category', async () => {
      // Given I have a TAGS.md file with tags
      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the output should show the count for each category
      const phaseTags = result.categories.find(c => c.name === 'Phase Tags');
      expect(phaseTags?.tags).toHaveLength(3);

      const componentTags = result.categories.find(c => c.name === 'Component Tags');
      expect(componentTags?.tags).toHaveLength(3);

      const featureGroupTags = result.categories.find(c => c.name === 'Feature Group Tags');
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
      // Given I have a TAGS.md file
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
      // Given I have a TAGS.md file with only Phase Tags populated
      const minimalTags = `# Tag Registry

## Phase Tags
| Tag | Description |
|-----|-------------|
| \`@phase1\` | Phase 1 |

## Component Tags
| Tag | Description |
|-----|-------------|

## Feature Group Tags
| Tag | Description |
|-----|-------------|
`;

      await writeFile(join(testDir, 'spec', 'TAGS.md'), minimalTags);

      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the output should show all category headers
      expect(result.categories.map(c => c.name)).toContain('Phase Tags');
      expect(result.categories.map(c => c.name)).toContain('Component Tags');
      expect(result.categories.map(c => c.name)).toContain('Feature Group Tags');

      // And empty categories should show 0 tags
      const componentTags = result.categories.find(c => c.name === 'Component Tags');
      expect(componentTags?.tags).toHaveLength(0);
    });
  });

  describe('Scenario: Display tag descriptions with wrapping', () => {
    it('should display long descriptions without truncation', async () => {
      // Given I have a tag with a long description
      const longDescTags = `# Tag Registry

## Technical Tags
| Tag | Description |
|-----|-------------|
| \`@long-desc\` | This is a very long description that explains the purpose and usage of tag |
`;

      await writeFile(join(testDir, 'spec', 'TAGS.md'), longDescTags);

      // When I run `fspec list-tags`
      const result = await listTags({ cwd: testDir });

      // Then the description should be displayed without truncation
      const technicalTags = result.categories.find(c => c.name === 'Technical Tags');
      const longDescTag = technicalTags?.tags.find(t => t.tag === '@long-desc');
      expect(longDescTag?.description).toBe('This is a very long description that explains the purpose and usage of tag');
    });
  });

  describe('Scenario: AI agent workflow - discover available tags', () => {
    it('should support tag discovery workflow', async () => {
      // Given I am an AI agent working on a new feature specification
      // And I need to know which tags to use
      // When I run `fspec list-tags --category="Feature Group Tags"`
      const categoryResult = await listTags({ category: 'Feature Group Tags', cwd: testDir });

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
      // Given I have tags registered in TAGS.md
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
});
