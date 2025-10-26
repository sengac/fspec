/**
 * Feature: spec/features/docstring-based-test-to-scenario-linking-system.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { linkCoverage } from '../link-coverage';
import {
  extractStepComments,
  validateSteps,
  formatValidationError,
} from '../../utils/step-validation';

describe('Feature: Docstring-based test-to-scenario linking system', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-step-validation-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Validate test with @step prefix comments matches feature steps', () => {
    it('should succeed with step validation passing when all @step comments match', async () => {
      // Given I have a feature file with scenario 'Login' containing steps 'Given I am on the login page', 'When I click the login button', 'Then I should be logged in'
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });

      const featureContent = `@authentication
Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I click the login button
    Then I should be logged in`;

      await writeFile(join(featuresDir, 'user-login.feature'), featureContent);

      // Coverage file
      const coverageData = {
        scenarios: [{ name: 'Login', testMappings: [] }],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 0,
          coveragePercent: 0,
          testFiles: [],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };
      await writeFile(
        join(featuresDir, 'user-login.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // And I have a test file with comments '// @step Given I am on the login page', '// @step When I click the login button', '// @step Then I should be logged in'
      const testContent = `/**
 * Feature: spec/features/user-login.feature
 */
describe('Login', () => {
  it('should log in successfully', () => {
    // @step Given I am on the login page
    // @step When I click the login button
    // @step Then I should be logged in
  });
});`;
      await writeFile(join(testsDir, 'auth.test.ts'), testContent);

      // When I run 'fspec link-coverage user-login --scenario Login --test-file src/__tests__/auth.test.ts --test-lines 10-25'
      const result = await linkCoverage('user-login', {
        scenario: 'Login',
        testFile: 'src/__tests__/auth.test.ts',
        testLines: '10-25',
        cwd: testDir,
      });

      // Then the command should succeed with step validation passing
      expect(result.success).toBe(true);

      // And the coverage file should be updated with the test mapping
      expect(result.message).toContain('✓');
    });
  });

  describe('Scenario: Fail validation when test missing step comments with helpful system-reminder', () => {
    it('should fail with exit code 1 and show system-reminder with exact step text', async () => {
      // Given I have a feature file with scenario containing step 'When I click the login button'
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });

      const featureContent = `@authentication
Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I click the login button
    Then I should be logged in`;

      await writeFile(join(featuresDir, 'user-login.feature'), featureContent);

      const coverageData = {
        scenarios: [{ name: 'Login', testMappings: [] }],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 0,
          coveragePercent: 0,
          testFiles: [],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };
      await writeFile(
        join(featuresDir, 'user-login.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // And I have a test file that is missing the '// @step When I click the login button' comment
      const testContent = `/**
 * Feature: spec/features/user-login.feature
 */
describe('Login', () => {
  it('should log in successfully', () => {
    // @step Given I am on the login page
    // MISSING: // @step When I click the login button
    // @step Then I should be logged in
  });
});`;
      await writeFile(join(testsDir, 'auth.test.ts'), testContent);

      // When I run 'fspec link-coverage user-login --scenario Login --test-file src/__tests__/auth.test.ts --test-lines 10-25'
      // Then the command should fail with exit code 1
      let errorThrown = false;
      let errorMessage = '';

      try {
        await linkCoverage('user-login', {
          scenario: 'Login',
          testFile: 'src/__tests__/auth.test.ts',
          testLines: '10-25',
          cwd: testDir,
        });
      } catch (error: any) {
        errorThrown = true;
        errorMessage = error.message;
      }

      expect(errorThrown).toBe(true);

      // And a <system-reminder> should show the exact step text to add: '// @step When I click the login button'
      expect(errorMessage).toContain('<system-reminder>');
      expect(errorMessage).toContain('When I click the login button');
      expect(errorMessage).toContain('// @step');

      // And the reminder should emphasize MANDATORY validation for story work units (no skip option)
      expect(errorMessage).toContain('MANDATORY');
      expect(errorMessage).not.toContain('--skip-step-validation');
    });
  });

  describe('Scenario: Match parameterized steps using hybrid similarity algorithm', () => {
    it('should match step with parameter using hybrid similarity threshold 0.75', async () => {
      // Given I have a feature file with parameterized step 'Given I have {int} items in my cart'
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });

      const featureContent = `@shopping
