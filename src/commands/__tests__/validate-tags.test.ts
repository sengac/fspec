import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { validateTags } from '../validate-tags';

describe('Feature: Validate Feature File Tags Against Registry', () => {
  let testDir: string;
  let originalCwd: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    originalCwd = process.cwd();
    process.chdir(testDir);

    // Create tags.json with standard tags
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });

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
        {
          name: 'Component Tags',
          description: 'Architectural components',
          required: false,
          tags: [
            { name: '@cli', description: 'CLI' },
            { name: '@parser', description: 'Parser' },
          ],
        },
        {
          name: 'Feature Group Tags',
          description: 'Functional areas',
          required: false,
          tags: [
            { name: '@feature-management', description: 'Feature Management' },
            { name: '@tag-management', description: 'Tag Management' },
            { name: '@validation', description: 'Validation' },
            { name: '@authentication', description: 'Authentication' },
          ],
        },
        {
          name: 'Testing Tags',
          description: 'Test-related tags',
          required: false,
          tags: [
            { name: '@smoke', description: 'Smoke tests' },
            { name: '@regression', description: 'Regression tests' },
            { name: '@edge-case', description: 'Edge case tests' },
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
      join(specDir, 'tags.json'),
      JSON.stringify(tagsJson, null, 2)
    );
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Validate tags in a compliant feature file', () => {
    it('should pass when all tags are registered', async () => {
      // Given I have a feature file with all registered tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const validContent = `@phase1 @cli @authentication
Feature: User Authentication

  Scenario: Login
    Given valid credentials
    When I login
    Then I am logged in`;

      await writeFile(join(featuresDir, 'auth.feature'), validContent);

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/auth.feature',
        cwd: testDir,
      });

      // Then it should pass
      expect(result.results).toHaveLength(1);
      expect(result.results[0].valid).toBe(true);
      expect(result.results[0].errors).toHaveLength(0);
      expect(result.validCount).toBe(1);
      expect(result.invalidCount).toBe(0);
    });
  });

  describe('Scenario: Detect unregistered tag', () => {
    it('should report unregistered tags with suggestions', async () => {
      // Given I have a feature file with an unregistered tag
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const invalidContent = `@phase1 @cli @validation @custom-tag
Feature: API Endpoints

  Scenario: Test
    Given something`;

      await writeFile(join(featuresDir, 'api.feature'), invalidContent);

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/api.feature',
        cwd: testDir,
      });

      // Then it should report the unregistered tag
      expect(result.results[0].valid).toBe(false);
      expect(result.results[0].errors).toHaveLength(1);
      expect(result.results[0].errors[0].tag).toBe('@custom-tag');
      expect(result.results[0].errors[0].message).toContain(
        'Unregistered tag: @custom-tag'
      );
      expect(result.results[0].errors[0].suggestion).toContain('tags.json');
    });
  });

  describe('Scenario: Validate all feature files', () => {
    it('should validate all files and report summary', async () => {
      // Given I have multiple feature files
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'valid.feature'),
        '@phase1 @cli @authentication\nFeature: Valid\n  Scenario: Test\n    Given test'
      );

      await writeFile(
        join(featuresDir, 'invalid.feature'),
        '@phase1 @cli @validation @experimental\nFeature: Invalid\n  Scenario: Test\n    Given test'
      );

      // When I run validate-tags
      const result = await validateTags({ cwd: testDir });

      // Then it should report summary
      expect(result.results).toHaveLength(2);
      expect(result.validCount).toBe(1);
      expect(result.invalidCount).toBe(1);
    });
  });

  describe('Scenario: Detect missing required phase tag', () => {
    it('should report missing phase tag', async () => {
      // Given I have a feature file without a phase tag
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const noPhaseContent = `@cli @validation
Feature: Broken Feature

  Scenario: Test
    Given something`;

      await writeFile(join(featuresDir, 'broken.feature'), noPhaseContent);

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/broken.feature',
        cwd: testDir,
      });

      // Then it should report missing phase tag
      expect(result.results[0].valid).toBe(false);
      const phaseError = result.results[0].errors.find(e =>
        e.message.includes('phase tag')
      );
      expect(phaseError).toBeDefined();
      expect(phaseError!.suggestion).toContain('Add a phase tag');
    });
  });

  describe('Scenario: Detect missing required component tag', () => {
    it('should report missing component tag', async () => {
      // Given I have a feature file without a component tag
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const noComponentContent = `@phase1 @validation
Feature: Broken Feature

  Scenario: Test
    Given something`;

      await writeFile(join(featuresDir, 'broken.feature'), noComponentContent);

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/broken.feature',
        cwd: testDir,
      });

      // Then it should report missing component tag
      expect(result.results[0].valid).toBe(false);
      const componentError = result.results[0].errors.find(e =>
        e.message.includes('component tag')
      );
      expect(componentError).toBeDefined();
    });
  });

  describe('Scenario: Detect missing required feature-group tag', () => {
    it('should report missing feature-group tag', async () => {
      // Given I have a feature file without a feature-group tag
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const noFeatureGroupContent = `@phase1 @cli
Feature: Broken Feature

  Scenario: Test
    Given something`;

      await writeFile(
        join(featuresDir, 'broken.feature'),
        noFeatureGroupContent
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/broken.feature',
        cwd: testDir,
      });

      // Then it should report missing feature-group tag
      expect(result.results[0].valid).toBe(false);
      const featureGroupError = result.results[0].errors.find(e =>
        e.message.includes('feature-group tag')
      );
      expect(featureGroupError).toBeDefined();
    });
  });

  describe('Scenario: Handle missing tags.json file', () => {
    it('should error when tags.json does not exist', async () => {
      // Given no tags.json exists
      await rm(join(testDir, 'spec', 'tags.json'));

      // When I run validate-tags
      // Then it should throw an error
      await expect(validateTags({ cwd: testDir })).rejects.toThrow(
        'tags.json not found'
      );
    });
  });

  describe('Scenario: Validate tags after creating new feature', () => {
    it('should warn about placeholder tags', async () => {
      // Given I have a feature with placeholder tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const placeholderContent = `@phase1 @component @feature-group
Feature: New Feature

  Scenario: Test
    Given something`;

      await writeFile(
        join(featuresDir, 'new-feature.feature'),
        placeholderContent
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/new-feature.feature',
        cwd: testDir,
      });

      // Then it should warn about placeholders
      expect(result.results[0].valid).toBe(false);
      const placeholderErrors = result.results[0].errors.filter(e =>
        e.message.includes('Placeholder')
      );
      expect(placeholderErrors.length).toBeGreaterThan(0);
      expect(placeholderErrors[0].suggestion).toContain('Replace');
    });
  });

  describe('Scenario: Report multiple violations in one file', () => {
    it('should list all unregistered tags in one file', async () => {
      // Given I have a feature with multiple unregistered tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const multipleErrorsContent = `@phase1 @unknown1 @unknown2
Feature: Multi Error Feature

  Scenario: Test
    Given something`;

      await writeFile(
        join(featuresDir, 'multi.feature'),
        multipleErrorsContent
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/multi.feature',
        cwd: testDir,
      });

      // Then it should list both unregistered tags
      expect(result.results[0].valid).toBe(false);
      const unregisteredErrors = result.results[0].errors.filter(e =>
        e.message.includes('Unregistered')
      );
      expect(unregisteredErrors.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Scenario: Validate tags in multiple files with summary', () => {
    it('should show summary with pass/fail counts', async () => {
      // Given I have 10 feature files (8 valid, 2 invalid)
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const validFeature =
        '@phase1 @cli @authentication\nFeature: Valid\n  Scenario: Test\n    Given test';
      const invalidFeature =
        '@phase1 @cli @validation @invalid-tag\nFeature: Invalid\n  Scenario: Test\n    Given test';

      for (let i = 1; i <= 8; i++) {
        await writeFile(join(featuresDir, `valid${i}.feature`), validFeature);
      }

      for (let i = 1; i <= 2; i++) {
        await writeFile(
          join(featuresDir, `invalid${i}.feature`),
          invalidFeature
        );
      }

      // When I run validate-tags
      const result = await validateTags({ cwd: testDir });

      // Then the summary should show 8 passed, 2 failed
      expect(result.validCount).toBe(8);
      expect(result.invalidCount).toBe(2);
      expect(result.results).toHaveLength(10);
    });
  });

  describe('Scenario: CAGE integration - prevent invalid tag commits', () => {
    it('should support CAGE workflow for tag validation', async () => {
      // Given I am working in a CAGE-monitored project
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const unregisteredContent = `@phase1 @cli @validation @unregistered-tag
Feature: Test Feature

  Scenario: Test
    Given something`;

      await writeFile(join(featuresDir, 'test.feature'), unregisteredContent);

      // When a PreToolUse hook runs validate-tags
      const result = await validateTags({ cwd: testDir });

      // And unregistered tags are detected
      expect(result.invalidCount).toBeGreaterThan(0);

      // Then the hook can warn the AI agent
      expect(result.results[0].valid).toBe(false);

      // And the AI agent can fix tags before proceeding
      const fixedContent = `@phase1 @cli @authentication
Feature: Test Feature

  Scenario: Test
    Given something`;

      await writeFile(join(featuresDir, 'test.feature'), fixedContent);

      const fixedResult = await validateTags({ cwd: testDir });

      // And specifications remain compliant with tag registry
      expect(fixedResult.invalidCount).toBe(0);
      expect(fixedResult.validCount).toBe(1);
    });
  });

  describe('Scenario: Validate scenario-level tags are registered', () => {
    it('should validate scenario-level tags against registry', async () => {
      // Given I have a feature file with scenario-level tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureWithScenarioTags = `@phase1
@cli
@authentication
Feature: User Login

  @smoke
  @regression
  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

      await writeFile(
        join(featuresDir, 'login.feature'),
        featureWithScenarioTags
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/login.feature',
        cwd: testDir,
      });

      // Then it should exit with code 0 (all tags registered)
      expect(result.results[0].valid).toBe(true);
      expect(result.results[0].errors).toHaveLength(0);
      expect(result.validCount).toBe(1);
      expect(result.invalidCount).toBe(0);
    });
  });

  describe('Scenario: Detect unregistered scenario-level tag', () => {
    it('should report unregistered scenario-level tags', async () => {
      // Given I have a feature file with an unregistered scenario tag
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureWithUnregisteredTag = `@phase1
@cli
@authentication
Feature: User Login

  @smoke
  @unregistered-scenario-tag
  Scenario: Test scenario
    Given a step
    When another step
    Then result`;

      await writeFile(
        join(featuresDir, 'login.feature'),
        featureWithUnregisteredTag
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/login.feature',
        cwd: testDir,
      });

      // Then it should exit with code 1
      expect(result.results[0].valid).toBe(false);

      // And the output should contain the unregistered tag
      const unregisteredError = result.results[0].errors.find(e =>
        e.tag === '@unregistered-scenario-tag'
      );
      expect(unregisteredError).toBeDefined();
      expect(unregisteredError!.message).toContain('Unregistered tag: @unregistered-scenario-tag');
    });
  });

  describe('Scenario: Validate both feature-level and scenario-level tags', () => {
    it('should validate tags at both levels', async () => {
      // Given I have a feature file with tags at both levels
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureWithBothLevels = `@phase1
@cli
@authentication
Feature: User Login

  @smoke
  Scenario: Basic login
    Given I am on the login page
    When I enter credentials
    Then I am logged in

  @regression
  @edge-case
  Scenario: Login with expired session
    Given I have an expired session
    When I attempt to login
    Then I am prompted to re-authenticate`;

      await writeFile(
        join(featuresDir, 'login.feature'),
        featureWithBothLevels
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/login.feature',
        cwd: testDir,
      });

      // Then all tags at all levels should be validated
      expect(result.results[0].valid).toBe(true);
      expect(result.results[0].errors).toHaveLength(0);
      expect(result.validCount).toBe(1);
      expect(result.invalidCount).toBe(0);
    });
  });

  describe('Scenario: Detect mix of registered and unregistered scenario tags', () => {
    it('should report mix of valid and invalid scenario tags', async () => {
      // Given I have a feature file with multiple scenarios
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureWithMixedTags = `@phase1
@cli
@authentication
Feature: User Login

  @smoke
  Scenario: Valid scenario
    Given a step
    When another step
    Then result

  @unregistered-tag1
  @unregistered-tag2
  Scenario: Invalid scenario
    Given a step
    When another step
    Then result`;

      await writeFile(
        join(featuresDir, 'login.feature'),
        featureWithMixedTags
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/login.feature',
        cwd: testDir,
      });

      // Then it should exit with code 1
      expect(result.results[0].valid).toBe(false);

      // And the output should list both unregistered tags
      const unregisteredErrors = result.results[0].errors.filter(e =>
        e.tag === '@unregistered-tag1' || e.tag === '@unregistered-tag2'
      );
      expect(unregisteredErrors).toHaveLength(2);

      // And the output should specify the tags are unregistered
      expect(unregisteredErrors[0].message).toContain('Unregistered tag');
      expect(unregisteredErrors[1].message).toContain('Unregistered tag');
    });
  });

  describe('Scenario: Validate scenario tags do not require phase/component/feature-group tags', () => {
    it('should allow scenario tags without required categories', async () => {
      // Given I have a feature file with properly tagged feature and minimal scenario tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureWithMinimalScenarioTags = `@phase1
@cli
@authentication
Feature: User Login

  @smoke
  Scenario: Quick test
    Given a step
    When another step
    Then result`;

      await writeFile(
        join(featuresDir, 'login.feature'),
        featureWithMinimalScenarioTags
      );

      // When I run validate-tags
      const result = await validateTags({
        file: 'spec/features/login.feature',
        cwd: testDir,
      });

      // Then scenario tags should not be required to have phase/component/feature-group tags
      // And only feature-level tags should be checked for required categories
      expect(result.results[0].valid).toBe(true);
      expect(result.results[0].errors).toHaveLength(0);
      expect(result.validCount).toBe(1);
      expect(result.invalidCount).toBe(0);
    });
  });

  describe('Scenario: JSON-backed workflow - validate against tags.json registry', () => {
    it('should load tag registry from tags.json and validate all tags', async () => {
      // Given I have tags.json with registered tags in multiple categories
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // And I have feature files using both registered and unregistered tags
      await writeFile(
        join(featuresDir, 'valid.feature'),
        '@phase1 @cli @authentication\nFeature: Valid\n  Scenario: Test\n    Given test'
      );

      await writeFile(
        join(featuresDir, 'invalid.feature'),
        '@phase1 @cli @validation @unregistered-tag\nFeature: Invalid\n  Scenario: Test\n    Given test'
      );

      // When I run `fspec validate-tags`
      const result = await validateTags({ cwd: testDir });

      // Then the command should load tag registry from spec/tags.json
      // (verified by not throwing "tags.json not found" error)
      expect(result).toBeDefined();

      // And validate all tags against the JSON registry
      expect(result.results).toHaveLength(2);

      // And report unregistered tags with file locations
      const invalidResult = result.results.find(r =>
        r.file === 'spec/features/invalid.feature'
      );
      expect(invalidResult).toBeDefined();
      expect(invalidResult!.valid).toBe(false);
      expect(invalidResult!.errors).toHaveLength(1);
      expect(invalidResult!.errors[0].tag).toBe('@unregistered-tag');

      // And check for required tag categories (phase, component, feature-group)
      // (both files have phase and component tags, so they pass those checks)
      const validResult = result.results.find(r =>
        r.file === 'spec/features/valid.feature'
      );
      expect(validResult).toBeDefined();
      expect(validResult!.valid).toBe(true);

      // And the command should exit with code 1 if violations found
      expect(result.invalidCount).toBe(1);
      expect(result.validCount).toBe(1);
    });
  });
});
