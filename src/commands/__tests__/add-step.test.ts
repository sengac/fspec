import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { addStep } from '../add-step';

describe('Feature: Add Step to Existing Scenario', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-add-step');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add Given step to scenario', () => {
    it('should add Given step to scenario', async () => {
      // Given I have a feature file "spec/features/login.feature" with scenario "User login"
      const featureContent = `Feature: Login

  Scenario: User login
    Given I have an account
`;
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        featureContent
      );

      // When I run `fspec add-step login "User login" given "I am on the login page"`
      const result = await addStep(
        'login',
        'User login',
        'given',
        'I am on the login page',
        {
          cwd: testDir,
        }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should contain "Given I am on the login page"
      const updatedContent = await readFile(
        join(testDir, 'spec/features/login.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('Given I am on the login page');

      // And the file should remain valid Gherkin syntax
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Add When step to scenario', () => {
    it('should add When step with proper indentation', async () => {
      // Given I have a feature file with scenario "Submit form"
      const featureContent = `Feature: My Feature

  Scenario: Submit form
    Given I am on the form page
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Submit form" when "I click the submit button"`
      const result = await addStep(
        'my-feature',
        'Submit form',
        'when',
        'I click the submit button',
        {
          cwd: testDir,
        }
      );

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the scenario should contain "When I click the submit button"
      expect(updatedContent).toContain('When I click the submit button');

      // And the step should be properly indented
      expect(updatedContent).toContain('    When I click the submit button');
    });
  });

  describe('Scenario: Add Then step to scenario', () => {
    it('should add Then step to scenario', async () => {
      // Given I have a feature file with scenario "Validation"
      const featureContent = `Feature: My Feature

  Scenario: Validation
    Given test
    When test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Validation" then "I should see a success message"`
      await addStep(
        'my-feature',
        'Validation',
        'then',
        'I should see a success message',
        {
          cwd: testDir,
        }
      );

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the scenario should contain "Then I should see a success message"
      expect(updatedContent).toContain('Then I should see a success message');
    });
  });

  describe('Scenario: Add And step to scenario', () => {
    it('should add And step to scenario', async () => {
      // Given I have a feature file with scenario "Multiple steps"
      const featureContent = `Feature: My Feature

  Scenario: Multiple steps
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Multiple steps" and "I fill in the email field"`
      await addStep(
        'my-feature',
        'Multiple steps',
        'and',
        'I fill in the email field',
        {
          cwd: testDir,
        }
      );

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the scenario should contain "And I fill in the email field"
      expect(updatedContent).toContain('And I fill in the email field');
    });
  });

  describe('Scenario: Add But step to scenario', () => {
    it('should add But step to scenario', async () => {
      // Given I have a feature file with scenario "Edge case"
      const featureContent = `Feature: My Feature

  Scenario: Edge case
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Edge case" but "I should not see an error"`
      await addStep(
        'my-feature',
        'Edge case',
        'but',
        'I should not see an error',
        {
          cwd: testDir,
        }
      );

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the scenario should contain "But I should not see an error"
      expect(updatedContent).toContain('But I should not see an error');
    });
  });

  describe('Scenario: Add multiple steps to same scenario', () => {
    it('should add steps in order', async () => {
      // Given I have a feature file with scenario "Complex workflow" with 1 step
      const featureContent = `Feature: My Feature

  Scenario: Complex workflow
    Given I am logged in
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Complex workflow" when "I enter my password"`
      await addStep(
        'my-feature',
        'Complex workflow',
        'when',
        'I enter my password',
        {
          cwd: testDir,
        }
      );

      // And I run `fspec add-step my-feature "Complex workflow" then "I should be logged in"`
      await addStep(
        'my-feature',
        'Complex workflow',
        'then',
        'I should be logged in',
        {
          cwd: testDir,
        }
      );

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the scenario should have 3 steps total
      const stepMatches = updatedContent.match(
        /^\s+(Given|When|Then|And|But) /gm
      );
      expect(stepMatches?.length).toBe(3);

      // And all steps should be in the order they were added
      const givenIndex = updatedContent.indexOf('Given I am logged in');
      const whenIndex = updatedContent.indexOf('When I enter my password');
      const thenIndex = updatedContent.indexOf('Then I should be logged in');
      expect(givenIndex).toBeLessThan(whenIndex);
      expect(whenIndex).toBeLessThan(thenIndex);

      // And the file should remain valid Gherkin syntax
      const result = await addStep(
        'my-feature',
        'Complex workflow',
        'and',
        'test',
        {
          cwd: testDir,
          dryRun: true,
        }
      );
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Preserve indentation when adding step', () => {
    it('should match existing indentation', async () => {
      // Given I have a scenario with 4-space indented steps
      const featureContent = `Feature: My Feature

  Scenario: Test
    Given existing step
    When another step
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Test" given "new step"`
      await addStep('my-feature', 'Test', 'given', 'new step', {
        cwd: testDir,
      });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the new step should have the same indentation as existing steps
      const lines = updatedContent.split('\n');
      const stepLines = lines.filter(line =>
        /^\s+(Given|When|Then) /.test(line)
      );
      const indentations = stepLines.map(
        line => line.match(/^(\s+)/)?.[1].length
      );
      const allSameIndent = indentations.every(
        indent => indent === indentations[0]
      );
      expect(allSameIndent).toBe(true);

      // And the indentation should be 4 spaces from feature level
      expect(updatedContent).toContain('    Given new step');
    });
  });

  describe('Scenario: Handle feature file not found', () => {
    it('should show error and suggest create-feature', async () => {
      // Given there is no feature file "spec/features/missing.feature"
      // (file does not exist)

      // When I run `fspec add-step missing "Scenario" given "step"`
      const result = await addStep('missing', 'Scenario', 'given', 'step', {
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

  describe('Scenario: Handle scenario not found', () => {
    it('should show error and list available scenarios', async () => {
      // Given I have a feature file "spec/features/test.feature"
      const featureContent = `Feature: Test

  Scenario: Existing scenario
    Given test

  Scenario: Another scenario
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // And the file does not have a scenario named "Missing scenario"
      // When I run `fspec add-step test "Missing scenario" given "step"`
      const result = await addStep(
        'test',
        'Missing scenario',
        'given',
        'step',
        { cwd: testDir }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show error that scenario was not found
      expect(result.error).toContain('not found') ||
        expect(result.error).toContain('Missing scenario');

      // And the output should list available scenarios
      expect(result.suggestion).toContain('Existing scenario');
      expect(result.suggestion).toContain('Another scenario');

      // And the file should not be modified
      const fileContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(fileContent).toBe(featureContent);
    });
  });

  describe('Scenario: Handle invalid feature file syntax', () => {
    it('should show error and not modify file', async () => {
      // Given I have a feature file with invalid Gherkin syntax
      const invalidContent = 'This is not valid Gherkin';
      await writeFile(
        join(testDir, 'spec/features/broken.feature'),
        invalidContent
      );

      // When I run `fspec add-step broken "Scenario" given "step"`
      const result = await addStep('broken', 'Scenario', 'given', 'step', {
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

  describe('Scenario: Handle invalid step type', () => {
    it('should show error and list valid types', async () => {
      // Given I have a feature file with scenario "Test"
      const featureContent = `Feature: Test

  Scenario: Test
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Test" invalid "step text"`
      const result = await addStep(
        'my-feature',
        'Test',
        'invalid',
        'step text',
        {
          cwd: testDir,
        }
      );

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show error about invalid step type
      expect(result.error).toContain('invalid') ||
        expect(result.error).toContain('step type');

      // And the output should list valid step types: given, when, then, and, but
      expect(result.suggestion).toContain('given');
      expect(result.suggestion).toContain('when');
      expect(result.suggestion).toContain('then');
      expect(result.suggestion).toContain('and');
      expect(result.suggestion).toContain('but');
    });
  });

  describe('Scenario: Use feature name without extension', () => {
    it('should find feature file by name', async () => {
      // Given I have a feature file "spec/features/user-auth.feature" with scenario "Login"
      const featureContent = `Feature: User Auth

  Scenario: Login
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/user-auth.feature'),
        featureContent
      );

      // When I run `fspec add-step user-auth "Login" given "I am logged out"`
      const result = await addStep(
        'user-auth',
        'Login',
        'given',
        'I am logged out',
        {
          cwd: testDir,
        }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should contain the new step
      const updatedContent = await readFile(
        join(testDir, 'spec/features/user-auth.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('Given I am logged out');
    });
  });

  describe('Scenario: Use full file path', () => {
    it('should accept full file path', async () => {
      // Given I have a feature file "spec/features/checkout.feature" with scenario "Payment"
      const featureContent = `Feature: Checkout

  Scenario: Payment
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/checkout.feature'),
        featureContent
      );

      // When I run `fspec add-step spec/features/checkout.feature "Payment" when "I enter card details"`
      const result = await addStep(
        'spec/features/checkout.feature',
        'Payment',
        'when',
        'I enter card details',
        { cwd: testDir }
      );

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the scenario should contain the new step
      const updatedContent = await readFile(
        join(testDir, 'spec/features/checkout.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('When I enter card details');
    });
  });

  describe('Scenario: Case-insensitive step type', () => {
    it('should normalize step type to proper case', async () => {
      // Given I have a feature file with scenario "Test"
      const featureContent = `Feature: Test

  Scenario: Test
    When test
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Test" GIVEN "step text"`
      const result = await addStep('my-feature', 'Test', 'GIVEN', 'step text', {
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // And the step should be formatted as "Given step text"
      expect(updatedContent).toContain('Given step text');
    });
  });

  describe('Scenario: Handle scenario with data table', () => {
    it('should add step before data table', async () => {
      // Given I have a scenario with a data table at the end
      const featureContent = `Feature: Test

  Scenario: Scenario
    Given I have users:
      | name  | email         |
      | Alice | alice@test.com |
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Scenario" and "another step"`
      await addStep('my-feature', 'Scenario', 'and', 'another step', {
        cwd: testDir,
      });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the new step should be added before the data table
      const stepIndex = updatedContent.indexOf('And another step');
      const tableIndex = updatedContent.indexOf('| name');
      expect(stepIndex).toBeLessThan(tableIndex);

      // And the data table should remain at the end
      expect(updatedContent).toContain('| Alice | alice@test.com |');

      // And the file should remain valid Gherkin syntax
      const result = await addStep('my-feature', 'Scenario', 'and', 'test', {
        cwd: testDir,
        dryRun: true,
      });
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Handle scenario with doc string', () => {
    it('should add step before doc string', async () => {
      // Given I have a scenario with a doc string at the end
      const featureContent = `Feature: Test

  Scenario: Scenario
    Given I have a document
      """
      This is a doc string
      """
`;
      await writeFile(
        join(testDir, 'spec/features/my-feature.feature'),
        featureContent
      );

      // When I run `fspec add-step my-feature "Scenario" and "another step"`
      await addStep('my-feature', 'Scenario', 'and', 'another step', {
        cwd: testDir,
      });

      const updatedContent = await readFile(
        join(testDir, 'spec/features/my-feature.feature'),
        'utf-8'
      );

      // Then the new step should be added before the doc string
      const stepIndex = updatedContent.indexOf('And another step');
      const docStringIndex = updatedContent.indexOf('"""');
      expect(stepIndex).toBeLessThan(docStringIndex);

      // And the doc string should remain at the end
      expect(updatedContent).toContain('This is a doc string');

      // And the file should remain valid Gherkin syntax
      const result = await addStep('my-feature', 'Scenario', 'and', 'test', {
        cwd: testDir,
        dryRun: true,
      });
      expect(result.valid).toBe(true);
    });
  });
});