Feature: Shopping Cart

  Scenario: Add items
    Given I have {int} items in my cart
    When I proceed to checkout
    Then I should see the items in my order`;

      await writeFile(
        join(featuresDir, 'shopping-cart.feature'),
        featureContent
      );

      const coverageData = {
        scenarios: [{ name: 'Add items', testMappings: [] }],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 0,
          coveragePercent: 0,
          testFiles: [],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };
      await writeFile(
        join(featuresDir, 'shopping-cart.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // And I have a test file with comment '// @step Given I have 5 items in my cart'
      const testContent = `/**
 * Feature: spec/features/shopping-cart.feature
 */
describe('Add items', () => {
  it('should add items to cart', () => {
    // @step Given I have 5 items in my cart
    // @step When I proceed to checkout
    // @step Then I should see the items in my order
  });
});`;
      await writeFile(join(testsDir, 'cart.test.ts'), testContent);

      // When I run 'fspec link-coverage shopping-cart --scenario Add-items --test-file src/__tests__/cart.test.ts --test-lines 20-35'
      const result = await linkCoverage('shopping-cart', {
        scenario: 'Add items',
        testFile: 'src/__tests__/cart.test.ts',
        testLines: '20-35',
        cwd: testDir,
      });

      // Then the step should match using hybrid similarity with threshold 0.75
      // And the command should succeed with step validation passing
      expect(result.success).toBe(true);
      expect(result.message).toContain('✓');
    });
  });

  describe('Scenario: Support backward compatibility with plain step comments without @step prefix', () => {
    it('should match steps even without @step prefix for backward compatibility', async () => {
      // Given I have a legacy test file with plain comments '// Given I am logged in' (no @step prefix)
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });

      const featureContent = `@session
Feature: User Session

  Scenario: Session management
    Given I am logged in
    When I navigate to my profile
    Then I should see my user details`;

      await writeFile(
        join(featuresDir, 'user-session.feature'),
        featureContent
      );

      const coverageData = {
        scenarios: [{ name: 'Session management', testMappings: [] }],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 0,
          coveragePercent: 0,
          testFiles: [],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };
      await writeFile(
        join(featuresDir, 'user-session.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // And I have a feature file with step 'Given I am logged in'
      const testContent = `/**
 * Feature: spec/features/user-session.feature
 */
describe('Session management', () => {
  it('should manage user session', () => {
    // Given I am logged in
    // When I navigate to my profile
    // Then I should see my user details
  });
});`;
      await writeFile(join(testsDir, 'session.test.ts'), testContent);

      // When I run 'fspec link-coverage user-session --scenario Session-management --test-file src/__tests__/session.test.ts --test-lines 15-30'
      const result = await linkCoverage('user-session', {
        scenario: 'Session management',
        testFile: 'src/__tests__/session.test.ts',
        testLines: '15-30',
        cwd: testDir,
      });

      // Then the step should match even without @step prefix
      // And the command should succeed with step validation passing
      expect(result.success).toBe(true);
      expect(result.message).toContain('✓');
    });
  });

  describe('Scenario: Display step-level validation status in show-coverage output', () => {
    it('should display step-level validation with matched and missing indicators', async () => {
      // Given I have a feature with scenario 'Login' containing 3 steps
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });

      const featureContent = `@authentication
Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I click the login button
    Then I should be logged in`;

      await writeFile(join(featuresDir, 'user-login.feature'), featureContent);

      // Coverage file with test mapping
      const coverageData = {
        scenarios: [
          {
            name: 'Login',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '10-20',
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: [],
          totalLinesCovered: 10,
        },
      };
      await writeFile(
        join(featuresDir, 'user-login.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // And the test file has matching comments for 2 steps but is missing comment for 'Then I should be logged in'
      const testContent = `/**
 * Feature: spec/features/user-login.feature
 */
describe('Login', () => {
  it('should log in successfully', () => {
    // @step Given I am on the login page
    // @step When I click the login button
    // MISSING: // @step Then I should be logged in
  });
});`;
      await writeFile(join(testsDir, 'auth.test.ts'), testContent);

      // When I run validation on the test file
      const featureSteps = [
        'Given I am on the login page',
        'When I click the login button',
        'Then I should be logged in',
      ];

      const validationResult = validateSteps(featureSteps, testContent);

      // Then validation should show 2 matched and 1 missing
      expect(validationResult.matches.length).toBe(3);
      expect(validationResult.matches[0].matched).toBe(true); // Given
      expect(validationResult.matches[1].matched).toBe(true); // When
      expect(validationResult.matches[2].matched).toBe(false); // Then - missing

      // And missing steps should be identified
      expect(validationResult.missingSteps.length).toBe(1);
      expect(validationResult.missingSteps[0]).toContain(
        'Then I should be logged in'
      );
    });
  });
});
