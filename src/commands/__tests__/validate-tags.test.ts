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

    // Create TAGS.md with standard tags
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });

    const tagsContent = `# Tag Registry

## Phase Tags
| Tag | Description |
|-----|-------------|
| \`@phase1\` | Phase 1 |
| \`@phase2\` | Phase 2 |

## Component Tags
| Tag | Description |
|-----|-------------|
| \`@cli\` | CLI |
| \`@parser\` | Parser |

## Feature Group Tags
| Tag | Description |
|-----|-------------|
| \`@feature-management\` | Feature Management |
| \`@tag-management\` | Tag Management |
| \`@validation\` | Validation |
| \`@authentication\` | Authentication |
`;

    await writeFile(join(specDir, 'TAGS.md'), tagsContent);
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
      expect(result.results[0].errors[0].suggestion).toContain('TAGS.md');
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

  describe('Scenario: Handle missing TAGS.md file', () => {
    it('should error when TAGS.md does not exist', async () => {
      // Given no TAGS.md exists
      await rm(join(testDir, 'spec', 'TAGS.md'));

      // When I run validate-tags
      // Then it should throw an error
      await expect(validateTags({ cwd: testDir })).rejects.toThrow(
        'TAGS.md not found'
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
});
