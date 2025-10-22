import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { retag } from '../retag';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

describe('Feature: Bulk Rename Tags Across Files', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-retag');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Rename tag across multiple feature files', () => {
    it('should rename tag in all matching files', async () => {
      // Given I have 5 feature files
      // And 3 files use the tag @critical
      const withPhase1_1 = `@critical @critical
Feature: Feature 1
  Scenario: Test
    Given step`;

      const withPhase1_2 = `@critical
Feature: Feature 2
  Scenario: Test
    Given step`;

      const withPhase1_3 = `@critical @api
Feature: Feature 3
  Scenario: Test
    Given step`;

      const withoutPhase1_1 = `@high
Feature: Feature 4
  Scenario: Test
    Given step`;

      const withoutPhase1_2 = `@medium
Feature: Feature 5
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/file1.feature'),
        withPhase1_1
      );
      await writeFile(
        join(testDir, 'spec/features/file2.feature'),
        withPhase1_2
      );
      await writeFile(
        join(testDir, 'spec/features/file3.feature'),
        withPhase1_3
      );
      await writeFile(
        join(testDir, 'spec/features/file4.feature'),
        withoutPhase1_1
      );
      await writeFile(
        join(testDir, 'spec/features/file5.feature'),
        withoutPhase1_2
      );

      // When I run `fspec retag --from=@critical --to=@release-one`
      const result = await retag({
        from: '@critical',
        to: '@release-one',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And all @critical tags should be changed to @release-one
      const content1 = await readFile(
        join(testDir, 'spec/features/file1.feature'),
        'utf-8'
      );
      const content2 = await readFile(
        join(testDir, 'spec/features/file2.feature'),
        'utf-8'
      );
      const content3 = await readFile(
        join(testDir, 'spec/features/file3.feature'),
        'utf-8'
      );

      expect(content1).toContain('@release-one');
      expect(content1).not.toContain('@critical');
      expect(content2).toContain('@release-one');
      expect(content3).toContain('@release-one');

      // And the 3 files should be updated
      expect(result.fileCount).toBe(3);

      // And the output should show "Renamed @critical to @release-one in 3 file(s) (5 occurrence(s))"
      expect(result.occurrenceCount).toBeGreaterThanOrEqual(3);
      expect(result.message).toMatch(/renamed.*@critical.*@release-one/i);
    });
  });

  describe('Scenario: Rename feature-level tag', () => {
    it('should rename tag at feature level', async () => {
      // Given I have a feature file with tag @deprecated at feature level
      const content = `@deprecated
Feature: Old Feature
  Scenario: Test scenario
    Given step 1
    When action
    Then result`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@deprecated --to=@legacy`
      const result = await retag({
        from: '@deprecated',
        to: '@legacy',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And the feature-level tag should change from @deprecated to @legacy
      expect(updatedContent).toContain('@legacy');
      expect(updatedContent).not.toContain('@deprecated');

      // And the file structure should be preserved
      expect(updatedContent).toContain('Feature: Old Feature');
      expect(updatedContent).toContain('Scenario: Test scenario');
      expect(updatedContent).toContain('Given step 1');
    });
  });

  describe('Scenario: Rename scenario-level tag', () => {
    it('should rename tags at scenario level', async () => {
      // Given I have scenarios tagged with @wip
      const content = `@feature
Feature: Test Feature

  @wip
  Scenario: WIP Scenario 1
    Given step 1

  @wip
  Scenario: WIP Scenario 2
    Given step 2

  @done
  Scenario: Done Scenario
    Given step 3`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@wip --to=@in-progress`
      const result = await retag({
        from: '@wip',
        to: '@in-progress',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And all scenario-level @wip tags should change to @in-progress
      expect(updatedContent).toContain('@in-progress');
      expect(updatedContent).not.toContain('@wip');
      expect(result.occurrenceCount).toBe(2);

      // And feature-level tags should remain unchanged
      expect(updatedContent).toContain('@feature');
    });
  });

  describe('Scenario: Rename tag used at both feature and scenario levels', () => {
    it('should rename tags at all levels', async () => {
      // Given I have files with @temp at feature level
      // And scenarios with @temp at scenario level
      const file1 = `@temp
Feature: Temp Feature
  Scenario: Regular scenario
    Given step`;

      const file2 = `@stable
Feature: Stable Feature

  @temp
  Scenario: Temp scenario 1
    Given step 1

  @temp
  Scenario: Temp scenario 2
    Given step 2`;

      await writeFile(join(testDir, 'spec/features/file1.feature'), file1);
      await writeFile(join(testDir, 'spec/features/file2.feature'), file2);

      // When I run `fspec retag --from=@temp --to=@temporary`
      const result = await retag({
        from: '@temp',
        to: '@temporary',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content1 = await readFile(
        join(testDir, 'spec/features/file1.feature'),
        'utf-8'
      );
      const content2 = await readFile(
        join(testDir, 'spec/features/file2.feature'),
        'utf-8'
      );

      // And all @temp tags should change to @temporary at both levels
      expect(content1).toContain('@temporary');
      expect(content1).not.toMatch(/@temp(\s|$)/);
      expect(content2).toContain('@temporary');
      expect(content2).not.toMatch(/@temp(\s|$)/);

      // And the output should show total occurrences changed
      expect(result.occurrenceCount).toBe(3); // 1 feature-level + 2 scenario-level
    });
  });

  describe('Scenario: Attempt to rename non-existent tag', () => {
    it('should return error for non-existent tag', async () => {
      // Given I have feature files with various tags
      const content = `@critical
Feature: Test
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // And no files contain the tag @nonexistent
      // When I run `fspec retag --from=@nonexistent --to=@new`
      const result = await retag({
        from: '@nonexistent',
        to: '@new',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag @nonexistent not found in any feature files"
      expect(result.error).toMatch(/@nonexistent.*not found/i);

      // And no files should be modified
      const content_after = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(content_after).toBe(content);
    });
  });

  describe('Scenario: Prevent rename to invalid tag format', () => {
    it('should reject invalid tag format', async () => {
      // Given I have files with tag @critical
      const content = `@critical
Feature: Test
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@critical --to=Phase1`
      const result = await retag({
        from: '@critical',
        to: 'Phase1',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Invalid tag format"
      expect(result.error).toMatch(/invalid.*tag.*format/i);

      // And no files should be modified
      const content_after = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(content_after).toBe(content);
    });
  });

  describe('Scenario: Dry run preview without making changes', () => {
    it('should preview changes without modifying files', async () => {
      // Given I have 10 files using @old-tag
      for (let i = 1; i <= 10; i++) {
        const content = `@old-tag
Feature: Feature ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/file${i}.feature`),
          content
        );
      }

      // When I run `fspec retag --from=@old-tag --to=@new-tag --dry-run`
      const result = await retag({
        from: '@old-tag',
        to: '@new-tag',
        dryRun: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Would rename @old-tag to @new-tag in 10 file(s)"
      expect(result.message).toMatch(
        /would rename.*@old-tag.*@new-tag.*10.*file/i
      );
      expect(result.fileCount).toBe(10);

      // And the output should list affected files
      expect(result.files).toBeDefined();
      expect(result.files?.length).toBe(10);

      // And no files should be modified
      for (let i = 1; i <= 10; i++) {
        const content = await readFile(
          join(testDir, `spec/features/file${i}.feature`),
          'utf-8'
        );
        expect(content).toContain('@old-tag');
        expect(content).not.toContain('@new-tag');
      }
    });
  });

  describe('Scenario: Rename tag with special characters', () => {
    it('should handle special characters in tags', async () => {
      // Given I have files tagged with @bug-#123
      const content = `@bug-#123
Feature: Bug Fix
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@bug-#123 --to=@issue-123`
      const result = await retag({
        from: '@bug-#123',
        to: '@issue-123',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And all @bug-#123 tags should change to @issue-123
      expect(updatedContent).toContain('@issue-123');
      expect(updatedContent).not.toContain('@bug-#123');

      // And files should remain valid Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Rename tag preserves other tags', () => {
    it('should preserve other tags and their order', async () => {
      // Given I have a feature with tags @critical @critical @api
      const content = `@critical @critical @api
Feature: Important Feature
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@critical --to=@v1-release`
      const result = await retag({
        from: '@critical',
        to: '@v1-release',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And the feature should have tags @v1-release @v1-release @api (all @critical replaced)
      expect(updatedContent).toContain('@v1-release');
      expect(updatedContent).not.toContain('@critical');
      expect(updatedContent).toContain('@api');

      // And tag order should be preserved
      const firstLine = updatedContent.split('\n')[0];
      expect(firstLine).toBe('@v1-release @v1-release @api');

      // And other tags should remain unchanged
      expect(updatedContent).toContain('@api');
    });
  });

  describe('Scenario: Rename tag validates Gherkin after changes', () => {
    it('should validate all modified files', async () => {
      // Given I have 20 files using @deprecated
      for (let i = 1; i <= 20; i++) {
        const content = `@deprecated
Feature: Feature ${i}
  Scenario: Test scenario
    Given step 1
    When action
    Then result`;
        await writeFile(
          join(testDir, `spec/features/file${i}.feature`),
          content
        );
      }

      // When I run `fspec retag --from=@deprecated --to=@legacy`
      const result = await retag({
        from: '@deprecated',
        to: '@legacy',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And all 20 files should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      for (let i = 1; i <= 20; i++) {
        const content = await readFile(
          join(testDir, `spec/features/file${i}.feature`),
          'utf-8'
        );
        expect(() => parser.parse(content)).not.toThrow();
      }

      // And the output should show "All modified files validated successfully"
      expect(result.message).toMatch(/validated successfully|renamed/i);
    });
  });

  describe('Scenario: Rename tag with multiple occurrences in single file', () => {
    it('should rename all occurrences in file', async () => {
      // Given I have a feature with @temp at feature level
      // And 3 scenarios with @temp at scenario level
      const content = `@temp
Feature: Temporary Feature

  @temp
  Scenario: Temp scenario 1
    Given step 1

  @temp
  Scenario: Temp scenario 2
    Given step 2

  @temp
  Scenario: Temp scenario 3
    Given step 3`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@temp --to=@temporary`
      const result = await retag({
        from: '@temp',
        to: '@temporary',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And all 4 occurrences should be renamed
      const occurrences = (updatedContent.match(/@temporary/g) || []).length;
      expect(occurrences).toBe(4);
      expect(updatedContent).not.toMatch(/@temp(\s|$)/);

      // And the output should show "4 occurrence(s)"
      expect(result.occurrenceCount).toBe(4);
    });
  });

  describe('Scenario: Prevent rename without required parameters', () => {
    it('should require both --from and --to', async () => {
      // Given I have feature files with various tags
      const content = `@old
Feature: Test
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec retag --from=@old` (missing --to)
      const result = await retag({
        from: '@old',
        to: '',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Both --from and --to are required"
      expect(result.error).toMatch(/both.*--from.*--to.*required/i);

      // And no files should be modified
      const content_after = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(content_after).toBe(content);
    });
  });
});
