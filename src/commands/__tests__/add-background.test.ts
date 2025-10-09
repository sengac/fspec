import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { addBackground } from '../add-background';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

describe('Feature: Add Background Section to Feature Files', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-add-background');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add background to feature without existing background', () => {
    it('should add Background after Feature line', async () => {
      // Given I have a feature file "login.feature" with no Background section
      const content = `@auth
Feature: User Login

  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec add-background login "As a user\nI want to log in\nSo that I can access my account"`
      const result = await addBackground({
        feature: 'login',
        text: 'As a user\nI want to log in\nSo that I can access my account',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );

      // And the feature file should contain "Background: User Story"
      expect(updatedContent).toContain('Background: User Story');

      // And the background should contain "As a user"
      expect(updatedContent).toContain('As a user');

      // And the background should contain "I want to log in"
      expect(updatedContent).toContain('I want to log in');

      // And the background should contain "So that I can access my account"
      expect(updatedContent).toContain('So that I can access my account');

      // And the Background should be after the Feature line
      const featureIndex = updatedContent.indexOf('Feature: User Login');
      const backgroundIndex = updatedContent.indexOf('Background: User Story');
      expect(backgroundIndex).toBeGreaterThan(featureIndex);

      // And the Background should be before the first Scenario
      const scenarioIndex = updatedContent.indexOf(
        'Scenario: Successful login'
      );
      expect(backgroundIndex).toBeLessThan(scenarioIndex);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add background with standard user story format', () => {
    it('should follow user story format', async () => {
      // Given I have a feature file "search.feature" with no Background section
      const content = `Feature: Product Search
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/search.feature'), content);

      // When I run `fspec add-background search "As a customer\nI want to search products\nSo that I can find what I need"`
      const result = await addBackground({
        feature: 'search',
        text: 'As a customer\nI want to search products\nSo that I can find what I need',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/search.feature'),
        'utf-8'
      );

      // And the background should follow the "As a... I want to... So that..." format
      expect(updatedContent).toContain('As a customer');
      expect(updatedContent).toContain('I want to search products');
      expect(updatedContent).toContain('So that I can find what I need');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Replace existing Background section', () => {
    it('should replace existing Background', async () => {
      // Given I have a feature file "checkout.feature" with existing Background "Old story"
      const content = `Feature: Checkout Process

  Background: User Story
    Old story

  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/checkout.feature'), content);

      // When I run `fspec add-background checkout "As a buyer\nI want to complete checkout\nSo that I can purchase items"`
      const result = await addBackground({
        feature: 'checkout',
        text: 'As a buyer\nI want to complete checkout\nSo that I can purchase items',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/checkout.feature'),
        'utf-8'
      );

      // And the background should contain "As a buyer"
      expect(updatedContent).toContain('As a buyer');

      // And the background should not contain "Old story"
      expect(updatedContent).not.toContain('Old story');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add background preserves scenarios', () => {
    it('should preserve all scenarios', async () => {
      // Given I have a feature file "payment.feature" with 3 scenarios
      const content = `Feature: Payment Processing

  Scenario: Add payment method
    Given I have payment info
    When I add it
    Then saved

  Scenario: Process payment
    Given I have cart
    When I pay
    Then processed

  Scenario: Confirm payment
    Given payment processed
    When I confirm
    Then confirmed`;

      await writeFile(join(testDir, 'spec/features/payment.feature'), content);

      // When I run `fspec add-background payment "As a customer\nI want to pay securely\nSo that my data is protected"`
      const result = await addBackground({
        feature: 'payment',
        text: 'As a customer\nI want to pay securely\nSo that my data is protected',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/payment.feature'),
        'utf-8'
      );

      // And the feature file should still have 3 scenarios
      const scenarioCount = (updatedContent.match(/Scenario:/g) || []).length;
      expect(scenarioCount).toBe(3);

      // And all scenario content should be preserved
      expect(updatedContent).toContain('Add payment method');
      expect(updatedContent).toContain('Process payment');
      expect(updatedContent).toContain('Confirm payment');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add background preserves architecture doc string', () => {
    it('should preserve doc string', async () => {
      // Given I have a feature file "api.feature" with architecture doc string
      const content = `Feature: API Operations
  """
  Architecture notes:
  - REST API implementation
  """

  Scenario: Test API
    Given I call API
    When response received
    Then success`;

      await writeFile(join(testDir, 'spec/features/api.feature'), content);

      // When I run `fspec add-background api "As a developer\nI want to use the API\nSo that I can integrate"`
      const result = await addBackground({
        feature: 'api',
        text: 'As a developer\nI want to use the API\nSo that I can integrate',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/api.feature'),
        'utf-8'
      );

      // And the architecture doc string should be preserved
      expect(updatedContent).toContain('Architecture notes:');
      expect(updatedContent).toContain('- REST API implementation');

      // And the Background should be after the doc string
      const docStringIndex = updatedContent.lastIndexOf('"""');
      const backgroundIndex = updatedContent.indexOf('Background: User Story');
      expect(backgroundIndex).toBeGreaterThan(docStringIndex);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Add background preserves feature-level tags', () => {
    it('should preserve feature tags', async () => {
      // Given I have a feature file "auth.feature" with tags "@security @critical"
      const content = `@security @critical
Feature: Authentication

  Scenario: Login
    Given I enter credentials
    When I submit
    Then authenticated`;

      await writeFile(join(testDir, 'spec/features/auth.feature'), content);

      // When I run `fspec add-background auth "As a user\nI want authentication\nSo that my data is secure"`
      const result = await addBackground({
        feature: 'auth',
        text: 'As a user\nI want authentication\nSo that my data is secure',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/auth.feature'),
        'utf-8'
      );

      // And the feature tags "@security @critical" should be preserved
      expect(updatedContent).toContain('@security @critical');

      // And the Background should be after the Feature line
      const featureIndex = updatedContent.indexOf('Feature: Authentication');
      const backgroundIndex = updatedContent.indexOf('Background: User Story');
      expect(backgroundIndex).toBeGreaterThan(featureIndex);

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
      // When I run `fspec add-background missing "As a user..."`
      const result = await addBackground({
        feature: 'missing',
        text: 'As a user...',
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
      // Given I have a feature file "spec/features/dashboard.feature"
      const content = `Feature: Dashboard
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/dashboard.feature'),
        content
      );

      // When I run `fspec add-background dashboard "As a user\nI want to view dashboard\nSo that I see overview"`
      const result = await addBackground({
        feature: 'dashboard',
        text: 'As a user\nI want to view dashboard\nSo that I see overview',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file "spec/features/dashboard.feature" should contain the Background
      const updatedContent = await readFile(
        join(testDir, 'spec/features/dashboard.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('Background: User Story');
      expect(updatedContent).toContain('As a user');
    });
  });

  describe('Scenario: Accept feature file by full path', () => {
    it('should accept full path', async () => {
      // Given I have a feature file "spec/features/reporting.feature"
      const content = `Feature: Reporting
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/reporting.feature'),
        content
      );

      // When I run `fspec add-background spec/features/reporting.feature "As a manager\nI want reports\nSo that I track progress"`
      const result = await addBackground({
        feature: 'spec/features/reporting.feature',
        text: 'As a manager\nI want reports\nSo that I track progress',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file should contain the Background with "As a manager"
      const updatedContent = await readFile(
        join(testDir, 'spec/features/reporting.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('As a manager');
    });
  });

  describe('Scenario: Proper indentation of Background content', () => {
    it('should indent Background content correctly', async () => {
      // Given I have a feature file "notifications.feature"
      const content = `Feature: Notifications
  Scenario: Test
    Given step`;

      await writeFile(
        join(testDir, 'spec/features/notifications.feature'),
        content
      );

      // When I run `fspec add-background notifications "As a user\nI want notifications\nSo that I stay informed"`
      const result = await addBackground({
        feature: 'notifications',
        text: 'As a user\nI want notifications\nSo that I stay informed',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/notifications.feature'),
        'utf-8'
      );

      // And the Background keyword should not be indented
      const lines = updatedContent.split('\n');
      const backgroundLine = lines.find(line =>
        line.includes('Background: User Story')
      );
      expect(backgroundLine?.startsWith('  Background:')).toBe(true);

      // And the Background content should be indented with 4 spaces
      expect(updatedContent).toContain('    As a user');
      expect(updatedContent).toContain('    I want notifications');
      expect(updatedContent).toContain('    So that I stay informed');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Preserve scenario-level tags', () => {
    it('should preserve scenario tags', async () => {
      // Given I have a feature file "orders.feature" with scenarios tagged "@smoke @regression"
      const content = `Feature: Order Management

  @smoke @regression
  Scenario: Create order
    Given I have items
    When I create order
    Then order created`;

      await writeFile(join(testDir, 'spec/features/orders.feature'), content);

      // When I run `fspec add-background orders "As a customer\nI want to manage orders\nSo that I track purchases"`
      const result = await addBackground({
        feature: 'orders',
        text: 'As a customer\nI want to manage orders\nSo that I track purchases',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/orders.feature'),
        'utf-8'
      );

      // And the scenario tags "@smoke @regression" should be preserved
      expect(updatedContent).toContain('@smoke @regression');

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Empty background text should be rejected', () => {
    it('should reject empty text', async () => {
      // Given I have a feature file "products.feature"
      const content = `Feature: Product Management
  Scenario: Test
    Given step`;

      await writeFile(join(testDir, 'spec/features/products.feature'), content);

      // When I run `fspec add-background products ""`
      const result = await addBackground({
        feature: 'products',
        text: '',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Background text cannot be empty"
      expect(result.error).toMatch(/background text cannot be empty/i);
    });
  });

  describe('Scenario: Background positioned after doc string and before scenarios', () => {
    it('should position Background correctly', async () => {
      // Given I have a feature file "integration.feature" with doc string and 2 scenarios
      const content = `Feature: System Integration
  """
  Integration architecture
  """

  Scenario: First integration
    Given system A
    When connects to B
    Then integrated

  Scenario: Second integration
    Given system C
    When connects to D
    Then integrated`;

      await writeFile(
        join(testDir, 'spec/features/integration.feature'),
        content
      );

      // When I run `fspec add-background integration "As a developer\nI want integration\nSo that systems connect"`
      const result = await addBackground({
        feature: 'integration',
        text: 'As a developer\nI want integration\nSo that systems connect',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/integration.feature'),
        'utf-8'
      );

      // And the Background should be after the architecture doc string
      const docStringEnd = updatedContent.lastIndexOf('"""');
      const backgroundIndex = updatedContent.indexOf('Background: User Story');
      expect(backgroundIndex).toBeGreaterThan(docStringEnd);

      // And the Background should be before the first Scenario
      const scenarioIndex = updatedContent.indexOf(
        'Scenario: First integration'
      );
      expect(backgroundIndex).toBeLessThan(scenarioIndex);

      // And the file should have valid Gherkin syntax
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });
});
