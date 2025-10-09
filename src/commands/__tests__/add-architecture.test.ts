import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { addArchitecture } from '../add-architecture';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

describe('Feature: Add Architecture Documentation to Feature Files', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-add-architecture');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add architecture notes to feature without existing doc string', () => {
    it('should add doc string after Feature line', async () => {
      // Given I have a feature file "login.feature" with no doc string
      const content = `@auth
Feature: User Login
  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec add-architecture login "Uses JWT for authentication"`
      const result = await addArchitecture({
        feature: 'login',
        text: 'Uses JWT for authentication',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );

      // And the feature file should contain a doc string with "Uses JWT for authentication"
      expect(updatedContent).toContain('"""');
      expect(updatedContent).toContain('Uses JWT for authentication');

      // And the doc string should be after the Feature line
      const featureIndex = updatedContent.indexOf('Feature: User Login');
      const docStringIndex = updatedContent.indexOf('"""');
      expect(docStringIndex).toBeGreaterThan(featureIndex);

      // And the doc string should be before the Background section
      const backgroundIndex = updatedContent.indexOf('Background: User Story');
      expect(docStringIndex).toBeLessThan(backgroundIndex);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add multi-line architecture notes', () => {
    it('should add multi-line doc string', async () => {
      // Given I have a feature file "api.feature" with no doc string
      const content = `Feature: API Operations
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/api.feature'), content);

      // When I run `fspec add-architecture api "Architecture notes:\n- Uses REST API\n- Requires authentication"`
      const result = await addArchitecture({
        feature: 'api',
        text: 'Architecture notes:\n- Uses REST API\n- Requires authentication',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/api.feature'),
        'utf-8'
      );

      // And the doc string should contain "Architecture notes:"
      expect(updatedContent).toContain('Architecture notes:');

      // And the doc string should contain "- Uses REST API"
      expect(updatedContent).toContain('- Uses REST API');

      // And the doc string should contain "- Requires authentication"
      expect(updatedContent).toContain('- Requires authentication');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Replace existing architecture doc string', () => {
    it('should replace existing doc string', async () => {
      // Given I have a feature file "payment.feature" with existing doc string "Old notes"
      const content = `Feature: Payment Processing
  """
  Old notes
  """

  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/payment.feature'), content);

      // When I run `fspec add-architecture payment "New architecture notes"`
      const result = await addArchitecture({
        feature: 'payment',
        text: 'New architecture notes',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/payment.feature'),
        'utf-8'
      );

      // And the doc string should contain "New architecture notes"
      expect(updatedContent).toContain('New architecture notes');

      // And the doc string should not contain "Old notes"
      expect(updatedContent).not.toContain('Old notes');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add architecture notes preserves scenarios', () => {
    it('should preserve all scenarios', async () => {
      // Given I have a feature file "checkout.feature" with 3 scenarios
      const content = `Feature: Checkout Process

  Scenario: Add to cart
    Given I have items
    When I add to cart
    Then cart updated

  Scenario: Proceed to payment
    Given I have cart items
    When I checkout
    Then payment page shown

  Scenario: Complete order
    Given I am on payment page
    When I submit payment
    Then order confirmed`;

      await writeFile(join(testDir, 'spec/features/checkout.feature'), content);

      // When I run `fspec add-architecture checkout "Payment processing architecture"`
      const result = await addArchitecture({
        feature: 'checkout',
        text: 'Payment processing architecture',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/checkout.feature'),
        'utf-8'
      );

      // And the feature file should still have 3 scenarios
      const scenarioCount = (updatedContent.match(/Scenario:/g) || []).length;
      expect(scenarioCount).toBe(3);

      // And all scenario content should be preserved
      expect(updatedContent).toContain('Add to cart');
      expect(updatedContent).toContain('Proceed to payment');
      expect(updatedContent).toContain('Complete order');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add architecture notes preserves Background section', () => {
    it('should preserve Background section', async () => {
      // Given I have a feature file "auth.feature" with Background section
      const content = `Feature: Authentication

  Background: User Story
    As a user
    I want to authenticate
    So that I can access protected resources

  Scenario: Login
    Given I enter credentials
    When I submit
    Then I am authenticated`;

      await writeFile(join(testDir, 'spec/features/auth.feature'), content);

      // When I run `fspec add-architecture auth "OAuth 2.0 implementation"`
      const result = await addArchitecture({
        feature: 'auth',
        text: 'OAuth 2.0 implementation',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/auth.feature'),
        'utf-8'
      );

      // And the Background section should be preserved
      expect(updatedContent).toContain('Background: User Story');
      expect(updatedContent).toContain('As a user');

      // And the doc string should be before the Background section
      const docStringIndex = updatedContent.indexOf('"""');
      const backgroundIndex = updatedContent.indexOf('Background: User Story');
      expect(docStringIndex).toBeLessThan(backgroundIndex);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add architecture notes preserves feature-level tags', () => {
    it('should preserve feature tags', async () => {
      // Given I have a feature file "search.feature" with tags "@api @critical"
      const content = `@api @critical
Feature: Search Functionality

  Scenario: Basic search
    Given I enter search term
    When I search
    Then results displayed`;

      await writeFile(join(testDir, 'spec/features/search.feature'), content);

      // When I run `fspec add-architecture search "ElasticSearch integration"`
      const result = await addArchitecture({
        feature: 'search',
        text: 'ElasticSearch integration',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/search.feature'),
        'utf-8'
      );

      // And the feature tags "@api @critical" should be preserved
      expect(updatedContent).toContain('@api @critical');

      // And the doc string should be after the Feature line
      const featureIndex = updatedContent.indexOf(
        'Feature: Search Functionality'
      );
      const docStringIndex = updatedContent.indexOf('"""');
      expect(docStringIndex).toBeGreaterThan(featureIndex);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Reject non-existent feature file', () => {
    it('should return error for missing file', async () => {
      // Given I have no feature file named "missing.feature"
      // When I run `fspec add-architecture missing "Some notes"`
      const result = await addArchitecture({
        feature: 'missing',
        text: 'Some notes',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Feature file not found"
      expect(result.error).toMatch(/feature file not found/i);
    });
  });

  describe('Scenario: Accept feature file by name without .feature extension', () => {
    it('should find file by name', async () => {
      // Given I have a feature file "spec/features/login.feature"
      const content = `Feature: Login
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec add-architecture login "Authentication notes"`
      const result = await addArchitecture({
        feature: 'login',
        text: 'Authentication notes',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file "spec/features/login.feature" should contain the doc string
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('Authentication notes');
    });
  });

  describe('Scenario: Accept feature file by full path', () => {
    it('should accept full path', async () => {
      // Given I have a feature file "spec/features/user-management.feature"
      const content = `Feature: User Management
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/user-management.feature'),
        content
      );

      // When I run `fspec add-architecture spec/features/user-management.feature "User CRUD operations"`
      const result = await addArchitecture({
        feature: 'spec/features/user-management.feature',
        text: 'User CRUD operations',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file should contain the doc string with "User CRUD operations"
      const updatedContent = await readFile(
        join(testDir, 'spec/features/user-management.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('User CRUD operations');
    });
  });

  describe('Scenario: Proper indentation of doc string content', () => {
    it('should indent doc string content correctly', async () => {
      // Given I have a feature file "reporting.feature"
      const content = `Feature: Reporting
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/reporting.feature'),
        content
      );

      // When I run `fspec add-architecture reporting "Line 1\nLine 2\nLine 3"`
      const result = await addArchitecture({
        feature: 'reporting',
        text: 'Line 1\nLine 2\nLine 3',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/reporting.feature'),
        'utf-8'
      );

      // And the doc string content should be indented with 2 spaces
      expect(updatedContent).toContain('  Line 1');
      expect(updatedContent).toContain('  Line 2');
      expect(updatedContent).toContain('  Line 3');

      // And the opening and closing triple quotes should not be indented
      const lines = updatedContent.split('\n');
      const openingQuoteLine = lines.find(
        line => line.trim().startsWith('"""') && line.trim() === '"""'
      );
      expect(openingQuoteLine).toBeDefined();
      expect(openingQuoteLine?.startsWith('  """')).toBe(true);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Preserve scenario-level tags', () => {
    it('should preserve scenario tags', async () => {
      // Given I have a feature file "notifications.feature" with scenarios tagged "@email @sms"
      const content = `Feature: Notifications

  @email @sms
  Scenario: Send notification
    Given I have a message
    When I send notification
    Then user receives it`;

      await writeFile(
        join(testDir, 'spec/features/notifications.feature'),
        content
      );

      // When I run `fspec add-architecture notifications "Notification service architecture"`
      const result = await addArchitecture({
        feature: 'notifications',
        text: 'Notification service architecture',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/notifications.feature'),
        'utf-8'
      );

      // And the scenario tags "@email @sms" should be preserved
      expect(updatedContent).toContain('@email @sms');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Empty architecture text should be rejected', () => {
    it('should reject empty text', async () => {
      // Given I have a feature file "dashboard.feature"
      const content = `Feature: Dashboard
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/dashboard.feature'),
        content
      );

      // When I run `fspec add-architecture dashboard ""`
      const result = await addArchitecture({
        feature: 'dashboard',
        text: '',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Architecture text cannot be empty"
      expect(result.error).toMatch(/architecture text cannot be empty/i);
    });
  });
});
