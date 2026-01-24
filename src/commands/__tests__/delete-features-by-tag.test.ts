import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readdir, access } from 'fs/promises';
import { join } from 'path';
import { deleteFeaturesByTag } from '../delete-features-by-tag';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Bulk Delete Feature Files by Tag', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('delete-features-by-tag');
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Delete feature files by single tag', () => {
    it('should delete only files with specified tag', async () => {
      // Given I have 5 feature files
      // And 2 files are tagged with @deprecated
      const file1 = `@deprecated
Feature: Old Feature 1
  Scenario: Test
    Given step`;

      const file2 = `@current
Feature: Current Feature
  Scenario: Test
    Given step`;

      const file3 = `@deprecated
Feature: Old Feature 2
  Scenario: Test
    Given step`;

      const file4 = `@active
Feature: Active Feature
  Scenario: Test
    Given step`;

      const file5 = `@new
Feature: New Feature
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/old1.feature'), file1);
      await writeFile(join(testDir, 'spec/features/current.feature'), file2);
      await writeFile(join(testDir, 'spec/features/old2.feature'), file3);
      await writeFile(join(testDir, 'spec/features/active.feature'), file4);
      await writeFile(join(testDir, 'spec/features/new.feature'), file5);

      // When I run `fspec delete-features --tag=@deprecated`
      const result = await deleteFeaturesByTag({
        tags: ['@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the 2 @deprecated feature files should be deleted
      await expect(
        access(join(testDir, 'spec/features/old1.feature'))
      ).rejects.toThrow();
      await expect(
        access(join(testDir, 'spec/features/old2.feature'))
      ).rejects.toThrow();

      // And the 3 non-tagged feature files should remain
      await expect(
        access(join(testDir, 'spec/features/current.feature'))
      ).resolves.toBeUndefined();
      await expect(
        access(join(testDir, 'spec/features/active.feature'))
      ).resolves.toBeUndefined();
      await expect(
        access(join(testDir, 'spec/features/new.feature'))
      ).resolves.toBeUndefined();

      // And the output should show "Deleted 2 feature file(s)"
      expect(result.deletedCount).toBe(2);
      expect(result.message).toMatch(/deleted.*2.*file/i);
    });
  });

  describe('Scenario: Delete feature files by multiple tags with AND logic', () => {
    it('should delete only files with all specified tags', async () => {
      // Given I have feature files with various tag combinations
      const bothTags1 = `@critical @deprecated
Feature: Both 1
  Scenario: Test
    Given step`;

      const bothTags2 = `@critical @deprecated
Feature: Both 2
  Scenario: Test
    Given step`;

      const onlyPhase1_1 = `@critical
Feature: Only Phase1 A
  Scenario: Test
    Given step`;

      const onlyPhase1_2 = `@critical
Feature: Only Phase1 B
  Scenario: Test
    Given step`;

      const onlyPhase1_3 = `@critical
Feature: Only Phase1 C
  Scenario: Test
    Given step`;

      const onlyDeprecated = `@deprecated
Feature: Only Deprecated
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/both1.feature'), bothTags1);
      await writeFile(join(testDir, 'spec/features/both2.feature'), bothTags2);
      await writeFile(
        join(testDir, 'spec/features/phase1a.feature'),
        onlyPhase1_1
      );
      await writeFile(
        join(testDir, 'spec/features/phase1b.feature'),
        onlyPhase1_2
      );
      await writeFile(
        join(testDir, 'spec/features/phase1c.feature'),
        onlyPhase1_3
      );
      await writeFile(
        join(testDir, 'spec/features/deprecated.feature'),
        onlyDeprecated
      );

      // When I run `fspec delete-features --tag=@critical --tag=@deprecated`
      const result = await deleteFeaturesByTag({
        tags: ['@critical', '@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And only the 2 files with both tags should be deleted
      expect(result.deletedCount).toBe(2);
      await expect(
        access(join(testDir, 'spec/features/both1.feature'))
      ).rejects.toThrow();
      await expect(
        access(join(testDir, 'spec/features/both2.feature'))
      ).rejects.toThrow();

      // And the 4 files without both tags should remain
      await expect(
        access(join(testDir, 'spec/features/phase1a.feature'))
      ).resolves.toBeUndefined();
      await expect(
        access(join(testDir, 'spec/features/phase1b.feature'))
      ).resolves.toBeUndefined();
      await expect(
        access(join(testDir, 'spec/features/phase1c.feature'))
      ).resolves.toBeUndefined();
      await expect(
        access(join(testDir, 'spec/features/deprecated.feature'))
      ).resolves.toBeUndefined();

      // And the output should show "Deleted 2 feature file(s)"
      expect(result.message).toMatch(/deleted.*2.*file/i);
    });
  });

  describe('Scenario: Dry run preview without making changes', () => {
    it('should preview deletions without removing files', async () => {
      // Given I have 10 feature files tagged with @obsolete
      for (let i = 1; i <= 10; i++) {
        const content = `@obsolete
Feature: Obsolete ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/obsolete${i}.feature`),
          content
        );
      }

      // When I run `fspec delete-features --tag=@obsolete --dry-run`
      const result = await deleteFeaturesByTag({
        tags: ['@obsolete'],
        dryRun: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Would delete 10 feature file(s)"
      expect(result.message).toMatch(/would delete.*10.*file/i);
      expect(result.deletedCount).toBe(10);

      // And the output should list the files that would be deleted
      expect(result.files).toBeDefined();
      expect(result.files?.length).toBe(10);

      // And no files should be deleted
      // And all 10 files should remain on filesystem
      for (let i = 1; i <= 10; i++) {
        await expect(
          access(join(testDir, `spec/features/obsolete${i}.feature`))
        ).resolves.toBeUndefined();
      }
    });
  });

  describe('Scenario: Attempt to delete with no matching files', () => {
    it('should report no matches without errors', async () => {
      // Given I have feature files with various tags
      const file1 = `@critical
Feature: Phase 1
  Scenario: Test
    Given step`;

      const file2 = `@high
Feature: Phase 2
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/phase1.feature'), file1);
      await writeFile(join(testDir, 'spec/features/phase2.feature'), file2);

      // And no files are tagged with @nonexistent
      // When I run `fspec delete-features --tag=@nonexistent`
      const result = await deleteFeaturesByTag({
        tags: ['@nonexistent'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No feature files found matching tags"
      expect(result.message).toMatch(/no.*file.*found/i);
      expect(result.deletedCount).toBe(0);

      // And no files should be deleted
      await expect(
        access(join(testDir, 'spec/features/phase1.feature'))
      ).resolves.toBeUndefined();
      await expect(
        access(join(testDir, 'spec/features/phase2.feature'))
      ).resolves.toBeUndefined();
    });
  });

  describe('Scenario: Delete feature files with special characters in tags', () => {
    it('should handle special characters in tag names', async () => {
      // Given I have 3 files tagged with @bug-#123
      for (let i = 1; i <= 3; i++) {
        const content = `@bug-#123
Feature: Bug ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/bug${i}.feature`),
          content
        );
      }

      // When I run `fspec delete-features --tag=@bug-#123`
      const result = await deleteFeaturesByTag({
        tags: ['@bug-#123'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the 3 files with @bug-#123 should be deleted
      expect(result.deletedCount).toBe(3);
      await expect(
        access(join(testDir, 'spec/features/bug1.feature'))
      ).rejects.toThrow();
      await expect(
        access(join(testDir, 'spec/features/bug2.feature'))
      ).rejects.toThrow();
      await expect(
        access(join(testDir, 'spec/features/bug3.feature'))
      ).rejects.toThrow();

      // And the output should show "Deleted 3 feature file(s)"
      expect(result.message).toMatch(/deleted.*3.*file/i);
    });
  });

  describe('Scenario: Prevent deletion without tag filter', () => {
    it('should require at least one tag', async () => {
      // Given I have 20 feature files in spec/features/
      for (let i = 1; i <= 20; i++) {
        const content = `@tag${i}
Feature: Feature ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/file${i}.feature`),
          content
        );
      }

      // When I run `fspec delete-features` (no tags)
      const result = await deleteFeaturesByTag({
        tags: [],
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "At least one --tag is required"
      expect(result.error).toMatch(/at least one.*tag.*required/i);

      // And no files should be deleted
      // And all 20 files should remain
      const files = await readdir(join(testDir, 'spec/features'));
      expect(files.length).toBe(20);
    });
  });

  describe('Scenario: Delete all files matching single tag', () => {
    it('should delete all matching files leaving directory empty', async () => {
      // Given I have 15 feature files
      // And all 15 files are tagged with @remove-all
      for (let i = 1; i <= 15; i++) {
        const content = `@remove-all
Feature: Remove ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/remove${i}.feature`),
          content
        );
      }

      // When I run `fspec delete-features --tag=@remove-all`
      const result = await deleteFeaturesByTag({
        tags: ['@remove-all'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And all 15 files should be deleted
      expect(result.deletedCount).toBe(15);

      // And spec/features/ directory should be empty
      const files = await readdir(join(testDir, 'spec/features'));
      expect(files.length).toBe(0);

      // And the output should show "Deleted 15 feature file(s)"
      expect(result.message).toMatch(/deleted.*15.*file/i);
    });
  });

  describe('Scenario: Delete files updates directory structure', () => {
    it('should maintain correct file count after deletion', async () => {
      // Given I have feature files in spec/features/ directory
      // And 5 files are tagged with @cleanup
      for (let i = 1; i <= 5; i++) {
        const content = `@cleanup
Feature: Cleanup ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/cleanup${i}.feature`),
          content
        );
      }

      // And the directory contains 12 total files
      for (let i = 6; i <= 12; i++) {
        const content = `@keep
Feature: Keep ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/keep${i}.feature`),
          content
        );
      }

      // When I run `fspec delete-features --tag=@cleanup`
      const result = await deleteFeaturesByTag({
        tags: ['@cleanup'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And 5 files should be deleted
      expect(result.deletedCount).toBe(5);

      // And 7 files should remain
      // And the directory should contain exactly 7 files
      const files = await readdir(join(testDir, 'spec/features'));
      expect(files.length).toBe(7);
    });
  });

  describe('Scenario: Delete files with nested directory paths', () => {
    it('should delete files from spec/features directory', async () => {
      // Given I have feature files with various paths
      // And files are located at spec/features/file.feature
      const old1 = `@old
Feature: Old 1
  Scenario: Test
    Given step`;

      const old2 = `@old
Feature: Old 2
  Scenario: Test
    Given step`;

      const old3 = `@old
Feature: Old 3
  Scenario: Test
    Given step`;

      const keep = `@keep
Feature: Keep
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/old1.feature'), old1);
      await writeFile(join(testDir, 'spec/features/old2.feature'), old2);
      await writeFile(join(testDir, 'spec/features/old3.feature'), old3);
      await writeFile(join(testDir, 'spec/features/keep.feature'), keep);

      // And 3 files are tagged with @old
      // When I run `fspec delete-features --tag=@old`
      const result = await deleteFeaturesByTag({
        tags: ['@old'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the 3 @old files should be deleted from spec/features/
      expect(result.deletedCount).toBe(3);
      await expect(
        access(join(testDir, 'spec/features/old1.feature'))
      ).rejects.toThrow();
      await expect(
        access(join(testDir, 'spec/features/old2.feature'))
      ).rejects.toThrow();
      await expect(
        access(join(testDir, 'spec/features/old3.feature'))
      ).rejects.toThrow();

      // And the remaining files should be intact
      await expect(
        access(join(testDir, 'spec/features/keep.feature'))
      ).resolves.toBeUndefined();
    });
  });

  describe('Scenario: Confirm deletion of multiple files', () => {
    it('should list all deleted files', async () => {
      // Given I have 8 feature files tagged with @deprecated
      for (let i = 1; i <= 8; i++) {
        const content = `@deprecated
Feature: Phase 0 Feature ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/phase0-${i}.feature`),
          content
        );
      }

      // When I run `fspec delete-features --tag=@deprecated`
      const result = await deleteFeaturesByTag({
        tags: ['@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And all 8 files should be deleted
      expect(result.deletedCount).toBe(8);
      for (let i = 1; i <= 8; i++) {
        await expect(
          access(join(testDir, `spec/features/phase0-${i}.feature`))
        ).rejects.toThrow();
      }

      // And the output should list each deleted file
      expect(result.files).toBeDefined();
      expect(result.files?.length).toBe(8);

      // And the output should show total count "Deleted 8 feature file(s)"
      expect(result.message).toMatch(/deleted.*8.*file/i);
    });
  });
});
