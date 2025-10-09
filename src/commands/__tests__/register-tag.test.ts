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

    // Create minimal TAGS.md
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });

    const tagsContent = `# Tag Registry

## Phase Tags
| Tag | Description |
|-----|-------------|
| \`@phase1\` | Phase 1 |

## Component Tags
| Tag | Description |
|-----|-------------|
| \`@cli\` | CLI |

## Feature Group Tags
| Tag | Description |
|-----|-------------|
| \`@validation\` | Validation |

## Technical Tags
| Tag | Description |
|-----|-------------|
| \`@gherkin\` | Gherkin spec |
`;

    await writeFile(join(specDir, 'TAGS.md'), tagsContent);
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
      // Given I have a TAGS.md file with tag @cli registered
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

      // And TAGS.md should remain unchanged
      const originalContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      const cliCount = (originalContent.match(/@cli/g) || []).length;
      expect(cliCount).toBe(1); // Only the original occurrence
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

  describe("Scenario: Create TAGS.md if it doesn't exist", () => {
    it('should create TAGS.md with standard structure and add tag', async () => {
      // Given no TAGS.md file exists in spec/
      await rm(join(testDir, 'spec', 'TAGS.md'));

      // When I run `fspec register-tag @custom "Technical Tags" "Custom feature"`
      const result = await registerTag(
        '@custom',
        'Technical Tags',
        'Custom feature',
        { cwd: testDir }
      );

      // Then a new TAGS.md file should be created with standard structure
      const tagsContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(tagsContent).toContain('# Tag Registry');
      expect(tagsContent).toContain('## Phase Tags');
      expect(tagsContent).toContain('## Component Tags');
      expect(tagsContent).toContain('## Technical Tags');

      // And the tag @custom should be added to Technical Tags section
      expect(tagsContent).toContain('`@custom`');
      expect(tagsContent).toContain('Custom feature');

      // And the file should include all standard category sections
      expect(tagsContent).toContain('## Feature Group Tags');
      expect(result.created).toBe(true);
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

  describe('Scenario: Preserve TAGS.md formatting when adding tag', () => {
    it('should maintain formatting and order', async () => {
      // Given I have a TAGS.md file with custom formatting
      const originalContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );

      // When I run `fspec register-tag @new-tag "Technical Tags" "New feature"`
      await registerTag('@new-tag', 'Technical Tags', 'New feature', {
        cwd: testDir,
      });

      // Then the tag should be added without affecting other sections
      const newContent = await readFile(
        join(testDir, 'spec', 'TAGS.md'),
        'utf-8'
      );
      expect(newContent).toContain('## Phase Tags');
      expect(newContent).toContain('@phase1');
      expect(newContent).toContain('`@new-tag`');

      // And table column alignment should be preserved
      expect(newContent).toMatch(/\|\s*Tag\s*\|\s*Description\s*\|/);

      // And existing tags should remain in their original order
      expect(newContent.indexOf('@gherkin')).toBeGreaterThan(0);
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
});
