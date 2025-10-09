import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { updateStep } from '../update-step';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

describe('Feature: Update Step in Scenario', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-update-step');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Update step text only', () => {
    it('should update step text while preserving keyword', async () => {
      // Given I have a step "Given I am logged in"
      const content = `Feature: Login

  Scenario: Login test
    Given I am logged in
    When I click button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/login.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step login "Login test" "Given I am logged in" --text="Given I am authenticated"`
      const result = await updateStep({
        feature: 'login',
        scenario: 'Login test',
        currentStep: 'Given I am logged in',
        text: 'Given I am authenticated',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step should be "Given I am authenticated"
      expect(updatedContent).toContain('Given I am authenticated');
      expect(updatedContent).not.toContain('Given I am logged in');

      // And the keyword should remain "Given"
      expect(updatedContent).toMatch(/Given I am authenticated/);

      // And other steps should be preserved
      expect(updatedContent).toContain('When I click button');
      expect(updatedContent).toContain('Then I see result');

      // And the feature file should be valid Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();

      // And the output should show "Successfully updated step"
      expect(result.message).toContain('Successfully updated step');
    });
  });

  describe('Scenario: Update step keyword only', () => {
    it('should update keyword while preserving text', async () => {
      // Given I have a step "Given I am on the page"
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on the page
    Then I see content
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "Given I am on the page" --keyword="When"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'Given I am on the page',
        keyword: 'When',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step should be "When I am on the page"
      expect(updatedContent).toContain('When I am on the page');

      // And the step text should remain "I am on the page"
      expect(updatedContent).toMatch(/When I am on the page/);
    });
  });

  describe('Scenario: Update both step text and keyword', () => {
    it('should update both keyword and text', async () => {
      // Given I have a step "Given I click the button"
      const content = `Feature: Test

  Scenario: Test scenario
    Given I click the button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "Given I click the button" --keyword="When" --text="When I submit the form"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'Given I click the button',
        keyword: 'When',
        text: 'When I submit the form',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step should be "When I submit the form"
      expect(updatedContent).toContain('When I submit the form');

      // And the old step should not exist
      expect(updatedContent).not.toContain('Given I click the button');
    });
  });

  describe('Scenario: Update step in specific scenario when multiple scenarios have same step', () => {
    it('should update step in specified scenario only', async () => {
      // Given I have two scenarios "First" and "Second"
      // And both scenarios have step "Given I am logged in"
      const content = `Feature: Test

  Scenario: First
    Given I am logged in
    When I do something

  Scenario: Second
    Given I am logged in
    When I do other thing
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "First" "Given I am logged in" --text="Given I have authenticated"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'First',
        currentStep: 'Given I am logged in',
        text: 'Given I have authenticated',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step in "First" scenario should be updated
      const firstScenarioSection = updatedContent.split('Scenario: Second')[0];
      expect(firstScenarioSection).toContain('Given I have authenticated');

      // And the step in "Second" scenario should remain unchanged
      const secondScenarioSection = updatedContent.split('Scenario: Second')[1];
      expect(secondScenarioSection).toContain('Given I am logged in');
    });
  });

  describe('Scenario: Update step preserves position in scenario', () => {
    it('should maintain step order', async () => {
      // Given I have a scenario with steps in order: "Given A", "When B", "Then C"
      const content = `Feature: Test

  Scenario: Test scenario
    Given A
    When B
    Then C
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "When B" --text="When B updated"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'When B',
        text: 'When B updated',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the steps should remain in order: "Given A", "When B updated", "Then C"
      const lines = updatedContent.split('\n');
      const stepLines = lines.filter(l => l.trim().match(/^(Given|When|Then)/));
      expect(stepLines[0]).toContain('Given A');
      expect(stepLines[1]).toContain('When B updated');
      expect(stepLines[2]).toContain('Then C');
    });
  });

  describe('Scenario: Update step with special characters', () => {
    it('should handle special characters', async () => {
      // Given I have a step "Given I can login"
      const content = `Feature: Test

  Scenario: Test scenario
    Given I can login
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "Given I can login" --text="Given I can't login with @invalid credentials"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'Given I can login',
        text: "Given I can't login with @invalid credentials",
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step should contain apostrophe and special characters
      expect(updatedContent).toContain(
        "Given I can't login with @invalid credentials"
      );

      // And the feature file should be valid Gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      expect(() => parser.parse(updatedContent)).not.toThrow();
    });
  });

  describe('Scenario: Attempt to update non-existent step', () => {
    it('should return error for non-existent step', async () => {
      // Given I have a scenario with 3 steps
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on page
    When I click button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec update-step test "Test scenario" "Given non-existent step" --text="New text"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'Given non-existent step',
        text: 'New text',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Step 'Given non-existent step' not found"
      expect(result.error).toMatch(/not found/i);
      expect(result.error).toContain('Given non-existent step');

      // And the file should remain unchanged
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Update And step to Given step', () => {
    it('should change And keyword to Given', async () => {
      // Given I have a step "And I enter password"
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on page
    And I enter password
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "And I enter password" --keyword="Given"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'And I enter password',
        keyword: 'Given',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the step should be "Given I enter password"
      expect(updatedContent).toContain('Given I enter password');
      expect(updatedContent).not.toMatch(/And I enter password/);
    });
  });

  describe('Scenario: Update step preserves indentation', () => {
    it('should maintain proper indentation', async () => {
      // Given I have a properly indented step
      const content = `Feature: Test

  Scenario: Test scenario
    Given I am on page
    When I click button
    Then I see result
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "Given I am on page" --text="Given I am on the login page"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'Given I am on page',
        text: 'Given I am on the login page',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');

      // And the updated step should maintain proper indentation
      expect(updatedContent).toContain('    Given I am on the login page');
    });
  });

  describe('Scenario: Handle scenario not found', () => {
    it('should return error when scenario does not exist', async () => {
      // Given I have a feature file
      const content = `Feature: Test

  Scenario: Existing scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);
      const originalContent = content;

      // When I run `fspec update-step test "Non-existent scenario" "Given test" --text="New text"`
      const result = await updateStep({
        feature: 'test',
        scenario: 'Non-existent scenario',
        currentStep: 'Given test',
        text: 'New text',
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

  describe('Scenario: Require at least one update option', () => {
    it('should return error when no updates specified', async () => {
      // Given I have a step "Given test"
      const content = `Feature: Test

  Scenario: Test scenario
    Given test
`;
      const filePath = join(testDir, 'spec/features/test.feature');
      await writeFile(filePath, content);

      // When I run `fspec update-step test "Test scenario" "Given test"` without --text or --keyword
      const result = await updateStep({
        feature: 'test',
        scenario: 'Test scenario',
        currentStep: 'Given test',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "No updates specified. Use --text and/or --keyword"
      expect(result.error).toMatch(/no updates specified/i);
      expect(result.error).toMatch(/--text|--keyword/i);
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

      // When I run `fspec update-step broken "Some scenario" "Given test" --text="New text"`
      const result = await updateStep({
        feature: 'broken',
        scenario: 'Some scenario',
        currentStep: 'Given test',
        text: 'New text',
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
