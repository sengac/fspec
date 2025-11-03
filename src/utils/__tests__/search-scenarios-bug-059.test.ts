/**
 * Feature: spec/features/search-scenarios-returns-no-results-when-it-should-find-matching-scenarios.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { searchScenarios } from '../../commands/search-scenarios';

describe('Feature: search-scenarios returns no results when it should find matching scenarios', () => {
  let testDir: string;
  let specDir: string;
  let featuresDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // @step Given I am setting up a test environment
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    featuresDir = join(specDir, 'features');
    workUnitsFile = join(specDir, 'work-units.json');

    await mkdir(featuresDir, { recursive: true });

    // Initialize work units file
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {},
          states: {
            backlog: [],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
            blocked: [],
          },
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Clean up test environment
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Search by feature file name (kebab-case)', () => {
    it('should find scenarios by searching feature file name', async () => {
      // @step Given a feature file named "spec/features/report-bug-to-github-with-ai-assistance.feature"
      // @step And the feature file contains scenarios
      const featureContent = `Feature: Report bug to GitHub with AI assistance

  Background: User Story
    As a developer
    I want to report bugs
    So that issues are tracked

  Scenario: Create bug report
    Given I have a bug
    When I report it
    Then it is tracked
`;
      await writeFile(
        join(featuresDir, 'report-bug-to-github-with-ai-assistance.feature'),
        featureContent
      );

      // @step When I run "fspec search-scenarios --query=report-bug"
      const result = await searchScenarios({
        query: 'report-bug',
        cwd: testDir,
      });

      // @step Then I should see scenarios from "report-bug-to-github-with-ai-assistance.feature"
      // @step And the results should not be empty
      expect(result.scenarios.length).toBeGreaterThan(0);
      expect(result.scenarios[0].featureFilePath).toContain(
        'report-bug-to-github-with-ai-assistance.feature'
      );
    });
  });

  describe('Scenario: Search by feature name', () => {
    it('should find scenarios by searching feature name', async () => {
      // @step Given a feature file with "Feature: User Authentication"
      // @step And the feature file contains scenarios
      const featureContent = `Feature: User Authentication

  Background: User Story
    As a user
    I want to authenticate
    So that I can access the system

  Scenario: Login with valid credentials
    Given I have valid credentials
    When I login
    Then I am authenticated
`;
      await writeFile(
        join(featuresDir, 'user-authentication.feature'),
        featureContent
      );

      // @step When I run "fspec search-scenarios --query=authentication"
      const result = await searchScenarios({
        query: 'authentication',
        cwd: testDir,
      });

      // @step Then I should see all scenarios from the "User Authentication" feature
      // @step And the results should not be empty
      expect(result.scenarios.length).toBeGreaterThan(0);
      expect(result.scenarios[0].scenarioName).toBe(
        'Login with valid credentials'
      );
    });
  });

  describe('Scenario: Search by feature description', () => {
    it('should find scenarios by searching feature description', async () => {
      // @step Given a feature file with description containing "mermaid diagram validation"
      // @step And the feature file contains scenarios
      const featureContent = `Feature: Diagram Validation

  """
  This feature validates mermaid diagram syntax before saving.
  """

  Background: User Story
    As a developer
    I want diagram validation
    So that syntax errors are caught

  Scenario: Validate mermaid syntax
    Given I have a mermaid diagram
    When I validate it
    Then syntax errors are detected
`;
      await writeFile(
        join(featuresDir, 'diagram-validation.feature'),
        featureContent
      );

      // @step When I run "fspec search-scenarios --query=mermaid"
      const result = await searchScenarios({
        query: 'mermaid',
        cwd: testDir,
      });

      // @step Then I should see all scenarios from features mentioning "mermaid" in description
      // @step And the results should not be empty
      expect(result.scenarios.length).toBeGreaterThan(0);
      expect(result.scenarios[0].scenarioName).toBe('Validate mermaid syntax');
    });
  });

  describe('Scenario: Search by work unit title', () => {
    it('should find scenarios by searching work unit title', async () => {
      // @step Given a work unit "BUG-059" with title "search-scenarios returns no results"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['BUG-059'] = {
        id: 'BUG-059',
        title: 'search-scenarios returns no results',
        status: 'testing',
        type: 'bug',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // @step And a feature file tagged with "@BUG-059"
      const featureContent = `@BUG-059
Feature: Search Scenarios Bug

  Background: User Story
    As a developer
    I want to search scenarios
    So that I can find them

  Scenario: Search functionality works
    Given I search for scenarios
    When I run search
    Then results are returned
`;
      await writeFile(join(featuresDir, 'search-bug.feature'), featureContent);

      // @step When I run "fspec search-scenarios --query=returns no results"
      const result = await searchScenarios({
        query: 'returns no results',
        cwd: testDir,
      });

      // @step Then I should see scenarios from the feature tagged with "@BUG-059"
      // @step And the results should not be empty
      expect(result.scenarios.length).toBeGreaterThan(0);
      expect(result.scenarios[0].workUnitId).toBe('BUG-059');
    });
  });

  describe('Scenario: Search by scenario name (existing functionality)', () => {
    it('should find scenarios by searching scenario name', async () => {
      // @step Given a feature file with scenario "Login with valid credentials"
      const featureContent = `Feature: User Login

  Background: User Story
    As a user
    I want to login
    So that I can access features

  Scenario: Login with valid credentials
    Given I have valid credentials
    When I login
    Then I am logged in
`;
      await writeFile(join(featuresDir, 'user-login.feature'), featureContent);

      // @step When I run "fspec search-scenarios --query=login"
      const result = await searchScenarios({
        query: 'login',
        cwd: testDir,
      });

      // @step Then I should see the scenario "Login with valid credentials"
      // @step And the results should not be empty
      expect(result.scenarios.length).toBeGreaterThan(0);
      expect(result.scenarios[0].scenarioName).toBe(
        'Login with valid credentials'
      );
    });
  });
});

// Helper function to read files
async function readFile(path: string, encoding: string): Promise<string> {
  const { readFile: fsReadFile } = await import('fs/promises');
  return fsReadFile(path, encoding);
}
