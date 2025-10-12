import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { getScenarios } from '../get-scenarios';

describe('Feature: Get Scenarios by Tag', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-get-scenarios');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Get scenarios from features with single tag', () => {
    it('should show all scenarios from features with tag', async () => {
      // Given I have 3 feature files tagged with @phase1
      for (let i = 1; i <= 3; i++) {
        const content = `@phase1
Feature: Feature ${i}

  Scenario: First scenario
    Given test

  Scenario: Second scenario
    Given test
`;
        await writeFile(
          join(testDir, 'spec/features', `f${i}.feature`),
          content
        );
      }

      // When I run `fspec get-scenarios --tag=@phase1`
      const result = await getScenarios({ tags: ['@phase1'], cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show 6 scenarios total
      expect(result.scenarios.length).toBe(6);

      // And each scenario should show feature path, scenario name, and line number
      expect(result.scenarios[0]).toHaveProperty('feature');
      expect(result.scenarios[0]).toHaveProperty('name');
      expect(result.scenarios[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Get scenarios with multiple tags (AND logic)', () => {
    it('should only show scenarios from features with all tags', async () => {
      // Given I have feature files with tags @phase1 @critical
      await writeFile(
        join(testDir, 'spec/features/both.feature'),
        `@phase1 @critical
Feature: Both Tags

  Scenario: Test
    Given test
`
      );

      // And I have feature files with only @phase1
      await writeFile(
        join(testDir, 'spec/features/phase1-only.feature'),
        `@phase1
Feature: Phase 1 Only

  Scenario: Test
    Given test
`
      );

      // And I have feature files with only @critical
      await writeFile(
        join(testDir, 'spec/features/critical-only.feature'),
        `@critical
Feature: Critical Only

  Scenario: Test
    Given test
`
      );

      // When I run `fspec get-scenarios --tag=@phase1 --tag=@critical`
      const result = await getScenarios({
        tags: ['@phase1', '@critical'],
        cwd: testDir,
      });

      // Then the output should only show scenarios from features with both tags
      expect(result.scenarios.length).toBe(1);
      expect(result.scenarios[0].feature).toContain('both.feature');

      // And features with only one of the tags should be excluded
      expect(
        result.scenarios.every(s => !s.feature.includes('phase1-only'))
      ).toBe(true);
      expect(
        result.scenarios.every(s => !s.feature.includes('critical-only'))
      ).toBe(true);
    });
  });

  describe('Scenario: Get scenarios when no features match tags', () => {
    it('should show message when no scenarios found', async () => {
      // Given I have feature files without @deprecated tag
      await writeFile(
        join(testDir, 'spec/features/active.feature'),
        `@active
Feature: Active

  Scenario: Test
    Given test
`
      );

      // When I run `fspec get-scenarios --tag=@deprecated`
      const result = await getScenarios({
        tags: ['@deprecated'],
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show "No scenarios found matching tags: @deprecated"
      expect(result.scenarios.length).toBe(0);
      expect(result.message).toContain('No scenarios found');
      expect(result.message).toContain('@deprecated');
    });
  });

  describe('Scenario: Show scenario details with line numbers', () => {
    it('should show feature path, scenario name, and line number', async () => {
      // Given I have a feature file "spec/features/login.feature" tagged @auth
      const content = `@auth
Feature: Login

  Scenario: Successful login
    Given test

  Scenario: Failed login
    Given test
`;
      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec get-scenarios --tag=@auth`
      const result = await getScenarios({ tags: ['@auth'], cwd: testDir });

      // Then the output should show "login.feature:15 - Successful login"
      const successScenario = result.scenarios.find(
        s => s.name === 'Successful login'
      );
      expect(successScenario).toBeDefined();
      expect(successScenario!.feature).toContain('login.feature');
      expect(successScenario!.line).toBeGreaterThan(0);

      // And the output should show "login.feature:25 - Failed login"
      const failScenario = result.scenarios.find(
        s => s.name === 'Failed login'
      );
      expect(failScenario).toBeDefined();
      expect(failScenario!.feature).toContain('login.feature');
      expect(failScenario!.line).toBeGreaterThan(successScenario!.line);
    });
  });

  describe('Scenario: Handle missing spec/features directory', () => {
    it('should show error when directory not found', async () => {
      // Given spec/features/ directory does not exist
      const missingDir = join(testDir, 'missing');

      // When I run `fspec get-scenarios --tag=@phase1`
      const result = await getScenarios({ tags: ['@phase1'], cwd: missingDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show error "spec/features directory not found"
      expect(result.error).toContain('not found');
    });
  });

  describe('Scenario: Skip invalid feature files with warning', () => {
    it('should show scenarios from valid files and warn about invalid', async () => {
      // Given I have 2 valid feature files tagged @phase1
      await writeFile(
        join(testDir, 'spec/features/valid1.feature'),
        `@phase1
Feature: Valid 1

  Scenario: Test
    Given test
`
      );

      await writeFile(
        join(testDir, 'spec/features/valid2.feature'),
        `@phase1
Feature: Valid 2

  Scenario: Test
    Given test
`
      );

      // And I have 1 invalid feature file with syntax errors
      await writeFile(
        join(testDir, 'spec/features/invalid.feature'),
        'This is not valid Gherkin syntax'
      );

      // When I run `fspec get-scenarios --tag=@phase1`
      const result = await getScenarios({ tags: ['@phase1'], cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show scenarios from the 2 valid files
      expect(result.scenarios.length).toBe(2);

      // And the output should show warning about the invalid file
      expect(result.warnings).toBeDefined();
      expect(result.warnings!.length).toBe(1);
      expect(result.warnings![0]).toContain('invalid.feature');
    });
  });

  describe('Scenario: Get scenarios from all features when no tag specified', () => {
    it('should show all scenarios from all features', async () => {
      // Given I have 5 feature files with various tags
      for (let i = 1; i <= 5; i++) {
        const content = `@tag${i}
Feature: Feature ${i}

  Scenario: Scenario ${i}
    Given test
`;
        await writeFile(
          join(testDir, 'spec/features', `f${i}.feature`),
          content
        );
      }

      // When I run `fspec get-scenarios`
      const result = await getScenarios({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show all scenarios from all 5 features
      expect(result.scenarios.length).toBe(5);

      // And the output should show total count of scenarios
      expect(result.totalCount).toBe(5);
    });
  });

  describe('Scenario: Handle features with no scenarios', () => {
    it('should skip features with no scenarios', async () => {
      // Given I have a feature file tagged @phase1 with no scenarios
      await writeFile(
        join(testDir, 'spec/features/empty.feature'),
        `@phase1
Feature: Empty Feature
`
      );

      // And I have a feature file tagged @phase1 with 3 scenarios
      await writeFile(
        join(testDir, 'spec/features/with-scenarios.feature'),
        `@phase1
Feature: With Scenarios

  Scenario: First
    Given test

  Scenario: Second
    Given test

  Scenario: Third
    Given test
`
      );

      // When I run `fspec get-scenarios --tag=@phase1`
      const result = await getScenarios({ tags: ['@phase1'], cwd: testDir });

      // Then the output should only show the 3 scenarios
      expect(result.scenarios.length).toBe(3);

      // And the empty feature should not appear in output
      expect(
        result.scenarios.every(s => !s.feature.includes('empty.feature'))
      ).toBe(true);
    });
  });

  describe('Scenario: Format output as JSON', () => {
    it('should return valid JSON with scenario details', async () => {
      // Given I have feature files tagged @phase1 with scenarios
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        `@phase1
Feature: Test

  Scenario: Test scenario
    Given test
`
      );

      // When I run `fspec get-scenarios --tag=@phase1 --format=json`
      const result = await getScenarios({
        tags: ['@phase1'],
        format: 'json',
        cwd: testDir,
      });

      // Then the output should be valid JSON
      expect(result.success).toBe(true);

      // And each scenario should have feature, name, and line properties
      expect(result.scenarios[0]).toHaveProperty('feature');
      expect(result.scenarios[0]).toHaveProperty('name');
      expect(result.scenarios[0]).toHaveProperty('line');

      // And the JSON should be parseable by other tools
      const jsonString = JSON.stringify(result.scenarios);
      const parsed = JSON.parse(jsonString);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  describe('Scenario: Group scenarios by feature file', () => {
    it('should group scenarios by their feature file', async () => {
      // Given I have feature file "login.feature" with 3 scenarios
      await writeFile(
        join(testDir, 'spec/features/login.feature'),
        `@auth
Feature: Login

  Scenario: First
    Given test

  Scenario: Second
    Given test

  Scenario: Third
    Given test
`
      );

      // And I have feature file "signup.feature" with 2 scenarios
      await writeFile(
        join(testDir, 'spec/features/signup.feature'),
        `@auth
Feature: Signup

  Scenario: First
    Given test

  Scenario: Second
    Given test
`
      );

      // When I run `fspec get-scenarios --tag=@auth`
      const result = await getScenarios({ tags: ['@auth'], cwd: testDir });

      // Then the output should group scenarios by feature file
      expect(result.success).toBe(true);

      // And each group should show the feature name as a header
      const loginScenarios = result.scenarios.filter(s =>
        s.feature.includes('login.feature')
      );
      const signupScenarios = result.scenarios.filter(s =>
        s.feature.includes('signup.feature')
      );

      expect(loginScenarios.length).toBe(3);
      expect(signupScenarios.length).toBe(2);
    });
  });

  describe('Scenario: Count scenarios matching tags', () => {
    it('should show count of scenarios found', async () => {
      // Given I have 10 scenarios across various features
      for (let i = 1; i <= 5; i++) {
        const tags = i <= 3 ? '@critical' : '@normal';
        const scenarioCount = i <= 3 ? 2 : 2;
        let content = `${tags}\nFeature: Feature ${i}\n\n`;
        for (let j = 1; j <= scenarioCount; j++) {
          content += `  Scenario: Scenario ${j}\n    Given test\n\n`;
        }
        await writeFile(
          join(testDir, 'spec/features', `f${i}.feature`),
          content
        );
      }

      // When I run `fspec get-scenarios --tag=@critical`
      const result = await getScenarios({ tags: ['@critical'], cwd: testDir });

      // Then the output should show "Found 6 scenarios matching tags: @critical"
      expect(result.totalCount).toBe(6);
      expect(result.message).toContain('6 scenarios');
      expect(result.message).toContain('@critical');
    });
  });

  describe('Scenario: Filter scenarios by scenario-level tags', () => {
    it('should filter scenarios by tags at scenario level', async () => {
      // Given I have a feature file with scenario-level tags
      const content = `Feature: User Authentication

  @smoke
  Scenario: Quick login test
    Given I am on the login page
    When I enter credentials
    Then I am logged in

  @regression
  @edge-case
  Scenario: Login with expired session
    Given I have an expired session
    When I attempt to login
    Then I am prompted to re-authenticate

  Scenario: Standard login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      await writeFile(join(testDir, 'spec/features/auth.feature'), content);

      // When I run `fspec get-scenarios --tag=@smoke`
      const result = await getScenarios({ tags: ['@smoke'], cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should show only the "Quick login test" scenario
      expect(result.scenarios.length).toBe(1);
      expect(result.scenarios[0].name).toBe('Quick login test');

      // And the output should not show "Login with expired session"
      expect(
        result.scenarios.find(s => s.name === 'Login with expired session')
      ).toBeUndefined();

      // And the output should not show "Standard login"
      expect(
        result.scenarios.find(s => s.name === 'Standard login')
      ).toBeUndefined();
    });
  });

  describe('Scenario: Filter scenarios by multiple scenario-level tags with AND logic', () => {
    it('should match scenarios with all specified scenario tags', async () => {
      // Given I have a feature file with multiple scenario tags
      const content = `Feature: User Authentication

  @smoke
  @critical
  Scenario: Critical smoke test
    Given a step
    When another step
    Then result

  @smoke
  Scenario: Regular smoke test
    Given a step
    When another step
    Then result

  @regression
  @critical
  Scenario: Critical regression test
    Given a step
    When another step
    Then result
`;
      await writeFile(join(testDir, 'spec/features/auth.feature'), content);

      // When I run `fspec get-scenarios --tag=@smoke --tag=@critical`
      const result = await getScenarios({
        tags: ['@smoke', '@critical'],
        cwd: testDir,
      });

      // Then the output should show only "Critical smoke test"
      expect(result.scenarios.length).toBe(1);
      expect(result.scenarios[0].name).toBe('Critical smoke test');

      // And the output should not show "Regular smoke test"
      expect(
        result.scenarios.find(s => s.name === 'Regular smoke test')
      ).toBeUndefined();

      // And the output should not show "Critical regression test"
      expect(
        result.scenarios.find(s => s.name === 'Critical regression test')
      ).toBeUndefined();
    });
  });

  describe('Scenario: Match scenarios with inherited feature tags', () => {
    it('should match scenarios that inherit feature-level tags', async () => {
      // Given I have a feature file with feature-level tags
      const content = `@phase1
@authentication
Feature: User Login

  Scenario: Basic login
    Given I am on the login page
    When I enter credentials
    Then I am logged in

  @smoke
  Scenario: Quick test
    Given a quick test
    When I run it
    Then it passes
`;
      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec get-scenarios --tag=@phase1`
      const result = await getScenarios({ tags: ['@phase1'], cwd: testDir });

      // Then the output should show both scenarios
      expect(result.scenarios.length).toBe(2);
      expect(
        result.scenarios.find(s => s.name === 'Basic login')
      ).toBeDefined();
      expect(result.scenarios.find(s => s.name === 'Quick test')).toBeDefined();

      // And both scenarios inherit the @phase1 tag from the feature
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Filter scenarios combining feature tags and scenario tags', () => {
    it('should filter using both feature and scenario level tags', async () => {
      // Given I have a feature file with both feature and scenario tags
      const content = `@phase1
@authentication
Feature: User Login

  @smoke
  Scenario: Smoke test login
    Given I am on the login page
    When I enter credentials
    Then I am logged in

  @regression
  Scenario: Regression test login
    Given I am on the login page
    When I enter different credentials
    Then I am logged in
`;
      await writeFile(join(testDir, 'spec/features/login.feature'), content);

      // When I run `fspec get-scenarios --tag=@authentication --tag=@smoke`
      const result = await getScenarios({
        tags: ['@authentication', '@smoke'],
        cwd: testDir,
      });

      // Then the output should show only "Smoke test login"
      expect(result.scenarios.length).toBe(1);
      expect(result.scenarios[0].name).toBe('Smoke test login');

      // And the scenario matches both feature tag (@authentication) and scenario tag (@smoke)
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Show scenario tags in output', () => {
    it('should display scenario-level tags in output', async () => {
      // Given I have a feature file with scenario-level tags
      const content = `Feature: Testing

  @smoke
  @critical
  Scenario: Important test
    Given a step
    When another step
    Then result
`;
      await writeFile(join(testDir, 'spec/features/test.feature'), content);

      // When I run `fspec get-scenarios --tag=@smoke`
      const result = await getScenarios({ tags: ['@smoke'], cwd: testDir });

      // Then the output should show scenario tags in the output
      expect(result.success).toBe(true);
      expect(result.scenarios.length).toBe(1);

      // And the output should display "[@smoke @critical]" next to the scenario name
      expect(result.scenarios[0]).toHaveProperty('tags');
      expect(result.scenarios[0].tags).toEqual(['@smoke', '@critical']);
    });
  });
});
