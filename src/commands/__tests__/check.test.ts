import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { check } from '../check';

describe('Feature: Run All Validations', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-check');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: All validation checks pass', () => {
    it('should pass all checks', async () => {
      // Given I have 3 valid feature files with registered tags
      const tagsData = {
        categories: [
          {
            title: 'Phase Tags',
            tags: [
              { name: '@phase1', description: 'Phase 1 features' }
            ]
          }
        ]
      };

      await writeFile(join(testDir, 'spec/tags.json'), JSON.stringify(tagsData, null, 2));

      const feature1 = `@phase1
Feature: Feature 1

  Scenario: Test
    Given step
`;

      const feature2 = `@phase1
Feature: Feature 2

  Scenario: Test
    Given step
`;

      const feature3 = `@phase1
Feature: Feature 3

  Scenario: Test
    Given step
`;

      await writeFile(
        join(testDir, 'spec/features/feature1.feature'),
        feature1
      );
      await writeFile(
        join(testDir, 'spec/features/feature2.feature'),
        feature2
      );
      await writeFile(
        join(testDir, 'spec/features/feature3.feature'),
        feature3
      );

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Gherkin syntax: PASS"
      expect(result.gherkinStatus).toBe('PASS');

      // And the output should show "Tag validation: PASS"
      expect(result.tagStatus).toBe('PASS');

      // And the output should show "Formatting: PASS"
      expect(result.formatStatus).toBe('PASS');

      // And the output should show "All checks passed"
      expect(result.message).toMatch(/all checks passed/i);
    });
  });

  describe('Scenario: Gherkin syntax validation fails', () => {
    it('should fail on invalid Gherkin', async () => {
      // Given I have a feature file with invalid Gherkin syntax
      const invalidFeature = `Not valid Gherkin
Feature: Broken
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/broken.feature'),
        invalidFeature
      );

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Gherkin syntax: FAIL"
      expect(result.gherkinStatus).toBe('FAIL');

      // And the output should show the Gherkin error details
      expect(result.errors).toBeDefined();
      expect(result.errors?.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Tag validation fails', () => {
    it('should fail on unregistered tags', async () => {
      // Given I have a feature file with unregistered tag "@unknown-tag"
      const tagsData = {
        categories: [
          {
            title: 'Phase Tags',
            tags: [
              { name: '@phase1', description: 'Phase 1 features' }
            ]
          }
        ]
      };

      await writeFile(join(testDir, 'spec/tags.json'), JSON.stringify(tagsData, null, 2));

      const feature = `@unknown-tag
Feature: Test Feature
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/test.feature'), feature);

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Tag validation: FAIL"
      expect(result.tagStatus).toBe('FAIL');

      // And the output should show the unregistered tag "@unknown-tag"
      expect(result.errors).toBeDefined();
      expect(JSON.stringify(result.errors)).toContain('@unknown-tag');
    });
  });

  describe('Scenario: Formatting check fails', () => {
    it('should fail on incorrect formatting', async () => {
      // Given I have a feature file with incorrect formatting
      const unformatted = `Feature: Unformatted Feature
Scenario: Test
Given step`;

      await writeFile(
        join(testDir, 'spec/features/unformatted.feature'),
        unformatted
      );

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Formatting: FAIL"
      expect(result.formatStatus).toBe('FAIL');

      // And the output should show which files need formatting
      expect(result.errors).toBeDefined();
    });
  });

  describe('Scenario: Multiple validation failures', () => {
    it('should report all failures', async () => {
      // Given I have a feature file with invalid Gherkin syntax
      const invalidGherkin = `Invalid content
Feature: Broken
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/broken.feature'),
        invalidGherkin
      );

      // And I have a feature file with unregistered tag "@bad-tag"
      const tagsData = {
        categories: [
          {
            title: 'Phase Tags',
            tags: [
              { name: '@phase1', description: 'Phase 1 features' }
            ]
          }
        ]
      };

      await writeFile(join(testDir, 'spec/tags.json'), JSON.stringify(tagsData, null, 2));

      const badTag = `@bad-tag
Feature: Bad Tag
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/badtag.feature'), badTag);

      // And I have a feature file with incorrect formatting
      const unformatted = `Feature: Unformatted
Scenario: Test
Given step`;

      await writeFile(
        join(testDir, 'spec/features/unformatted.feature'),
        unformatted
      );

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Gherkin syntax: FAIL"
      expect(result.gherkinStatus).toBe('FAIL');

      // And the output should show "Tag validation: FAIL"
      expect(result.tagStatus).toBe('FAIL');

      // And the output should show "Formatting: FAIL" or "SKIP" (may skip if gherkin fails)
      expect(['FAIL', 'SKIP']).toContain(result.formatStatus);

      // And the output should list all errors
      expect(result.errors).toBeDefined();
      expect(result.errors!.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: No feature files exist', () => {
    it('should handle no feature files', async () => {
      // Given I have no feature files
      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No feature files found"
      expect(result.message).toMatch(/no feature files found/i);
    });
  });

  describe('Scenario: Check reports file counts', () => {
    it('should show file count', async () => {
      // Given I have 10 valid feature files
      const tagsData = {
        categories: [
          {
            title: 'Phase Tags',
            tags: [
              { name: '@phase1', description: 'Phase 1 features' }
            ]
          }
        ]
      };

      await writeFile(join(testDir, 'spec/tags.json'), JSON.stringify(tagsData, null, 2));

      for (let i = 1; i <= 10; i++) {
        const feature = `@phase1
Feature: Feature ${i}

  Scenario: Test
    Given step
`;
        await writeFile(
          join(testDir, `spec/features/feature${i}.feature`),
          feature
        );
      }

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "Checked 10 feature file(s)"
      expect(result.fileCount).toBe(10);
    });
  });

  describe('Scenario: Check validates all feature files', () => {
    it('should validate all files and report errors', async () => {
      // Given I have 5 feature files
      // And 1 file has invalid Gherkin syntax
      for (let i = 1; i <= 4; i++) {
        const feature = `Feature: Feature ${i}
  Scenario: Test
    Given step`;
        await writeFile(
          join(testDir, `spec/features/feature${i}.feature`),
          feature
        );
      }

      const invalidFeature = `Invalid line
Feature: Feature 5
  Scenario: Test
    Given step`;
      await writeFile(
        join(testDir, 'spec/features/feature5.feature'),
        invalidFeature
      );

      // When I run `fspec check`
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show which file has invalid syntax
      expect(result.errors).toBeDefined();
      expect(JSON.stringify(result.errors)).toContain('feature5.feature');
    });
  });

  describe('Scenario: Check with verbose output', () => {
    it('should show detailed results', async () => {
      // Given I have 3 valid feature files
      const tagsData = {
        categories: [
          {
            title: 'Phase Tags',
            tags: [
              { name: '@phase1', description: 'Phase 1 features' }
            ]
          }
        ]
      };

      await writeFile(join(testDir, 'spec/tags.json'), JSON.stringify(tagsData, null, 2));

      for (let i = 1; i <= 3; i++) {
        const feature = `@phase1
Feature: Feature ${i}

  Scenario: Test
    Given step
`;
        await writeFile(
          join(testDir, `spec/features/feature${i}.feature`),
          feature
        );
      }

      // When I run `fspec check --verbose`
      const result = await check({
        verbose: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show detailed check results
      expect(result.details).toBeDefined();
    });
  });

  describe('Scenario: Check runs quickly on large repositories', () => {
    it('should complete in reasonable time', async () => {
      // Given I have 100 valid feature files
      const tagsData = {
        categories: [
          {
            title: 'Phase Tags',
            tags: [
              { name: '@phase1', description: 'Phase 1 features' }
            ]
          }
        ]
      };

      await writeFile(join(testDir, 'spec/tags.json'), JSON.stringify(tagsData, null, 2));

      for (let i = 1; i <= 100; i++) {
        const feature = `@phase1
Feature: Feature ${i}

  Scenario: Test
    Given step
`;
        await writeFile(
          join(testDir, `spec/features/feature${i}.feature`),
          feature
        );
      }

      // When I run `fspec check`
      const startTime = Date.now();
      const result = await check({
        cwd: testDir,
      });
      const duration = Date.now() - startTime;

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the command should complete within a reasonable time
      expect(duration).toBeLessThan(10000); // Less than 10 seconds
    });
  });

  describe('Scenario: Check output is CI-friendly', () => {
    it('should provide machine-readable output', async () => {
      // Given I have a feature file with validation errors
      const invalidFeature = `Invalid content
Feature: Broken
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/broken.feature'),
        invalidFeature
      );

      // When I run `fspec check` in a CI environment
      const result = await check({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And error messages should include file paths and line numbers
      expect(result.errors).toBeDefined();
      expect(result.errors!.length).toBeGreaterThan(0);
    });
  });
});
