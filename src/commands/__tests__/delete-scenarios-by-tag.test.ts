import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { deleteScenariosByTag } from '../delete-scenarios-by-tag';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Bulk Delete Scenarios by Tag', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('delete-scenarios-by-tag');
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Delete scenarios by single tag from one feature file', () => {
    it('should remove only tagged scenarios and preserve others', async () => {
      // Given I have a feature file with 5 scenarios
      // And 2 scenarios are tagged with @deprecated
      const featureContent = `Feature: Test Feature

  Scenario: Scenario 1
    Given step 1

  @deprecated
  Scenario: Scenario 2
    Given step 2

  Scenario: Scenario 3
    Given step 3

  @deprecated
  Scenario: Scenario 4
    Given step 4

  Scenario: Scenario 5
    Given step 5
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec delete-scenarios --tag=@deprecated`
      const result = await deleteScenariosByTag({
        tags: ['@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the 2 @deprecated scenarios should be removed
      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('Scenario 2');
      expect(updatedContent).not.toContain('Scenario 4');

      // And the 3 non-tagged scenarios should remain
      expect(updatedContent).toContain('Scenario: Scenario 1');
      expect(updatedContent).toContain('Scenario: Scenario 3');
      expect(updatedContent).toContain('Scenario: Scenario 5');

      // And the feature file structure should be preserved
      expect(updatedContent).toContain('Feature: Test Feature');

      // And the output should show "Deleted 2 scenario(s) from 1 file(s)"
      expect(result.deletedCount).toBe(2);
      expect(result.fileCount).toBe(1);
    });
  });

  describe('Scenario: Delete scenarios by single tag across multiple files', () => {
    it('should remove tagged scenarios from all files', async () => {
      // Given I have 3 feature files with scenarios
      // And 5 scenarios across files are tagged with @obsolete
      const file1 = `Feature: File 1

  @obsolete
  Scenario: Old scenario 1
    Given step 1

  Scenario: Current scenario
    Given step 2
`;

      const file2 = `Feature: File 2

  @obsolete
  Scenario: Old scenario 2
    Given step 3

  @obsolete
  Scenario: Old scenario 3
    Given step 4
`;

      const file3 = `Feature: File 3

  @obsolete
  Scenario: Old scenario 4
    Given step 5

  @obsolete
  Scenario: Old scenario 5
    Given step 6

  Scenario: Current scenario
    Given step 7
`;

      await writeFile(join(testDir, 'spec/features/file1.feature'), file1);
      await writeFile(join(testDir, 'spec/features/file2.feature'), file2);
      await writeFile(join(testDir, 'spec/features/file3.feature'), file3);

      // When I run `fspec delete-scenarios --tag=@obsolete`
      const result = await deleteScenariosByTag({
        tags: ['@obsolete'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And all 5 @obsolete scenarios should be removed
      expect(result.deletedCount).toBe(5);

      // And the output should show "Deleted 5 scenario(s) from 3 file(s)"
      expect(result.fileCount).toBe(3);

      // And all feature files should remain valid Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

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

      expect(() => parser.parse(content1)).not.toThrow();
      expect(() => parser.parse(content2)).not.toThrow();
      expect(() => parser.parse(content3)).not.toThrow();
    });
  });

  describe('Scenario: Delete scenarios by multiple tags with AND logic', () => {
    it('should remove only scenarios with all specified tags', async () => {
      // Given I have scenarios tagged with various combinations
      const featureContent = `Feature: Test Feature

  @critical @deprecated
  Scenario: Both tags 1
    Given step 1

  @critical
  Scenario: Only phase1
    Given step 2

  @deprecated
  Scenario: Only deprecated
    Given step 3

  @critical @deprecated
  Scenario: Both tags 2
    Given step 4

  @critical
  Scenario: Only phase1 again
    Given step 5

  @critical
  Scenario: Only phase1 third
    Given step 6
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec delete-scenarios --tag=@critical --tag=@deprecated`
      const result = await deleteScenariosByTag({
        tags: ['@critical', '@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And only the 2 scenarios with both tags should be removed
      expect(result.deletedCount).toBe(2);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('Both tags 1');
      expect(updatedContent).not.toContain('Both tags 2');

      // And the 4 scenarios without both tags should remain
      expect(updatedContent).toContain('Only phase1');
      expect(updatedContent).toContain('Only deprecated');
      expect(updatedContent).toContain('Only phase1 again');
      expect(updatedContent).toContain('Only phase1 third');
    });
  });

  describe('Scenario: Dry run preview without making changes', () => {
    it('should show what would be deleted without modifying files', async () => {
      // Given I have 10 scenarios tagged with @test
      const scenarios: string[] = [];
      for (let i = 1; i <= 10; i++) {
        scenarios.push(`  @test
  Scenario: Test scenario ${i}
    Given step ${i}
`);
      }

      const featureContent = `Feature: Test Feature

${scenarios.join('\n')}`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );
      const originalContent = featureContent;

      // When I run `fspec delete-scenarios --tag=@test --dry-run`
      const result = await deleteScenariosByTag({
        tags: ['@test'],
        dryRun: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Would delete 10 scenario(s) from X file(s)"
      expect(result.message).toMatch(/would delete.*10.*scenario/i);
      expect(result.deletedCount).toBe(10);
      expect(result.fileCount).toBe(1);

      // And the output should list the scenarios that would be deleted
      expect(result.scenarios).toBeDefined();
      expect(result.scenarios?.length).toBe(10);

      // And no files should be modified
      const currentContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(currentContent).toBe(originalContent);

      // And all 10 scenarios should remain in files
      expect(currentContent).toContain('Test scenario 1');
      expect(currentContent).toContain('Test scenario 10');
    });
  });

  describe('Scenario: Skip feature files with no matching scenarios', () => {
    it('should only modify files with matching scenarios', async () => {
      // Given I have 5 feature files
      const fileWithOld1 = `Feature: File 1
  @old
  Scenario: Old
    Given step 1
`;

      const fileWithOld2 = `Feature: File 2
  @old
  Scenario: Old
    Given step 2
`;

      const fileWithoutOld1 = `Feature: File 3
  Scenario: Current
    Given step 3
`;

      const fileWithoutOld2 = `Feature: File 4
  Scenario: Current
    Given step 4
`;

      const fileWithoutOld3 = `Feature: File 5
  Scenario: Current
    Given step 5
`;

      await writeFile(
        join(testDir, 'spec/features/file1.feature'),
        fileWithOld1
      );
      await writeFile(
        join(testDir, 'spec/features/file2.feature'),
        fileWithOld2
      );
      await writeFile(
        join(testDir, 'spec/features/file3.feature'),
        fileWithoutOld1
      );
      await writeFile(
        join(testDir, 'spec/features/file4.feature'),
        fileWithoutOld2
      );
      await writeFile(
        join(testDir, 'spec/features/file5.feature'),
        fileWithoutOld3
      );

      const contentFile3Before = fileWithoutOld1;
      const contentFile4Before = fileWithoutOld2;
      const contentFile5Before = fileWithoutOld3;

      // When I run `fspec delete-scenarios --tag=@old`
      const result = await deleteScenariosByTag({
        tags: ['@old'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And only the 2 files with @old scenarios should be modified
      expect(result.fileCount).toBe(2);

      // And the 3 files without @old scenarios should remain unchanged
      const contentFile3After = await readFile(
        join(testDir, 'spec/features/file3.feature'),
        'utf-8'
      );
      const contentFile4After = await readFile(
        join(testDir, 'spec/features/file4.feature'),
        'utf-8'
      );
      const contentFile5After = await readFile(
        join(testDir, 'spec/features/file5.feature'),
        'utf-8'
      );

      expect(contentFile3After).toBe(contentFile3Before);
      expect(contentFile4After).toBe(contentFile4Before);
      expect(contentFile5After).toBe(contentFile5Before);
    });
  });

  describe('Scenario: Handle feature with all scenarios deleted', () => {
    it('should preserve feature header when all scenarios removed', async () => {
      // Given I have a feature file with 3 scenarios
      // And all 3 scenarios are tagged with @remove
      const featureContent = `Feature: Test Feature

  @remove
  Scenario: Remove 1
    Given step 1

  @remove
  Scenario: Remove 2
    Given step 2

  @remove
  Scenario: Remove 3
    Given step 3
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec delete-scenarios --tag=@remove`
      const result = await deleteScenariosByTag({
        tags: ['@remove'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And all 3 scenarios should be removed
      expect(result.deletedCount).toBe(3);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And the feature file should contain only the Feature header
      expect(updatedContent).toContain('Feature: Test Feature');
      expect(updatedContent).not.toContain('Remove 1');
      expect(updatedContent).not.toContain('Remove 2');
      expect(updatedContent).not.toContain('Remove 3');

      // And the feature file should remain valid Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();

      // And the output should show "Deleted 3 scenario(s) from 1 file(s)"
      expect(result.fileCount).toBe(1);
    });
  });

  describe('Scenario: Delete scenarios preserves feature tags', () => {
    it('should keep feature-level tags intact', async () => {
      // Given I have a feature file tagged with @feature-tag
      // And it has 2 scenarios tagged @scenario-tag
      const featureContent = `@feature-tag
Feature: Test Feature

  @scenario-tag
  Scenario: Scenario 1
    Given step 1

  @scenario-tag
  Scenario: Scenario 2
    Given step 2

  Scenario: Scenario 3
    Given step 3
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec delete-scenarios --tag=@scenario-tag`
      const result = await deleteScenariosByTag({
        tags: ['@scenario-tag'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And the @feature-tag should remain on the feature
      expect(updatedContent).toContain('@feature-tag');
      expect(updatedContent).toContain('Feature: Test Feature');

      // And the 2 scenarios should be removed
      expect(updatedContent).not.toContain('Scenario 1');
      expect(updatedContent).not.toContain('Scenario 2');

      // And the feature structure should be preserved
      expect(updatedContent).toContain('Scenario: Scenario 3');
    });
  });

  describe('Scenario: Delete scenarios preserves Background section', () => {
    it('should keep Background section intact', async () => {
      // Given I have a feature with a Background section
      // And 2 scenarios tagged @cleanup
      const featureContent = `Feature: Test Feature

  Background: User Story
    As a developer
    I want background context
    So that scenarios have shared setup

  @cleanup
  Scenario: Cleanup 1
    Given step 1

  Scenario: Keep this
    Given step 2

  @cleanup
  Scenario: Cleanup 2
    Given step 3
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec delete-scenarios --tag=@cleanup`
      const result = await deleteScenariosByTag({
        tags: ['@cleanup'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And the Background section should remain intact
      expect(updatedContent).toContain('Background: User Story');
      expect(updatedContent).toContain('As a developer');
      expect(updatedContent).toContain('I want background context');

      // And only the 2 @cleanup scenarios should be removed
      expect(updatedContent).not.toContain('Cleanup 1');
      expect(updatedContent).not.toContain('Cleanup 2');
      expect(updatedContent).toContain('Keep this');

      // And the feature should remain valid Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Attempt to delete with no matching scenarios', () => {
    it('should report no matches and not modify files', async () => {
      // Given I have feature files with various scenarios
      const featureContent = `Feature: Test Feature

  @critical
  Scenario: Scenario 1
    Given step 1

  @high
  Scenario: Scenario 2
    Given step 2
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );
      const originalContent = featureContent;

      // And no scenarios are tagged with @nonexistent
      // When I run `fspec delete-scenarios --tag=@nonexistent`
      const result = await deleteScenariosByTag({
        tags: ['@nonexistent'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No scenarios found matching tags"
      expect(result.message).toMatch(/no scenarios found/i);
      expect(result.deletedCount).toBe(0);
      expect(result.fileCount).toBe(0);

      // And no files should be modified
      const currentContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(currentContent).toBe(originalContent);
    });
  });

  describe('Scenario: Delete scenarios with special characters in tags', () => {
    it('should handle special characters in tag names', async () => {
      // Given I have scenarios tagged with @bug-#123
      const featureContent = `Feature: Test Feature

  @bug-#123
  Scenario: Bug scenario
    Given step 1

  @feature
  Scenario: Feature scenario
    Given step 2
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec delete-scenarios --tag=@bug-#123`
      const result = await deleteScenariosByTag({
        tags: ['@bug-#123'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // And scenarios with @bug-#123 should be removed
      expect(updatedContent).not.toContain('Bug scenario');
      expect(updatedContent).toContain('Feature scenario');

      // And the feature files should remain valid Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Validate Gherkin syntax after bulk deletion', () => {
    it('should validate all modified files', async () => {
      // Given I have 20 scenarios tagged @temp across 5 files
      for (let fileNum = 1; fileNum <= 5; fileNum++) {
        const scenarios: string[] = [];
        for (let scenarioNum = 1; scenarioNum <= 4; scenarioNum++) {
          scenarios.push(`  @temp
  Scenario: File ${fileNum} Scenario ${scenarioNum}
    Given step ${scenarioNum}
`);
        }
        const content = `Feature: File ${fileNum}

${scenarios.join('\n')}`;
        await writeFile(
          join(testDir, `spec/features/file${fileNum}.feature`),
          content
        );
      }

      // When I run `fspec delete-scenarios --tag=@temp`
      const result = await deleteScenariosByTag({
        tags: ['@temp'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);
      expect(result.deletedCount).toBe(20);
      expect(result.fileCount).toBe(5);

      // And all 5 modified files should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      for (let fileNum = 1; fileNum <= 5; fileNum++) {
        const content = await readFile(
          join(testDir, `spec/features/file${fileNum}.feature`),
          'utf-8'
        );
        // And the Gherkin parser should successfully parse all files
        expect(() => parser.parse(content)).not.toThrow();
      }

      // And the output should show "All modified files validated successfully"
      expect(result.message).toMatch(
        /validated successfully|deleted.*scenario/i
      );
    });
  });

  describe('Scenario: Delete scenarios updates file line counts', () => {
    it('should reduce file size appropriately', async () => {
      // Given I have a feature file with 100 lines
      const scenarios: string[] = [];

      // 10 scenarios with @remove (5 lines each = 50 lines)
      for (let i = 1; i <= 10; i++) {
        scenarios.push(`  @remove
  Scenario: Remove ${i}
    Given step ${i}
    When action ${i}
    Then result ${i}
`);
      }

      // 10 scenarios without @remove (5 lines each = 50 lines)
      for (let i = 11; i <= 20; i++) {
        scenarios.push(`  Scenario: Keep ${i}
    Given step ${i}
    When action ${i}
    Then result ${i}
`);
      }

      const featureContent = `Feature: Large Feature

${scenarios.join('\n')}`;

      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );
      const originalLineCount = featureContent.split('\n').length;

      // When I run `fspec delete-scenarios --tag=@remove`
      const result = await deleteScenariosByTag({
        tags: ['@remove'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);
      expect(result.deletedCount).toBe(10);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      const updatedLineCount = updatedContent.split('\n').length;

      // And the file should be approximately 50 lines (removed ~50 lines)
      expect(originalLineCount - updatedLineCount).toBeGreaterThan(40);
      expect(originalLineCount - updatedLineCount).toBeLessThan(70);

      // And the file structure should remain valid
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });
});
