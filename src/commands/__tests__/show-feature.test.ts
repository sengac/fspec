import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { showFeature } from '../show-feature';

describe('Feature: Display Feature File Contents', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-show-feature');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show feature file contents in text format', () => {
    it('should display feature file contents', async () => {
      // Given I have a feature file "login.feature" with valid Gherkin
      const content = `@auth
Feature: User Login

  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec show-feature login`
      const result = await showFeature({
        feature: 'login',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should contain the feature file contents
      expect(result.content).toContain('Feature: User Login');
      expect(result.content).toContain('Scenario: Successful login');

      // And the output should be in plain text format
      expect(result.format).toBe('text');
    });
  });

  describe('Scenario: Show feature file by full path', () => {
    it('should accept full path', async () => {
      // Given I have a feature file "spec/features/checkout.feature"
      const content = `Feature: Checkout Process
  Scenario: Complete checkout
    Given I have items in cart
    When I checkout
    Then order is placed`;

      await writeFile(join(testDir, 'spec/features/checkout.feature'), content);

      // When I run `fspec show-feature spec/features/checkout.feature`
      const result = await showFeature({
        feature: 'spec/features/checkout.feature',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should contain the feature file contents
      expect(result.content).toContain('Feature: Checkout Process');
    });
  });

  describe('Scenario: Show feature file in JSON format', () => {
    it('should output JSON format', async () => {
      // Given I have a feature file "api.feature" with Feature, Background, and 2 Scenarios
      const content = `Feature: API Operations

  Background: User Story
    As a developer
    I want to use the API
    So that I can integrate

  Scenario: GET request
    Given I have an endpoint
    When I send GET
    Then I receive data

  Scenario: POST request
    Given I have data
    When I send POST
    Then data is created`;

      await writeFile(join(testDir, 'spec/features/api.feature'), content);

      // When I run `fspec show-feature api --format=json`
      const result = await showFeature({
        feature: 'api',
        format: 'json',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should be valid JSON
      const parsed = JSON.parse(result.content!);
      expect(parsed).toBeDefined();

      // And the JSON should contain the feature name
      expect(parsed.feature.name).toBe('API Operations');

      // And the JSON should contain the background section
      expect(parsed.feature.children).toBeDefined();
      const background = parsed.feature.children.find((child: any) => child.background);
      expect(background).toBeDefined();

      // And the JSON should contain 2 scenarios
      const scenarios = parsed.feature.children.filter((child: any) => child.scenario);
      expect(scenarios.length).toBe(2);
    });
  });

  describe('Scenario: Write feature contents to output file', () => {
    it('should write to output file', async () => {
      // Given I have a feature file "payment.feature"
      const content = `Feature: Payment Processing
  Scenario: Process payment
    Given I have payment info
    When I submit
    Then payment processed`;

      await writeFile(join(testDir, 'spec/features/payment.feature'), content);

      // When I run `fspec show-feature payment --output=feature-copy.txt`
      const result = await showFeature({
        feature: 'payment',
        output: join(testDir, 'feature-copy.txt'),
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file "feature-copy.txt" should exist
      await expect(access(join(testDir, 'feature-copy.txt'))).resolves.toBeUndefined();

      // And "feature-copy.txt" should contain the feature file contents
      const outputContent = await readFile(join(testDir, 'feature-copy.txt'), 'utf-8');
      expect(outputContent).toContain('Feature: Payment Processing');
    });
  });

  describe('Scenario: Show feature with architecture doc string', () => {
    it('should include doc string in output', async () => {
      // Given I have a feature file "auth.feature" with architecture doc string
      const content = `Feature: Authentication
  """
  Architecture notes:
  - Uses OAuth 2.0
  """

  Scenario: Login
    Given I enter credentials
    When I submit
    Then authenticated`;

      await writeFile(join(testDir, 'spec/features/auth.feature'), content);

      // When I run `fspec show-feature auth`
      const result = await showFeature({
        feature: 'auth',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should contain the architecture doc string
      expect(result.content).toContain('Architecture notes:');
      expect(result.content).toContain('- Uses OAuth 2.0');
    });
  });

  describe('Scenario: Show feature with tags', () => {
    it('should include tags in output', async () => {
      // Given I have a feature file "search.feature" with tags "@api @critical"
      const content = `@api @critical
Feature: Search Functionality

  Scenario: Basic search
    Given I enter search term
    When I search
    Then results displayed`;

      await writeFile(join(testDir, 'spec/features/search.feature'), content);

      // When I run `fspec show-feature search`
      const result = await showFeature({
        feature: 'search',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should contain "@api @critical"
      expect(result.content).toContain('@api @critical');
    });
  });

  describe('Scenario: Reject non-existent feature file', () => {
    it('should return error for missing file', async () => {
      // Given I have no feature file named "missing.feature"
      // When I run `fspec show-feature missing`
      const result = await showFeature({
        feature: 'missing',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Feature file not found"
      expect(result.error).toMatch(/feature file not found/i);
    });
  });

  describe('Scenario: Accept feature name without .feature extension', () => {
    it('should find file by name', async () => {
      // Given I have a feature file "spec/features/dashboard.feature"
      const content = `Feature: Dashboard
  Scenario: View dashboard
    Given I am logged in
    When I view dashboard
    Then I see overview`;

      await writeFile(join(testDir, 'spec/features/dashboard.feature'), content);

      // When I run `fspec show-feature dashboard`
      const result = await showFeature({
        feature: 'dashboard',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should contain the dashboard feature contents
      expect(result.content).toContain('Feature: Dashboard');
    });
  });

  describe('Scenario: Show feature with scenario-level tags', () => {
    it('should include scenario tags', async () => {
      // Given I have a feature file "notifications.feature" with scenarios tagged "@email @sms"
      const content = `Feature: Notifications

  @email @sms
  Scenario: Send notification
    Given I have a message
    When I send notification
    Then user receives it`;

      await writeFile(join(testDir, 'spec/features/notifications.feature'), content);

      // When I run `fspec show-feature notifications`
      const result = await showFeature({
        feature: 'notifications',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should contain "@email @sms"
      expect(result.content).toContain('@email @sms');
    });
  });

  describe('Scenario: JSON format includes all Gherkin elements', () => {
    it('should include all elements in JSON', async () => {
      // Given I have a feature file "integration.feature" with tags, doc string, Background, and scenarios
      const content = `@integration @api
Feature: System Integration
  """
  Integration architecture
  """

  Background: User Story
    As a developer
    I want integration
    So that systems connect

  Scenario: Connect systems
    Given system A
    When connects to B
    Then integrated`;

      await writeFile(join(testDir, 'spec/features/integration.feature'), content);

      // When I run `fspec show-feature integration --format=json`
      const result = await showFeature({
        feature: 'integration',
        format: 'json',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const parsed = JSON.parse(result.content!);

      // And the JSON should contain feature tags
      expect(parsed.feature.tags).toBeDefined();
      expect(parsed.feature.tags.length).toBeGreaterThan(0);

      // And the JSON should contain the architecture doc string
      expect(parsed.feature.description).toContain('Integration architecture');

      // And the JSON should contain the Background section
      const background = parsed.feature.children.find((child: any) => child.background);
      expect(background).toBeDefined();

      // And the JSON should contain all scenarios
      const scenarios = parsed.feature.children.filter((child: any) => child.scenario);
      expect(scenarios.length).toBeGreaterThan(0);

      // And the JSON should contain all steps
      expect(scenarios[0].scenario.steps.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Write JSON output to file', () => {
    it('should write JSON to file', async () => {
      // Given I have a feature file "reporting.feature"
      const content = `Feature: Reporting
  Scenario: Generate report
    Given I have data
    When I generate report
    Then report created`;

      await writeFile(join(testDir, 'spec/features/reporting.feature'), content);

      // When I run `fspec show-feature reporting --format=json --output=report.json`
      const result = await showFeature({
        feature: 'reporting',
        format: 'json',
        output: join(testDir, 'report.json'),
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file "report.json" should exist
      await expect(access(join(testDir, 'report.json'))).resolves.toBeUndefined();

      // And "report.json" should contain valid JSON
      const outputContent = await readFile(join(testDir, 'report.json'), 'utf-8');
      const parsed = JSON.parse(outputContent);
      expect(parsed).toBeDefined();
    });
  });

  describe('Scenario: Validate feature file Gherkin syntax', () => {
    it('should validate Gherkin syntax', async () => {
      // Given I have a feature file "orders.feature" with valid Gherkin
      const content = `Feature: Order Management
  Scenario: Create order
    Given I have items
    When I create order
    Then order created`;

      await writeFile(join(testDir, 'spec/features/orders.feature'), content);

      // When I run `fspec show-feature orders`
      const result = await showFeature({
        feature: 'orders',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature file syntax should be validated
      expect(result.validated).toBe(true);
    });
  });

  describe('Scenario: Show error for invalid Gherkin syntax', () => {
    it('should return error for invalid syntax', async () => {
      // Given I have a feature file "broken.feature" with invalid Gherkin syntax
      const content = `This is not valid Gherkin
Feature: Broken Feature
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/broken.feature'), content);

      // When I run `fspec show-feature broken`
      const result = await showFeature({
        feature: 'broken',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Invalid Gherkin syntax"
      expect(result.error).toMatch(/invalid gherkin syntax/i);
    });
  });
});
