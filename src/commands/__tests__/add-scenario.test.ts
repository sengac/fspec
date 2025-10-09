import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { addScenario } from '../add-scenario';

describe('Feature: Add Scenario to Existing Feature File', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-add-scenario');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add scenario to feature file with template', () => {
    it('should add scenario with Given/When/Then placeholders', async () => {
      // Given I have a feature file "spec/features/login.feature"
      const featureContent = `@phase1 @auth
Feature: User Login

  Background: User Story
    As a user
    I want to log in
    So that I can access my account

  Scenario: Existing scenario
    Given I am on the login page
    When I enter credentials
    Then I should be logged in
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-scenario login "Successful login with valid credentials"`
      const result = await addScenario(
        'login',
        'Successful login with valid credentials',
        {
          cwd: testDir,
        }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file should contain a new scenario named "Successful login with valid credentials"
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain(
        'Scenario: Successful login with valid credentials'
      );

      // And the scenario should have Given/When/Then placeholders
      expect(updatedContent).toContain('Given [precondition]');
      expect(updatedContent).toContain('When [action]');
      expect(updatedContent).toContain('Then [expected outcome]');

      // And the file should remain valid Gherkin syntax
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Add scenario with feature name (without .feature extension)', () => {
    it('should find feature file by name without extension', async () => {
      // Given I have a feature file "spec/features/user-auth.feature"
      const featureContent = `Feature: User Authentication

  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/user-auth.feature'),
        featureContent
      );

      // When I run `fspec add-scenario user-auth "Password reset"`
      const result = await addScenario('user-auth', 'Password reset', {
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file should contain a new scenario named "Password reset"
      const updatedContent = await readFile(
        join(testDir, 'spec/features/user-auth.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('Scenario: Password reset');
    });
  });

  describe('Scenario: Add scenario with full file path', () => {
    it('should accept full file path', async () => {
      // Given I have a feature file "spec/features/shopping-cart.feature"
      const featureContent = `Feature: Shopping Cart

  Scenario: View cart
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/shopping-cart.feature'),
        featureContent
      );

      // When I run `fspec add-scenario spec/features/shopping-cart.feature "Add item to cart"`
      const result = await addScenario(
        'spec/features/shopping-cart.feature',
        'Add item to cart',
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file should contain a new scenario named "Add item to cart"
      const updatedContent = await readFile(
        join(testDir, 'spec/features/shopping-cart.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('Scenario: Add item to cart');
    });
  });

  describe('Scenario: Add multiple scenarios to same feature', () => {
    it('should add scenarios in order', async () => {
      // Given I have a feature file "spec/features/payment.feature" with 1 scenario
      const featureContent = `Feature: Payment

  Scenario: Cash payment
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/payment.feature'),
        featureContent
      );

      // When I run `fspec add-scenario payment "Credit card payment"`
      await addScenario('payment', 'Credit card payment', { cwd: testDir });

      // And I run `fspec add-scenario payment "PayPal payment"`
      await addScenario('payment', 'PayPal payment', { cwd: testDir });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/payment.feature'),
        'utf-8'
      );

      // Then the file should contain 3 scenarios total
      const scenarioMatches = updatedContent.match(/Scenario:/g);
      expect(scenarioMatches?.length).toBe(3);

      // And all scenarios should be in the order they were added
      const cashIndex = updatedContent.indexOf('Scenario: Cash payment');
      const creditIndex = updatedContent.indexOf(
        'Scenario: Credit card payment'
      );
      const paypalIndex = updatedContent.indexOf('Scenario: PayPal payment');
      expect(cashIndex).toBeLessThan(creditIndex);
      expect(creditIndex).toBeLessThan(paypalIndex);

      // And the file should remain valid Gherkin syntax
      const result = await addScenario('payment', 'Test', {
        cwd: testDir,
        dryRun: true,
      });
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Preserve existing content when adding scenario', () => {
    it('should preserve tags, background, and existing scenarios', async () => {
      // Given I have a feature file with tags, background, and existing scenarios
      const featureContent = `@phase1 @important
Feature: My Feature

  Background: Setup
    Given setup step

  Scenario: First scenario
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-scenario my-feature "New scenario"`
      await addScenario('my-feature', 'New scenario', { cwd: testDir });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then all existing tags should be preserved
      expect(updatedContent).toContain('@phase1');
      expect(updatedContent).toContain('@important');

      // And the background section should be preserved
      expect(updatedContent).toContain('Background: Setup');
      expect(updatedContent).toContain('Given setup step');

      // And all existing scenarios should be preserved
      expect(updatedContent).toContain('Scenario: First scenario');

      // And the new scenario should be added at the end
      expect(updatedContent).toContain('Scenario: New scenario');
      const firstIndex = updatedContent.indexOf('Scenario: First scenario');
      const newIndex = updatedContent.indexOf('Scenario: New scenario');
      expect(newIndex).toBeGreaterThan(firstIndex);
    });
  });

  describe('Scenario: Insert scenario before Scenario Outline if present', () => {
    it('should insert new scenario before Scenario Outline', async () => {
      // Given I have a feature file with 2 scenarios and 1 Scenario Outline
      const featureContent = `Feature: Test

  Scenario: First
    Given test

  Scenario: Second
    Given test

  Scenario Outline: Parameterized
    Given <param>

    Examples:
      | param |
      | value |
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-scenario my-feature "New scenario"`
      await addScenario('my-feature', 'New scenario', { cwd: testDir });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the new scenario should be inserted after the 2 existing scenarios
      const firstIndex = updatedContent.indexOf('Scenario: First');
      const secondIndex = updatedContent.indexOf('Scenario: Second');
      const newIndex = updatedContent.indexOf('Scenario: New scenario');
      expect(newIndex).toBeGreaterThan(firstIndex);
      expect(newIndex).toBeGreaterThan(secondIndex);

      // And the new scenario should be before the Scenario Outline
      const outlineIndex = updatedContent.indexOf('Scenario Outline:');
      expect(newIndex).toBeLessThan(outlineIndex);

      // And the file should remain valid Gherkin syntax
      const result = await addScenario('my-feature', 'Test', {
        cwd: testDir,
        dryRun: true,
      });
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Handle feature file not found', () => {
    it('should show error and suggest create-feature', async () => {
      // Given there is no feature file "spec/features/missing.feature"
      // (file does not exist)

      // When I run `fspec add-scenario missing "New scenario"`
      const result = await addScenario('missing', 'New scenario', {
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show error "Feature file not found"
      expect(result.error).toContain('not found');

      // And the output should suggest using create-feature command
      expect(result.suggestion).toContain('create-feature');
    });
  });

  describe('Scenario: Handle invalid feature file syntax', () => {
    it('should show error and not modify file', async () => {
      // Given I have a feature file "spec/features/broken.feature" with invalid syntax
      const invalidContent = 'This is not valid Gherkin syntax';
      await writeFile(
        join(testDir, 'spec/features/broken.feature'),
        invalidContent
      );

      // When I run `fspec add-scenario broken "New scenario"`
      const result = await addScenario('broken', 'New scenario', {
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show error about invalid Gherkin syntax
      expect(result.error).toMatch(/invalid|syntax/);

      // And the output should suggest running validate command first
      expect(result.suggestion).toContain('validate');

      // And the file should not be modified
      const fileContent = await readFile(
        join(testDir, 'spec/features/broken.feature'),
        'utf-8'
      );
      expect(fileContent).toBe(invalidContent);
    });
  });

  describe('Scenario: Handle duplicate scenario name', () => {
    it('should show warning but still add scenario', async () => {
      // Given I have a feature file with scenario "Login with email"
      const featureContent = `Feature: Login

  Scenario: Login with email
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-scenario my-feature "Login with email"`
      const result = await addScenario('my-feature', 'Login with email', {
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show warning about duplicate scenario name
      expect(result.warning).toMatch(/duplicate|exists/);

      // And the new scenario should still be added
      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );
      const scenarioMatches = updatedContent.match(
        /Scenario: Login with email/g
      );
      expect(scenarioMatches?.length).toBe(2);
    });
  });

  describe('Scenario: Use proper indentation in added scenario', () => {
    it('should match existing 2-space indentation', async () => {
      // Given I have a feature file with 2-space indentation
      const featureContent = `Feature: Test

  Scenario: Existing
    Given step one
    When step two
    Then step three
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-scenario my-feature "New scenario"`
      await addScenario('my-feature', 'New scenario', { cwd: testDir });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the new scenario should use 2-space indentation
      expect(updatedContent).toContain('  Scenario: New scenario');

      // And all steps should be indented 4 spaces from feature level
      expect(updatedContent).toContain('    Given [precondition]');
      expect(updatedContent).toContain('    When [action]');
      expect(updatedContent).toContain('    Then [expected outcome]');

      // And the indentation should match existing scenarios
      const lines = updatedContent.split('\n');
      const scenarioLines = lines.filter(line => line.includes('Scenario:'));
      const allHaveSameIndent = scenarioLines.every(line =>
        line.startsWith('  Scenario:')
      );
      expect(allHaveSameIndent).toBe(true);
    });
  });

  describe('Scenario: Scenario template includes placeholder steps', () => {
    it('should include Given/When/Then placeholders', async () => {
      // Given I have a feature file "spec/features/test.feature"
      const featureContent = `Feature: Test

  Scenario: Existing
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run `fspec add-scenario test "Test scenario"`
      await addScenario('test', 'Test scenario', { cwd: testDir });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );

      // Then the scenario should contain "Given [precondition]"
      expect(updatedContent).toContain('Given [precondition]');

      // And the scenario should contain "When [action]"
      expect(updatedContent).toContain('When [action]');

      // And the scenario should contain "Then [expected outcome]"
      expect(updatedContent).toContain('Then [expected outcome]');
    });
  });
});
