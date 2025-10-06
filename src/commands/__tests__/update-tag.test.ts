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
    it('should update description and preserve category', async () => {
      // Given I have a tag @phase1 registered in TAGS.md with description "Phase 1 features"
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features

### @phase2
Phase 2 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @phase1 --description="Phase 1 - Core validation and feature management"`
      const result = await updateTag({
        tag: '@phase1',
        description: 'Phase 1 - Core validation and feature management',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag @phase1 description should be updated in TAGS.md
      const updatedContent = await readFile(join(testDir, 'spec/TAGS.md'), 'utf-8');
      expect(updatedContent).toContain('Phase 1 - Core validation and feature management');

      // And the tag @phase1 category should remain unchanged
      expect(updatedContent).toContain('## Tag Categories');
      expect(updatedContent).toContain('### @phase1');

      // And the output should show "Successfully updated @phase1"
      expect(result.message).toContain('Successfully updated @phase1');
    });
  });

  describe('Scenario: Update tag category only', () => {
    it('should move tag to new category', async () => {
      // Given I have a tag @deprecated registered in category "Status Tags"
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features

## Status Tags

### @deprecated
Deprecated features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @deprecated --category="Tag Categories"`
      const result = await updateTag({
        tag: '@deprecated',
        category: 'Tag Categories',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag @deprecated should be moved to category "Tag Categories"
      const updatedContent = await readFile(join(testDir, 'spec/TAGS.md'), 'utf-8');
      const tagCategoriesSection = updatedContent.split('## Status Tags')[0];
      expect(tagCategoriesSection).toContain('@deprecated');

      // And the tag @deprecated description should remain unchanged
      expect(updatedContent).toContain('Deprecated features');
    });
  });

  describe('Scenario: Update both category and description', () => {
    it('should update both fields', async () => {
      // Given I have a tag @phase1 registered with category "Tag Categories" and description "Phase 1"
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1

## Other Category

### @other
Other tag
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @phase1 --category="Tag Categories" --description="Phase 1 - Core validation and feature management"`
      const result = await updateTag({
        tag: '@phase1',
        category: 'Tag Categories',
        description: 'Phase 1 - Core validation and feature management',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag @phase1 category should be "Tag Categories"
      const updatedContent = await readFile(join(testDir, 'spec/TAGS.md'), 'utf-8');
      const tagCategoriesSection = updatedContent.split('## Other Category')[0];
      expect(tagCategoriesSection).toContain('@phase1');

      // And the tag @phase1 description should be "Phase 1 - Core validation and feature management"
      expect(updatedContent).toContain(
        'Phase 1 - Core validation and feature management'
      );
    });
  });

  describe('Scenario: Attempt to update non-existent tag', () => {
    it('should return error for non-existent tag', async () => {
      // Given I do not have a tag @nonexistent in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);
      const originalContent = tagsContent;

      // When I run `fspec update-tag @nonexistent --description="New description"`
      const result = await updateTag({
        tag: '@nonexistent',
        description: 'New description',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @nonexistent not found in registry"
      expect(result.error).toMatch(/not found/i);
      expect(result.error).toContain('@nonexistent');

      // And TAGS.md should remain unchanged
      const updatedContent = await readFile(join(testDir, 'spec/TAGS.md'), 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Update tag with invalid category', () => {
    it('should return error with category list', async () => {
      // Given I have a tag @phase1 registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @phase1 --category="Invalid Category"`
      const result = await updateTag({
        tag: '@phase1',
        category: 'Invalid Category',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Invalid category: Invalid Category"
      expect(result.error).toContain('Invalid category');
      expect(result.error).toContain('Invalid Category');

      // And the output should list available categories
      expect(result.error).toContain('Tag Categories');
    });
  });

  describe('Scenario: Update tag without any changes', () => {
    it('should return error when no updates specified', async () => {
      // Given I have a tag @phase1 registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @phase1` without --category or --description
      const result = await updateTag({
        tag: '@phase1',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "No updates specified. Use --category and/or --description"
      expect(result.error).toMatch(/no updates specified/i);
      expect(result.error).toMatch(/--category|--description/i);
    });
  });

  describe('Scenario: Update tag preserves other tags', () => {
    it('should only modify the specified tag', async () => {
      // Given I have multiple tags in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features

### @phase2
Phase 2 features

### @phase3
Phase 3 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @phase1 --description="Updated description"`
      await updateTag({
        tag: '@phase1',
        description: 'Updated description',
        cwd: testDir,
      });

      // Then only @phase1 should be modified
      const updatedContent = await readFile(join(testDir, 'spec/TAGS.md'), 'utf-8');
      expect(updatedContent).toContain('Updated description');

      // And all other tags should remain unchanged
      expect(updatedContent).toContain('Phase 2 features');
      expect(updatedContent).toContain('Phase 3 features');

      // And the TAGS.md structure should be preserved
      expect(updatedContent).toContain('## Tag Categories');
      expect(updatedContent).toContain('### @phase2');
      expect(updatedContent).toContain('### @phase3');
    });
  });

  describe('Scenario: Update tag handles special characters in description', () => {
    it('should preserve special characters', async () => {
      // Given I have a tag @auth registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @auth
Authentication features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // When I run `fspec update-tag @auth --description="Authentication & authorization with OAuth2.0"`
      const result = await updateTag({
        tag: '@auth',
        description: 'Authentication & authorization with OAuth2.0',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the description should contain "&" and "2.0"
      const updatedContent = await readFile(join(testDir, 'spec/TAGS.md'), 'utf-8');
      expect(updatedContent).toContain('&');
      expect(updatedContent).toContain('2.0');

      // And the markdown should be properly escaped
      expect(updatedContent).toContain('Authentication & authorization with OAuth2.0');
    });
  });
});
