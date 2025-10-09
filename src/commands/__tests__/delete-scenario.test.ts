import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { deleteScenario } from '../delete-scenario';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

describe('Feature: Delete Scenario from Feature File', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-delete-scenario');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Delete scenario by exact name', () => {
    it('should remove scenario and preserve others', async () => {
      // Given I have a feature file "login.feature" with 3 scenarios
      const content = `Feature: Login

  Scenario: Valid login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in

  Scenario: Invalid password
    Given I am on the login page
    When I enter invalid password
    Then I should see an error

  Scenario: Locked account
    Given I am on the login page
    When I enter locked account credentials
    Then I should see locked message
`;
      const filePath = join(testDir, 'spec/features/login.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-scenario login "Invalid password"`
      const result = await deleteScenario({
        feature: 'login',
        scenario: 'Invalid password',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario "Invalid password" should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('Invalid password');

      // And the other 2 scenarios should remain
      expect(updatedContent).toContain('Valid login');
      expect(updatedContent).toContain('Locked account');

      // And the feature file should be valid Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();

      // And the output should show "Successfully deleted scenario 'Invalid password' from login.feature"
      expect(result.message).toContain('Successfully deleted');
      expect(result.message).toContain('Invalid password');
    });
  });

  describe('Scenario: Delete scenario from nested directory', () => {
    it('should handle full paths', async () => {
      // Given I have a feature file "spec/features/auth/login.feature"
      await mkdir(join(testDir, 'spec/features/auth'), { recursive: true });
      const content = `Feature: Login

  Scenario: Test scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/auth/login.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-scenario spec/features/auth/login.feature "Test scenario"`
      const result = await deleteScenario({
        feature: 'spec/features/auth/login.feature',
        scenario: 'Test scenario',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('Test scenario');

      // And the file should remain valid
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Delete last remaining scenario', () => {
    it('should preserve feature structure when deleting last scenario', async () => {
      // Given I have a feature file with only one scenario "Only scenario"
      const content = `Feature: Test Feature

  Background: Setup
    Given initial setup

  Scenario: Only scenario
    Given test
    When action
    Then result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-scenario test "Only scenario"`
      const result = await deleteScenario({
        feature: 'test',
        scenario: 'Only scenario',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain('Only scenario');

      // And the feature should have no scenarios
      expect(updatedContent).not.toContain('Scenario:');

      // And the background and feature header should remain intact
      expect(updatedContent).toContain('Feature: Test Feature');
      expect(updatedContent).toContain('Background: Setup');
    });
  });

  describe('Scenario: Attempt to delete non-existent scenario', () => {
    it('should return error for non-existent scenario', async () => {
      // Given I have a feature file "login.feature" with scenarios
      const content = `Feature: Login

  Scenario: Valid login
    Given test
`;
      const filePath = join(testDir, 'spec/features/login.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec delete-scenario login "Non-existent scenario"`
      const result = await deleteScenario({
        feature: 'login',
        scenario: 'Non-existent scenario',
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

  describe('Scenario: Delete scenario preserves background', () => {
    it('should keep background section intact', async () => {
      // Given I have a feature file with a Background section
      const content = `Feature: Test

  Background: User Story
    As a user
    I want to test
    So that it works

  Scenario: Test scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-scenario test "Test scenario"`
      const result = await deleteScenario({
        feature: 'test',
        scenario: 'Test scenario',
        cwd: testDir,
      });

      // Then the Background section should remain intact
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toContain('Background: User Story');
      expect(updatedContent).toContain('As a user');

      // And the scenario should be removed
      expect(updatedContent).not.toContain('Test scenario');
    });
  });

  describe('Scenario: Delete scenario preserves formatting', () => {
    it('should maintain file formatting', async () => {
      // Given I have a formatted feature file
      const content = `Feature: Test

  Scenario: First scenario
    Given test

  Scenario: Test scenario
    Given to delete

  Scenario: Last scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-scenario test "Test scenario"`
      const result = await deleteScenario({
        feature: 'test',
        scenario: 'Test scenario',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then the remaining file should preserve indentation
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toContain('  Scenario: First scenario');
      expect(updatedContent).toContain('    Given test');

      // And the remaining file should preserve spacing
      expect(updatedContent).toMatch(/Scenario: First scenario\s+Given test/s);

      // And the file should remain properly formatted
      expect(updatedContent).toContain('Feature: Test');
    });
  });

  describe('Scenario: Delete scenario with special characters in name', () => {
    it('should handle special characters', async () => {
      // Given I have a scenario named "User can't login with @special chars"
      const content = `Feature: Test

  Scenario: User can't login with @special chars
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec delete-scenario test "User can't login with @special chars"`
      const result = await deleteScenario({
        feature: 'test',
        scenario: "User can't login with @special chars",
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should be removed
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).not.toContain(
        "User can't login with @special chars"
      );
    });
  });

  describe('Scenario: Handle feature file with invalid syntax', () => {
    it('should return error for invalid Gherkin', async () => {
      // Given I have a feature file "broken.feature" with syntax errors
      const content = `Feature Test
  This is broken Gherkin
  Scenario: Some scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/broken.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec delete-scenario broken "Some scenario"`
      const result = await deleteScenario({
        feature: 'broken',
        scenario: 'Some scenario',
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
