import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { createFeature } from '../create-feature';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

describe('Feature: Create Feature File with Template', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create feature file with valid name', () => {
    it('should create a valid Gherkin feature file', async () => {
      // Given I am in a project with a spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "User Authentication"`
      await createFeature('User Authentication', testDir);

      // Then a file "spec/features/user-authentication.feature" should be created
      const featureFile = join(featuresDir, 'user-authentication.feature');
      const content = await readFile(featureFile, 'utf-8');

      // And the file should be valid parseable Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      expect(() => {
        gherkinDocument = parser.parse(content);
      }).not.toThrow();

      // And the parsed feature should have correct name
      expect(gherkinDocument.feature).toBeDefined();
      expect(gherkinDocument.feature!.name).toBe('User Authentication');

      // And the file should have required tags
      const tags = gherkinDocument.feature!.tags.map(t => t.name);
      expect(tags).toContain('@phase1');
      expect(tags).toContain('@component');
      expect(tags).toContain('@feature-group');

      // And it should have a Background section
      const backgrounds = gherkinDocument.feature!.children.filter(
        c => c.background
      );
      expect(backgrounds).toHaveLength(1);
      expect(backgrounds[0].background!.name).toBe('User Story');

      // And it should have a Scenario section
      const scenarios = gherkinDocument.feature!.children.filter(
        c => c.scenario
      );
      expect(scenarios).toHaveLength(1);

      // And the scenario should have Given/When/Then steps
      const steps = scenarios[0].scenario!.steps;
      expect(steps).toHaveLength(3);
      expect(steps[0].keyword.trim()).toBe('Given');
      expect(steps[1].keyword.trim()).toBe('When');
      expect(steps[2].keyword.trim()).toBe('Then');
    });
  });

  describe('Scenario: Convert feature name to kebab-case', () => {
    it('should convert name to kebab-case for filename', async () => {
      // Given I am in a project with a spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "Real Time Event Monitoring"`
      await createFeature('Real Time Event Monitoring', testDir);

      // Then a file "spec/features/real-time-event-monitoring.feature" should be created
      const featureFile = join(
        featuresDir,
        'real-time-event-monitoring.feature'
      );
      await access(featureFile); // Throws if file doesn't exist

      // And the file should contain "Feature: Real Time Event Monitoring"
      const content = await readFile(featureFile, 'utf-8');
      expect(content).toContain('Feature: Real Time Event Monitoring');
    });
  });

  describe("Scenario: Create spec/features/ directory if it doesn't exist", () => {
    it('should create directory structure automatically', async () => {
      // Given I am in a project without a spec/features/ directory
      // When I run `fspec create-feature "New Feature"`
      await createFeature('New Feature', testDir);

      // Then the directory "spec/features/" should be created
      const featuresDir = join(testDir, 'spec', 'features');
      await access(featuresDir);

      // And a file "spec/features/new-feature.feature" should be created
      const featureFile = join(featuresDir, 'new-feature.feature');
      await access(featureFile);
    });
  });

  describe('Scenario: Prevent overwriting existing file', () => {
    it('should error if file already exists', async () => {
      // Given I have an existing file "spec/features/user-login.feature"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });
      const existingFile = join(featuresDir, 'user-login.feature');
      const originalContent = 'existing content';
      await writeFile(existingFile, originalContent);

      // When I run `fspec create-feature "User Login"`
      // Then the command should exit with code 1
      await expect(createFeature('User Login', testDir)).rejects.toThrow(
        'already exists'
      );

      // And the existing file should not be modified
      const content = await readFile(existingFile, 'utf-8');
      expect(content).toBe(originalContent);
    });
  });

  describe('Scenario: Handle special characters in feature name', () => {
    it('should sanitize special characters for filename', async () => {
      // Given I am in a project with a spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "API/REST Endpoints"`
      await createFeature('API/REST Endpoints', testDir);

      // Then a file "spec/features/api-rest-endpoints.feature" should be created
      const featureFile = join(featuresDir, 'api-rest-endpoints.feature');
      await access(featureFile);

      // And the file should contain "Feature: API/REST Endpoints"
      const content = await readFile(featureFile, 'utf-8');
      expect(content).toContain('Feature: API/REST Endpoints');
    });
  });

  describe('Scenario: Create feature with minimal name', () => {
    it('should create feature file with single word name', async () => {
      // Given I am in a project with a spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "Login"`
      await createFeature('Login', testDir);

      // Then a file "spec/features/login.feature" should be created
      const featureFile = join(featuresDir, 'login.feature');
      await access(featureFile);

      // And the file should contain "Feature: Login"
      const content = await readFile(featureFile, 'utf-8');
      expect(content).toContain('Feature: Login');
    });
  });

  describe('Scenario: Show success message with file path', () => {
    it('should return file path for verification', async () => {
      // Given I am in a project with a spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "User Permissions"`
      const result = await createFeature('User Permissions', testDir);

      // Then the command should return the file path
      expect(result.filePath).toContain(
        'spec/features/user-permissions.feature'
      );

      // And the file should exist at that path
      const content = await readFile(result.filePath, 'utf-8');
      expect(content).toContain('Feature: User Permissions');
    });
  });

  describe('Scenario: Created file has proper template structure', () => {
    it('should include all required template sections', async () => {
      // Given I am in a project with a spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "Data Export"`
      await createFeature('Data Export', testDir);

      // Then the file should have proper Gherkin structure
      const featureFile = join(featuresDir, 'data-export.feature');
      const content = await readFile(featureFile, 'utf-8');

      // And the file should have tag placeholders
      expect(content).toContain('@phase1');
      expect(content).toContain('@component');
      expect(content).toContain('@feature-group');

      // And the file should have architecture notes section
      expect(content).toContain('Architecture notes:');
      expect(content).toContain('TODO: Add key architectural decisions');

      // And the file should have user story template in Background
      expect(content).toContain('Background: User Story');
      expect(content).toContain('As a [role]');
      expect(content).toContain('I want to [action]');
      expect(content).toContain('So that [benefit]');

      // And the file should have example scenario with steps
      expect(content).toContain('Scenario: [Scenario name]');
      expect(content).toContain('Given [precondition]');
      expect(content).toContain('When [action]');
      expect(content).toContain('Then [expected outcome]');
    });
  });

  describe('Scenario: AI agent workflow - create and validate', () => {
    it('should create file that passes validation immediately', async () => {
      // Given I am an AI agent creating a new specification
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "Shopping Cart"`
      const result = await createFeature('Shopping Cart', testDir);

      // Then a file should be created
      expect(result.filePath).toContain('spec/features/shopping-cart.feature');

      // And when I validate it, it should pass
      const content = await readFile(result.filePath, 'utf-8');
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      expect(() => {
        gherkinDocument = parser.parse(content);
      }).not.toThrow();

      // And I can immediately edit the file to add real scenarios
      expect(gherkinDocument.feature).toBeDefined();
      expect(gherkinDocument.feature!.name).toBe('Shopping Cart');
    });
  });
});
