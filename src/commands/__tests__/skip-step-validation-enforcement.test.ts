/**
 * Feature: spec/features/ai-agents-skip-docstring-step-validation-by-using-skip-step-validation-flag.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { linkCoverage } from '../link-coverage';

describe('Feature: AI agents skip docstring step validation by using --skip-step-validation flag', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-skip-validation-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Attempt to skip step validation for story work unit fails with strict error', () => {
    // @step Given I have a story work unit "AUTH-001" with a feature file
    // @step And the feature file has a scenario "Login with valid credentials"
    // @step And I have a test file "src/__tests__/auth.test.ts" with test code but missing step comments
    // @step When I run "fspec link-coverage user-login --scenario 'Login with valid credentials' --test-file src/__tests__/auth.test.ts --test-lines 10-20 --skip-step-validation"
    // @step Then the command should fail with exit code 1
    // @step And the error message should contain "skip-step-validation flag is ONLY allowed for task work units"
    // @step And the error message should contain "Story and bug work units require MANDATORY step validation"
    // @step And the error message should warn "Attempting to skip will require going back to fix docstrings when detected"
    // @step And the error message should NOT suggest using --skip-step-validation flag

    it('should reject --skip-step-validation for story work units with strict error', async () => {
      // Setup: Create story work unit with feature file
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');

      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Create work units file with story work unit
      const workUnitsData = {
        prefixes: { AUTH: { description: 'Authentication', workUnitIds: ['AUTH-001'] } },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            type: 'story',
            title: 'User Login',
            description: 'User login feature',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          }
        }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // Create feature file with @AUTH-001 tag
      const featureContent = `@AUTH-001 @authentication @cli
Feature: User Login

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

      await writeFile(join(featuresDir, 'user-login.feature'), featureContent);

      // Create coverage file
      const coverageData = {
        scenarios: [{ name: 'Login with valid credentials', testMappings: [] }],
        stats: { totalScenarios: 1, coveredScenarios: 0, coveragePercent: 0, testFiles: [], implFiles: [], totalLinesCovered: 0 }
      };
      await writeFile(join(featuresDir, 'user-login.feature.coverage'), JSON.stringify(coverageData, null, 2));

      // Create test file WITHOUT step comments
      const testContent = `describe('Login', () => {
  it('should log in successfully', () => {
    // Test implementation here
    expect(true).toBe(true);
  });
});`;
      await writeFile(join(testsDir, 'auth.test.ts'), testContent);

      // Attempt to link coverage with --skip-step-validation
      await expect(
        linkCoverage('user-login', {
          scenario: 'Login with valid credentials',
          testFile: 'src/__tests__/auth.test.ts',
          testLines: '10-20',
          skipStepValidation: true,
          cwd: testDir,
        })
      ).rejects.toThrow();

      try {
        await linkCoverage('user-login', {
          scenario: 'Login with valid credentials',
          testFile: 'src/__tests__/auth.test.ts',
          testLines: '10-20',
          skipStepValidation: true,
          cwd: testDir,
        });
      } catch (error: any) {
        const errorMessage = error.message;

        // Verify strict error messages
        expect(errorMessage).toContain('skip-step-validation flag is ONLY allowed for task work units');
        expect(errorMessage).toContain('Story and bug work units require MANDATORY step validation');
        expect(errorMessage).toContain('Attempting to skip step validation will be detected and require going back to fix docstrings');

        // Verify error message contains additional warnings
        expect(errorMessage).toContain('There is NO bypass for story and bug work units');
      }
    });
  });

  describe('Scenario: Attempt to skip step validation for bug work unit fails with strict error', () => {
    // @step Given I have a bug work unit "BUG-044" with a feature file
    // @step And the feature file has a scenario "Fix validation bypass"
    // @step And I have a test file "src/__tests__/validation.test.ts" with test code but missing step comments
    // @step When I run "fspec link-coverage ai-agents-skip-docstring-step-validation-by-using-skip-step-validation-flag --scenario 'Fix validation bypass' --test-file src/__tests__/validation.test.ts --test-lines 5-15 --skip-step-validation"
    // @step Then the command should fail with exit code 1
    // @step And the error message should contain "skip-step-validation flag is ONLY allowed for task work units"
    // @step And the error message should contain "Bug work units require MANDATORY step validation"

    it('should reject --skip-step-validation for bug work units with strict error', async () => {
      // Setup: Create bug work unit with feature file
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');

      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Create work units file with bug work unit
      const workUnitsData = {
        prefixes: { BUG: { description: 'Bug fixes', workUnitIds: ['BUG-044'] } },
        workUnits: {
          'BUG-044': {
            id: 'BUG-044',
            type: 'bug',
            title: 'Fix validation bypass',
            description: 'Fix validation bypass issue',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          }
        }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // Create feature file with @BUG-044 tag
      const featureContent = `@BUG-044 @validation @cli
Feature: Fix validation bypass

  Scenario: Fix validation bypass
    Given the system has a validation bypass
    When I apply the fix
    Then validation should be enforced`;

      await writeFile(join(featuresDir, 'fix-validation-bypass.feature'), featureContent);

      // Create coverage file
      const coverageData = {
        scenarios: [{ name: 'Fix validation bypass', testMappings: [] }],
        stats: { totalScenarios: 1, coveredScenarios: 0, coveragePercent: 0, testFiles: [], implFiles: [], totalLinesCovered: 0 }
      };
      await writeFile(join(featuresDir, 'fix-validation-bypass.feature.coverage'), JSON.stringify(coverageData, null, 2));

      // Create test file WITHOUT step comments
      const testContent = `describe('Validation', () => {
  it('should enforce validation', () => {
    expect(true).toBe(true);
  });
});`;
      await writeFile(join(testsDir, 'validation.test.ts'), testContent);

      // Attempt to link coverage with --skip-step-validation
      try {
        await linkCoverage('fix-validation-bypass', {
          scenario: 'Fix validation bypass',
          testFile: 'src/__tests__/validation.test.ts',
          testLines: '5-15',
          skipStepValidation: true,
          cwd: testDir,
        });
        throw new Error('Should have thrown error');
      } catch (error: any) {
        const errorMessage = error.message;

        // Verify strict error messages for bug work units
        expect(errorMessage).toContain('skip-step-validation flag is ONLY allowed for task work units');
        expect(errorMessage).toContain('Bug work units require MANDATORY step validation');
      }
    });
  });

  describe('Scenario: Skip step validation for task work unit succeeds', () => {
    // @step Given I have a task work unit "TASK-001" with a feature file
    // @step And the feature file has a scenario "Setup infrastructure"
    // @step And I have a test file "src/__tests__/setup.test.ts" with test code but missing step comments
    // @step When I run "fspec link-coverage infrastructure-setup --scenario 'Setup infrastructure' --test-file src/__tests__/setup.test.ts --test-lines 5-15 --skip-step-validation"
    // @step Then the command should succeed with exit code 0
    // @step And the output should confirm "Coverage linked successfully"
    // @step And the output should contain a warning "⚠️  Step validation skipped (task work unit)"

    it('should allow --skip-step-validation for task work units', async () => {
      // Setup: Create task work unit with feature file
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');

      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Create work units file with task work unit
      const workUnitsData = {
        prefixes: { TASK: { description: 'Tasks', workUnitIds: ['TASK-001'] } },
        workUnits: {
          'TASK-001': {
            id: 'TASK-001',
            type: 'task',
            title: 'Setup infrastructure',
            description: 'Infrastructure setup',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          }
        }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // Create feature file with @TASK-001 tag
      const featureContent = `@TASK-001 @infrastructure
Feature: Infrastructure Setup

  Scenario: Setup infrastructure
    Given I have infrastructure requirements
    When I run the setup script
    Then infrastructure should be configured`;

      await writeFile(join(featuresDir, 'infrastructure-setup.feature'), featureContent);

      // Create coverage file
      const coverageData = {
        scenarios: [{ name: 'Setup infrastructure', testMappings: [] }],
        stats: { totalScenarios: 1, coveredScenarios: 0, coveragePercent: 0, testFiles: [], implFiles: [], totalLinesCovered: 0 }
      };
      await writeFile(join(featuresDir, 'infrastructure-setup.feature.coverage'), JSON.stringify(coverageData, null, 2));

      // Create test file WITHOUT step comments
      const testContent = `describe('Setup', () => {
  it('should setup infrastructure', () => {
    expect(true).toBe(true);
  });
});`;
      await writeFile(join(testsDir, 'setup.test.ts'), testContent);

      // Link coverage with --skip-step-validation (should succeed for tasks)
      const result = await linkCoverage('infrastructure-setup', {
        scenario: 'Setup infrastructure',
        testFile: 'src/__tests__/setup.test.ts',
        testLines: '5-15',
        skipStepValidation: true,
        cwd: testDir,
      });

      // Verify success
      expect(result.success).toBe(true);
      expect(result.message).toContain('✓');

      // Verify warning about skipped validation
      if (result.warnings) {
        expect(result.warnings).toContain('Step validation skipped (task work unit)');
      }
    });
  });

  describe('Scenario: Story work unit with missing step comments receives strict system-reminder without skip option', () => {
    // @step Given I have a story work unit "AUTH-002" with a feature file
    // @step And the feature file has a scenario "Password reset flow" with steps
    // @step And I have a test file "src/__tests__/password-reset.test.ts" with test code
    // @step But the test file is missing step comments
    // @step When I run "fspec link-coverage password-reset --scenario 'Password reset flow' --test-file src/__tests__/password-reset.test.ts --test-lines 20-35"
    // @step Then the command should fail with exit code 1
    // @step And the output should contain a system-reminder showing missing step comments
    // @step And the system-reminder should show exact step text to add with "// @step" prefix
    // @step And the system-reminder should NOT mention --skip-step-validation flag
    // @step And the system-reminder should emphasize "Step validation is MANDATORY for story work units"

    it('should show strict system-reminder for story work units without skip option', async () => {
      // Setup
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');

      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Create work units file with story work unit
      const workUnitsData = {
        prefixes: { AUTH: { description: 'Authentication', workUnitIds: ['AUTH-002'] } },
        workUnits: {
          'AUTH-002': {
            id: 'AUTH-002',
            type: 'story',
            title: 'Password reset',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          }
        }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      const featureContent = `@AUTH-002
Feature: Password Reset

  Scenario: Password reset flow
    Given I am on the password reset page
    When I enter my email address
    Then I should receive a password reset email`;

      await writeFile(join(featuresDir, 'password-reset.feature'), featureContent);

      const coverageData = {
        scenarios: [{ name: 'Password reset flow', testMappings: [] }],
        stats: { totalScenarios: 1, coveredScenarios: 0, coveragePercent: 0, testFiles: [], implFiles: [], totalLinesCovered: 0 }
      };
      await writeFile(join(featuresDir, 'password-reset.feature.coverage'), JSON.stringify(coverageData, null, 2));

      const testContent = `describe('Password reset', () => {
  it('should reset password', () => {
    expect(true).toBe(true);
  });
});`;
      await writeFile(join(testsDir, 'password-reset.test.ts'), testContent);

      // Attempt to link without skip flag (should fail with strict reminder)
      try {
        await linkCoverage('password-reset', {
          scenario: 'Password reset flow',
          testFile: 'src/__tests__/password-reset.test.ts',
          testLines: '20-35',
          cwd: testDir,
        });
        throw new Error('Should have thrown error');
      } catch (error: any) {
        const errorMessage = error.message;

        // Verify system-reminder is present
        expect(errorMessage).toContain('<system-reminder>');
        expect(errorMessage).toContain('STEP VALIDATION FAILED');

        // Verify shows exact steps to add
        expect(errorMessage).toContain('// @step Given I am on the password reset page');
        expect(errorMessage).toContain('// @step When I enter my email address');
        expect(errorMessage).toContain('// @step Then I should receive a password reset email');

        // Verify does NOT mention skip flag
        expect(errorMessage).not.toContain('--skip-step-validation');
        expect(errorMessage).not.toContain('To override validation');

        // Verify emphasizes mandatory validation
        expect(errorMessage).toContain('MANDATORY');
      }
    });
  });

  describe('Scenario: Bug work unit with missing step comments receives strict system-reminder without skip option', () => {
    // @step Given I have a bug work unit "BUG-045" with a feature file
    // @step And the feature file has a scenario "Fix login timeout" with steps
    // @step And I have a test file "src/__tests__/login-timeout.test.ts" with test code
    // @step But the test file is missing step comments
    // @step When I run "fspec link-coverage fix-login-timeout --scenario 'Fix login timeout' --test-file src/__tests__/login-timeout.test.ts --test-lines 10-25"
    // @step Then the command should fail with exit code 1
    // @step And the output should contain a system-reminder showing missing step comments
    // @step And the system-reminder should show exact step text to add with "// @step" prefix
    // @step And the system-reminder should NOT mention --skip-step-validation flag
    // @step And the system-reminder should emphasize "Step validation is MANDATORY for bug work units"

    it('should show strict system-reminder for bug work units without skip option', async () => {
      // Setup
      const featuresDir = join(testDir, 'spec', 'features');
      const testsDir = join(testDir, 'src', '__tests__');
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');

      await mkdir(featuresDir, { recursive: true });
      await mkdir(testsDir, { recursive: true });
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Create work units file with bug work unit
      const workUnitsData = {
        prefixes: { BUG: { description: 'Bug fixes', workUnitIds: ['BUG-045'] } },
        workUnits: {
          'BUG-045': {
            id: 'BUG-045',
            type: 'bug',
            title: 'Fix login timeout',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          }
        }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      const featureContent = `@BUG-045
Feature: Fix Login Timeout

  Scenario: Fix login timeout
    Given the login service is slow to respond
    When the timeout threshold is exceeded
    Then the user should see a timeout error`;

      await writeFile(join(featuresDir, 'fix-login-timeout.feature'), featureContent);

      const coverageData = {
        scenarios: [{ name: 'Fix login timeout', testMappings: [] }],
        stats: { totalScenarios: 1, coveredScenarios: 0, coveragePercent: 0, testFiles: [], implFiles: [], totalLinesCovered: 0 }
      };
      await writeFile(join(featuresDir, 'fix-login-timeout.feature.coverage'), JSON.stringify(coverageData, null, 2));

      const testContent = `describe('Login timeout', () => {
  it('should handle timeout', () => {
    expect(true).toBe(true);
  });
});`;
      await writeFile(join(testsDir, 'login-timeout.test.ts'), testContent);

      // Attempt to link without skip flag (should fail with strict reminder)
      try {
        await linkCoverage('fix-login-timeout', {
          scenario: 'Fix login timeout',
          testFile: 'src/__tests__/login-timeout.test.ts',
          testLines: '10-25',
          cwd: testDir,
        });
        throw new Error('Should have thrown error');
      } catch (error: any) {
        const errorMessage = error.message;

        // Verify system-reminder is present
        expect(errorMessage).toContain('<system-reminder>');
        expect(errorMessage).toContain('STEP VALIDATION FAILED');

        // Verify does NOT mention skip flag
        expect(errorMessage).not.toContain('--skip-step-validation');

        // Verify emphasizes mandatory validation for bugs
        expect(errorMessage).toContain('MANDATORY');
      }
    });
  });
});
