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
    it('should remove tag from registry', async () => {
      // Given I have a tag @deprecated registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features

### @deprecated
Deprecated features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // And the tag @deprecated is not used in any feature files
      // (no feature files exist)

      // When I run `fspec delete-tag @deprecated`
      const result = await deleteTag({
        tag: '@deprecated',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the tag @deprecated should be removed from TAGS.md
      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@deprecated');

      // And the TAGS.md structure should be preserved
      expect(updatedContent).toContain('# Tag Registry');
      expect(updatedContent).toContain('## Tag Categories');
      expect(updatedContent).toContain('@phase1');

      // And the output should show "Successfully deleted tag @deprecated"
      expect(result.message).toContain('Successfully deleted');
      expect(result.message).toContain('@deprecated');
    });
  });

  describe('Scenario: Attempt to delete tag in use', () => {
    it('should return error and list files using tag', async () => {
      // Given I have a tag @phase1 registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // And the tag @phase1 is used in 5 feature files
      await mkdir(join(testDir, 'spec/features'), { recursive: true });
      for (let i = 1; i <= 5; i++) {
        await writeFile(
          join(testDir, 'spec/features', `test${i}.feature`),
          `@phase1\nFeature: Test ${i}\n  Scenario: Test\n    Given test\n`
        );
      }

      // When I run `fspec delete-tag @phase1`
      const result = await deleteTag({
        tag: '@phase1',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @phase1 is used in 5 feature file(s)"
      expect(result.error).toMatch(/used in.*5.*file/i);
      expect(result.error).toContain('@phase1');

      // And the output should list the feature files using the tag
      expect(result.error).toContain('test1.feature');
      expect(result.error).toContain('test5.feature');

      // And the tag should remain in TAGS.md
      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );
      expect(updatedContent).toContain('@phase1');

      // And the output should suggest using --force to delete anyway
      expect(result.error).toMatch(/--force/i);
    });
  });

  describe('Scenario: Force delete tag in use', () => {
    it('should delete tag with warning', async () => {
      // Given I have a tag @deprecated registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @deprecated
Deprecated features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // And the tag @deprecated is used in 2 feature files
      await mkdir(join(testDir, 'spec/features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec/features/test1.feature'),
        `@deprecated\nFeature: Test 1\n  Scenario: Test\n    Given test\n`
      );
      await writeFile(
        join(testDir, 'spec/features/test2.feature'),
        `@deprecated\nFeature: Test 2\n  Scenario: Test\n    Given test\n`
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
      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@deprecated');

      // And the output should show warning about files still using the tag
      expect(result.warning).toBeDefined();
      expect(result.warning).toMatch(/still used/i);

      // And the output should list the 2 feature files
      expect(result.warning).toContain('test1.feature');
      expect(result.warning).toContain('test2.feature');
    });
  });

  describe('Scenario: Attempt to delete non-existent tag', () => {
    it('should return error for non-existent tag', async () => {
      // Given I do not have a tag @nonexistent in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);
      const originalContent = tagsContent;

      // When I run `fspec delete-tag @nonexistent`
      const result = await deleteTag({
        tag: '@nonexistent',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @nonexistent not found in registry"
      expect(result.error).toMatch(/not found/i);
      expect(result.error).toContain('@nonexistent');

      // And TAGS.md should remain unchanged
      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Delete tag preserves other tags in same category', () => {
    it('should preserve other tags', async () => {
      // Given I have tags @phase1, @phase2, @phase3 in category "Tag Categories"
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

      // When I run `fspec delete-tag @phase2`
      const result = await deleteTag({
        tag: '@phase2',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );

      // And @phase1 and @phase3 should remain in TAGS.md
      expect(updatedContent).toContain('@phase1');
      expect(updatedContent).toContain('@phase3');
      expect(updatedContent).not.toContain('@phase2');

      // And the category "Tag Categories" should remain intact
      expect(updatedContent).toContain('## Tag Categories');
    });
  });

  describe('Scenario: Delete tag from specific category', () => {
    it('should remove tag from correct category', async () => {
      // Given I have a tag @custom registered in category "Technical Tags"
      const tagsContent = `# Tag Registry

## Tag Categories

### @phase1
Phase 1 features

## Technical Tags

### @custom
Custom technical tag
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // And the tag is not used in any feature files

      // When I run `fspec delete-tag @custom`
      const result = await deleteTag({
        tag: '@custom',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );

      // And the tag should be removed from "Technical Tags" category
      expect(updatedContent).not.toContain('@custom');

      // And other tags in "Technical Tags" should remain
      expect(updatedContent).toContain('## Technical Tags');
      expect(updatedContent).toContain('@phase1');
    });
  });

  describe('Scenario: Delete tag updates tag statistics', () => {
    it('should reduce tag count', async () => {
      // Given I have 10 tags registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @tag1
Tag 1

### @tag2
Tag 2

### @tag3
Tag 3

### @tag4
Tag 4

### @tag5
Tag 5

### @tag6
Tag 6

### @tag7
Tag 7

### @tag8
Tag 8

### @tag9
Tag 9

### @obsolete
Obsolete tag
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // And one tag @obsolete is unused
      // When I run `fspec delete-tag @obsolete`
      const result = await deleteTag({
        tag: '@obsolete',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );

      // And the tag count should be 9
      const tagMatches = updatedContent.match(/^### @/gm);
      expect(tagMatches).toHaveLength(9);

      // And the tag should not appear in tag statistics
      expect(updatedContent).not.toContain('@obsolete');
    });
  });

  describe('Scenario: Handle TAGS.md with invalid format', () => {
    it('should return error for invalid structure', async () => {
      // Given I have a TAGS.md file with invalid structure
      const tagsContent = `This is not valid TAGS.md format
No proper headers here
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);
      const originalContent = tagsContent;

      // When I run `fspec delete-tag @sometag`
      const result = await deleteTag({
        tag: '@sometag',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Could not parse TAGS.md structure"
      expect(result.error).toMatch(/not found|parse/i);

      // And the file should remain unchanged
      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Delete last tag in category leaves category intact', () => {
    it('should keep category after deleting last tag', async () => {
      // Given I have only one tag @lonely in category "Custom Category"
      const tagsContent = `# Tag Registry

## Custom Category

### @lonely
Lonely tag
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);

      // And the tag is not used in any feature files
      // When I run `fspec delete-tag @lonely`
      const result = await deleteTag({
        tag: '@lonely',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );

      // And the tag should be removed
      expect(updatedContent).not.toContain('@lonely');

      // And the category "Custom Category" should remain (empty)
      expect(updatedContent).toContain('## Custom Category');
    });
  });

  describe('Scenario: Dry run shows what would be deleted', () => {
    it('should preview deletion without changes', async () => {
      // Given I have a tag @test registered in TAGS.md
      const tagsContent = `# Tag Registry

## Tag Categories

### @test
Test tag
`;
      await writeFile(join(testDir, 'spec/TAGS.md'), tagsContent);
      const originalContent = tagsContent;

      // And the tag is not used in any feature files
      // When I run `fspec delete-tag @test --dry-run`
      const result = await deleteTag({
        tag: '@test',
        dryRun: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Would delete tag @test"
      expect(result.message).toMatch(/would delete/i);
      expect(result.message).toContain('@test');

      // And the tag should remain in TAGS.md
      const updatedContent = await readFile(
        join(testDir, 'spec/TAGS.md'),
        'utf-8'
      );
      expect(updatedContent).toBe(originalContent);

      // And the output should show the category it would be removed from
      expect(result.message).toContain('Tag Categories');
    });
  });
});
