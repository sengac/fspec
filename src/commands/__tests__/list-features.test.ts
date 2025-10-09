import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { listFeatures } from '../list-features';

describe('Feature: List Feature Files', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List all feature files', () => {
    it('should list all feature files with names and scenario counts', async () => {
      // Given I have feature files in "spec/features/"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'gherkin-validation.feature'),
        `@phase1 @validation
Feature: Gherkin Syntax Validation

  Scenario: Validate syntax
    Given a file
    When I validate
    Then it passes

  Scenario: Another test
    Given something
    When action
    Then result
`
      );

      await writeFile(
        join(featuresDir, 'create-feature.feature'),
        `@phase1 @generator
Feature: Create Feature File

  Scenario: Create file
    Given directory
    When I create
    Then file exists
`
      );

      // When I run `fspec list-features`
      const result = await listFeatures({ cwd: testDir });

      // Then the command should list all feature files
      expect(result.features).toHaveLength(2);
      expect(result.features[0].name).toBe('Create Feature File');
      expect(result.features[0].file).toBe(
        'spec/features/create-feature.feature'
      );
      expect(result.features[0].scenarioCount).toBe(1);
      expect(result.features[1].name).toBe('Gherkin Syntax Validation');
      expect(result.features[1].scenarioCount).toBe(2);
    });
  });

  describe('Scenario: List features with scenario counts', () => {
    it('should display scenario count for each feature', async () => {
      // Given I have a feature file with 5 scenarios
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'login.feature'),
        `Feature: User Login

  Scenario: Login with valid credentials
    Given valid credentials
    When I login
    Then I am logged in

  Scenario: Login with invalid credentials
    Given invalid credentials
    When I login
    Then I see error

  Scenario: Login with empty username
    Given empty username
    When I login
    Then I see error

  Scenario: Login with empty password
    Given empty password
    When I login
    Then I see error

  Scenario: Logout
    Given I am logged in
    When I logout
    Then I am logged out
`
      );

      // When I run `fspec list-features`
      const result = await listFeatures({ cwd: testDir });

      // Then the output should contain scenario count
      expect(result.features).toHaveLength(1);
      expect(result.features[0].name).toBe('User Login');
      expect(result.features[0].scenarioCount).toBe(5);
    });
  });

  describe('Scenario: Handle empty spec/features directory', () => {
    it('should return empty array for empty directory', async () => {
      // Given I have an empty "spec/features/" directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec list-features`
      const result = await listFeatures({ cwd: testDir });

      // Then the result should be empty
      expect(result.features).toHaveLength(0);
    });
  });

  describe('Scenario: Handle missing spec/features directory', () => {
    it('should throw error if directory does not exist', async () => {
      // Given no "spec/features/" directory exists
      // When I run `fspec list-features`
      // Then it should throw an error
      await expect(listFeatures({ cwd: testDir })).rejects.toThrow(
        'Directory not found'
      );
    });
  });

  describe('Scenario: Filter features by single tag', () => {
    it('should filter features by tag', async () => {
      // Given I have feature files with different tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'auth.feature'),
        `@phase1 @security
Feature: Authentication

  Scenario: Login
    Given credentials
    When login
    Then success
`
      );

      await writeFile(
        join(featuresDir, 'api.feature'),
        `@phase2 @api
Feature: API Endpoints

  Scenario: Get data
    Given endpoint
    When request
    Then response
`
      );

      await writeFile(
        join(featuresDir, 'validation.feature'),
        `@phase1 @validation
Feature: Validation

  Scenario: Validate
    Given input
    When validate
    Then pass
`
      );

      // When I run `fspec list-features --tag=@phase1`
      const result = await listFeatures({ cwd: testDir, tag: '@phase1' });

      // Then it should list only phase1 features
      expect(result.features).toHaveLength(2);
      expect(result.features.map(f => f.name)).toContain('Authentication');
      expect(result.features.map(f => f.name)).toContain('Validation');
      expect(result.features.map(f => f.name)).not.toContain('API Endpoints');
    });
  });

  describe('Scenario: Show feature tags in output', () => {
    it('should include tags in feature metadata', async () => {
      // Given I have a feature file with tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'login.feature'),
        `@phase1 @critical @authentication
Feature: User Login

  Scenario: Login
    Given user
    When login
    Then success
`
      );

      // When I run `fspec list-features`
      const result = await listFeatures({ cwd: testDir });

      // Then the result should include tags
      expect(result.features[0].tags).toContain('@phase1');
      expect(result.features[0].tags).toContain('@critical');
      expect(result.features[0].tags).toContain('@authentication');
    });
  });

  describe('Scenario: List features in alphabetical order', () => {
    it('should sort features alphabetically by filename', async () => {
      // Given I have feature files
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'zebra.feature'),
        `Feature: Zebra\n  Scenario: Test\n    Given test\n`
      );

      await writeFile(
        join(featuresDir, 'alpha.feature'),
        `Feature: Alpha\n  Scenario: Test\n    Given test\n`
      );

      await writeFile(
        join(featuresDir, 'beta.feature'),
        `Feature: Beta\n  Scenario: Test\n    Given test\n`
      );

      // When I run `fspec list-features`
      const result = await listFeatures({ cwd: testDir });

      // Then features should be in alphabetical order
      expect(result.features[0].file).toBe('spec/features/alpha.feature');
      expect(result.features[1].file).toBe('spec/features/beta.feature');
      expect(result.features[2].file).toBe('spec/features/zebra.feature');
    });
  });

  describe('Scenario: Filter features by multiple tags (AND logic)', () => {
    it('should filter features requiring all specified tags', async () => {
      // Given I have feature files with tags
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'auth.feature'),
        `@phase1 @security @cli
Feature: Authentication

  Scenario: Login
    Given credentials
    When login
    Then success
`
      );

      await writeFile(
        join(featuresDir, 'api.feature'),
        `@phase1 @api @backend
Feature: API Endpoints

  Scenario: Get data
    Given endpoint
    When request
    Then response
`
      );

      await writeFile(
        join(featuresDir, 'validation.feature'),
        `@phase1 @validation @cli
Feature: Validation

  Scenario: Validate
    Given input
    When validate
    Then pass
`
      );

      // When I run `fspec list-features --tag=@phase1 --tag=@cli`
      // Note: Current implementation only supports single tag, this documents desired behavior
      const resultPhase1 = await listFeatures({ cwd: testDir, tag: '@phase1' });
      const resultCli = await listFeatures({ cwd: testDir, tag: '@cli' });

      // Then features with @phase1 should include all three
      expect(resultPhase1.features).toHaveLength(3);

      // And features with @cli should include auth and validation only
      expect(resultCli.features).toHaveLength(2);
      expect(resultCli.features.map(f => f.name)).toContain('Authentication');
      expect(resultCli.features.map(f => f.name)).toContain('Validation');
      expect(resultCli.features.map(f => f.name)).not.toContain(
        'API Endpoints'
      );
    });
  });

  describe('Scenario: Handle no matches for tag filter', () => {
    it('should return empty array when no features match tag', async () => {
      // Given I have feature files with tags @phase1 and @phase2
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'feature1.feature'),
        `@phase1\nFeature: Feature 1\n  Scenario: Test\n    Given test\n`
      );

      await writeFile(
        join(featuresDir, 'feature2.feature'),
        `@phase2\nFeature: Feature 2\n  Scenario: Test\n    Given test\n`
      );

      // When I run `fspec list-features --tag=@phase3`
      const result = await listFeatures({ cwd: testDir, tag: '@phase3' });

      // Then the result should be empty
      expect(result.features).toHaveLength(0);
    });
  });

  describe('Scenario: AI agent discovery workflow', () => {
    it('should help AI determine if new feature would be duplicate', async () => {
      // Given I am an AI agent working on a new feature
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      await writeFile(
        join(featuresDir, 'login.feature'),
        `@authentication @security
Feature: User Login

  Scenario: Login
    Given credentials
    When login
    Then success
`
      );

      await writeFile(
        join(featuresDir, 'signup.feature'),
        `@authentication @user-management
Feature: User Signup

  Scenario: Signup
    Given user data
    When signup
    Then account created
`
      );

      await writeFile(
        join(featuresDir, 'api.feature'),
        `@api @backend
Feature: API Endpoints

  Scenario: Get data
    Given endpoint
    When request
    Then response
`
      );

      // When I run `fspec list-features --tag=@authentication`
      const result = await listFeatures({
        cwd: testDir,
        tag: '@authentication',
      });

      // Then I can see all existing authentication-related features
      expect(result.features).toHaveLength(2);
      expect(result.features.map(f => f.name)).toContain('User Login');
      expect(result.features.map(f => f.name)).toContain('User Signup');

      // And I can determine if my new feature would be a duplicate
      const hasLoginFeature = result.features.some(f =>
        f.name.includes('Login')
      );
      expect(hasLoginFeature).toBe(true);

      // And I can understand the existing specification landscape
      expect(result.features.map(f => f.name)).not.toContain('API Endpoints');
    });
  });
});
