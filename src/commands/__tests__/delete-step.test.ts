import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { deleteStep } from '../delete-step';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Delete Step from Scenario', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('delete-step');
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Delete step by exact text', () => {
    it('should remove specified step and preserve others', async () => {
      // Given I have a scenario "Login" with steps
      const content = `Feature: Login

  Scenario: Login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
    And I should see the dashboard
`;
      const filePath = join(testDir, 'spec/features/login.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step login "Login" "And I should see the dashboard"`
      const result = await deleteStep({
        feature: 'login',
        scenario: 'Login',
        step: 'And I should see the dashboard',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the step "And I should see the dashboard" should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('And I should see the dashboard');

      // And the other 3 steps should remain
      expect(updatedContent).toContain('Given I am on the login page');
      expect(updatedContent).toContain('When I enter valid credentials');
      expect(updatedContent).toContain('Then I should be logged in');

      // And the feature file should be valid Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();

      // And the output should show "Successfully deleted step from scenario 'Login'"
      expect(result.message).toContain('Successfully deleted step');
      expect(result.message).toContain('Login');
    });
  });

  describe('Scenario: Delete Given step', () => {
    it('should remove Given step', async () => {
      // Given I have a scenario with a Given step "I am logged in"
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am logged in
    When I click the button
    Then I should see success
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "Test scenario" "Given I am logged in"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: 'Given I am logged in',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the Given step should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('Given I am logged in');

      // And remaining steps should be preserved
      expect(updatedContent).toContain('When I click the button');
      expect(updatedContent).toContain('Then I should see success');
    });
  });

  describe('Scenario: Delete When step', () => {
    it('should remove When step and preserve Given and Then', async () => {
      // Given I have a scenario with steps Given, When, Then
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on the page
    When I click the button
    Then I should see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "Test scenario" "When I click the button"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: 'When I click the button',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the When step should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('When I click the button');

      // And the Given and Then steps should remain
      expect(updatedContent).toContain('Given I am on the page');
      expect(updatedContent).toContain('Then I should see result');
    });
  });

  describe('Scenario: Delete Then step', () => {
    it('should remove only specified Then step', async () => {
      // Given I have a scenario with multiple Then steps
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on the page
    When I click submit
    Then I should see success
    And I should see confirmation
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "Test scenario" "Then I should see success"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: 'Then I should see success',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And only the specified Then step should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('Then I should see success');
      expect(updatedContent).toContain('And I should see confirmation');
    });
  });

  describe('Scenario: Delete last remaining step', () => {
    it('should remove step leaving scenario with no steps', async () => {
      // Given I have a scenario with only one step "Given test"
      const content = `Feature: Test

  Scenario: Test scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "Test scenario" "Given test"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: 'Given test',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the step should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('Given test');

      // And the scenario should have no steps
      expect(updatedContent).toContain('Scenario: Test scenario');

      // And the scenario header should remain
      const lines = updatedContent.split('\n').filter(l => l.trim());
      expect(lines.some(l => l.includes('Scenario: Test scenario'))).toBe(true);
    });
  });

  describe('Scenario: Attempt to delete non-existent step', () => {
    it('should return error for non-existent step', async () => {
      // Given I have a scenario with 3 steps
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on the page
    When I click button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec delete-step test "Test scenario" "Given non-existent step"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: 'Given non-existent step',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Step 'Given non-existent step' not found in scenario 'Test scenario'"
      expect(result.error).toMatch(/not found/i);
      expect(result.error).toContain('Given non-existent step');
      expect(result.error).toContain('Test scenario');

      // And the file should remain unchanged
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Delete step from specific scenario when multiple scenarios exist', () => {
    it('should remove step from specified scenario only', async () => {
      // Given I have two scenarios "First" and "Second"
      // And both scenarios have a step "Given I am logged in"
      const content = `Feature: Test

  Scenario: First
    Given I am logged in
    When I do something
    Then I see result

  Scenario: Second
    Given I am logged in
    When I do other thing
    Then I see other result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "First" "Given I am logged in"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'First',
        step: 'Given I am logged in',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step should be removed from "First" scenario only
      const firstScenarioSection = updatedContent.split('Scenario: Second')[0];
      expect(firstScenarioSection).not.toContain('Given I am logged in');

      // And the step should remain in "Second" scenario
      const secondScenarioSection = updatedContent.split('Scenario: Second')[1];
      expect(secondScenarioSection).toContain('Given I am logged in');
    });
  });

  describe('Scenario: Delete step preserves indentation', () => {
    it('should maintain proper indentation', async () => {
      // Given I have a properly indented scenario
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on the page
    When I click submit
    Then I see success
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "Test scenario" "When I click submit"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: 'When I click submit',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the remaining steps should preserve proper indentation
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toContain('    Given I am on the page');
      expect(updatedContent).toContain('    Then I see success');

      // And the file should remain properly formatted
      expect(updatedContent).toContain('  Scenario: Test scenario');
    });
  });

  describe('Scenario: Delete step with special characters in text', () => {
    it('should handle special characters', async () => {
      // Given I have a step "Given I can't access @admin features"
      const content = `Feature: Test

  Scenario: Test scenario
    Given I can't access @admin features
    When I try to login
    Then I see error
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-step test "Test scenario" "Given I can't access @admin features"`
      const result = await deleteStep({
        feature: 'test',
        scenario: 'Test scenario',
        step: "Given I can't access @admin features",
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the step should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain(
        "Given I can't access @admin features"
      );
    });
  });

  describe('Scenario: Handle scenario not found', () => {
    it('should return error when scenario does not exist', async () => {
      // Given I have a feature file "login.feature"
      const content = `Feature: Login

  Scenario: Existing scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/login.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec delete-step login "Non-existent scenario" "Given test"`
      const result = await deleteStep({
        feature: 'login',
        scenario: 'Non-existent scenario',
        step: 'Given test',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Scenario 'Non-existent scenario' not found"
      expect(result.error).toMatch(/scenario.*not found/i);
      expect(result.error).toContain('Non-existent scenario');

      // And the file should remain unchanged
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Handle feature file with invalid syntax', () => {
    it('should return error for invalid Gherkin', async () => {
      // Given I have a feature file with syntax errors
      const content = `Feature Test
  This is broken
  Scenario: Some scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/broken.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec delete-step broken "Some scenario" "Given test"`
      const result = await deleteStep({
        feature: 'broken',
        scenario: 'Some scenario',
        step: 'Given test',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Invalid Gherkin syntax"
      expect(result.error).toMatch(/invalid.*syntax/i);

      // And the file should remain unchanged
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });
});
