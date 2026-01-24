import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { updateScenario } from '../update-scenario';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Update Scenario Name', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('update-scenario');
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Rename scenario with simple name', () => {
    it('should rename scenario and preserve structure', async () => {
      // Given I have a scenario named "Old scenario name"
      const content = `Feature: Login

  Scenario: Old scenario name
    Given I am on the page
    When I click button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/login.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario login "Old scenario name" "New scenario name"`
      const result = await updateScenario({
        feature: 'login',
        oldName: 'Old scenario name',
        newName: 'New scenario name',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the scenario name should be "New scenario name"
      expect(updatedContent).toContain('Scenario: New scenario name');
      expect(updatedContent).not.toContain('Scenario: Old scenario name');

      // And all steps should be preserved
      expect(updatedContent).toContain('Given I am on the page');
      expect(updatedContent).toContain('When I click button');
      expect(updatedContent).toContain('Then I see result');

      // And the feature file should be valid Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();

      // And the output should show "Successfully renamed scenario to 'New scenario name'"
      expect(result.message).toContain('Successfully renamed');
      expect(result.message).toContain('New scenario name');
    });
  });

  describe('Scenario: Rename scenario preserves steps', () => {
    it('should preserve all steps unchanged', async () => {
      // Given I have a scenario "Login test" with 5 steps
      const content = `Feature: Test

  Scenario: Login test
    Given I am on the login page
    When I enter username
    And I enter password
    And I click submit
    Then I should be logged in
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario test "Login test" "User authentication test"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'Login test',
        newName: 'User authentication test',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the scenario name should be updated
      expect(updatedContent).toContain('Scenario: User authentication test');

      // And all 5 steps should remain unchanged
      expect(updatedContent).toContain('Given I am on the login page');
      expect(updatedContent).toContain('When I enter username');
      expect(updatedContent).toContain('And I enter password');
      expect(updatedContent).toContain('And I click submit');
      expect(updatedContent).toContain('Then I should be logged in');

      // And step order should be preserved
      const lines = updatedContent.split('\n');
      const stepLines = lines.filter(l =>
        l.trim().match(/^(Given|When|And|Then)/)
      );
      expect(stepLines[0]).toContain('Given I am on the login page');
      expect(stepLines[4]).toContain('Then I should be logged in');
    });
  });

  describe('Scenario: Rename scenario preserves tags', () => {
    it('should preserve scenario tags', async () => {
      // Given I have a scenario with tags @critical @high
      // And the scenario is named "Payment processing"
      const content = `Feature: Test

  @critical @high
  Scenario: Payment processing
    Given I initiate payment
    When I confirm transaction
    Then payment should succeed
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario test "Payment processing" "Process payment transaction"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'Payment processing',
        newName: 'Process payment transaction',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the scenario tags should be preserved
      expect(updatedContent).toContain('@critical @high');

      // And the scenario name should be updated
      expect(updatedContent).toContain('Scenario: Process payment transaction');
    });
  });

  describe('Scenario: Rename scenario when multiple scenarios exist', () => {
    it('should rename only specified scenario', async () => {
      // Given I have a feature with scenarios "First", "Second", "Third"
      const content = `Feature: Test

  Scenario: First
    Given first step

  Scenario: Second
    Given second step

  Scenario: Third
    Given third step
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario test "Second" "Middle scenario"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'Second',
        newName: 'Middle scenario',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the "First" scenario should remain unchanged
      expect(updatedContent).toContain('Scenario: First');

      // And the "Second" scenario should be renamed to "Middle scenario"
      expect(updatedContent).toContain('Scenario: Middle scenario');
      expect(updatedContent).not.toContain('Scenario: Second');

      // And the "Third" scenario should remain unchanged
      expect(updatedContent).toContain('Scenario: Third');
    });
  });

  describe('Scenario: Attempt to rename to existing scenario name', () => {
    it('should return error for duplicate name', async () => {
      // Given I have scenarios "Login test" and "Registration test"
      const content = `Feature: Test

  Scenario: Login test
    Given login step

  Scenario: Registration test
    Given registration step
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec update-scenario test "Login test" "Registration test"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'Login test',
        newName: 'Registration test',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Scenario 'Registration test' already exists"
      expect(result.error).toMatch(/already exists/i);
      expect(result.error).toContain('Registration test');

      // And the file should remain unchanged
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Attempt to rename non-existent scenario', () => {
    it('should return error for non-existent scenario', async () => {
      // Given I have a feature file with existing scenarios
      const content = `Feature: Test

  Scenario: Existing scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec update-scenario test "Non-existent scenario" "New name"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'Non-existent scenario',
        newName: 'New name',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Scenario 'Non-existent scenario' not found"
      expect(result.error).toMatch(/not found/i);
      expect(result.error).toContain('Non-existent scenario');

      // And the file should remain unchanged
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Rename scenario with special characters', () => {
    it('should handle special characters', async () => {
      // Given I have a scenario named "User can login"
      const content = `Feature: Test

  Scenario: User can login
    Given test step
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario test "User can login" "User can't login with invalid credentials"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'User can login',
        newName: "User can't login with invalid credentials",
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the scenario name should contain apostrophe and special characters
      expect(updatedContent).toContain(
        "User can't login with invalid credentials"
      );

      // And the feature file should be valid Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Rename scenario preserves indentation', () => {
    it('should maintain proper indentation', async () => {
      // Given I have a properly indented scenario
      const content = `Feature: Test

  Scenario: Old name
    Given I am on the page
    When I click button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario test "Old name" "New name"`
      const result = await updateScenario({
        feature: 'test',
        oldName: 'Old name',
        newName: 'New name',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the scenario header should maintain proper indentation
      expect(updatedContent).toContain('  Scenario: New name');

      // And all steps should maintain their indentation
      expect(updatedContent).toContain('    Given I am on the page');
      expect(updatedContent).toContain('    When I click button');
      expect(updatedContent).toContain('    Then I see result');
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

      // When I run `fspec update-scenario broken "Some scenario" "New name"`
      const result = await updateScenario({
        feature: 'broken',
        oldName: 'Some scenario',
        newName: 'New name',
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

  describe('Scenario: Rename scenario from nested directory', () => {
    it('should handle full paths', async () => {
      // Given I have a feature file "spec/features/auth/login.feature"
      // And the feature has a scenario "Login flow"
      await mkdir(join(testDir, 'spec/features/auth'), { recursive: true });
      const content = `Feature: Login

  Scenario: Login flow
    Given test
`;
      const filePath = join(testDir, 'spec/features/auth/login.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-scenario spec/features/auth/login.feature "Login flow" "Authentication workflow"`
      const result = await updateScenario({
        feature: 'spec/features/auth/login.feature',
        oldName: 'Login flow',
        newName: 'Authentication workflow',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the scenario should be renamed
      expect(updatedContent).toContain('Scenario: Authentication workflow');

      // And the file should remain valid
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });
});
