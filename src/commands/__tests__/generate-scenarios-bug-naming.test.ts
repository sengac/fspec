/**
 * Feature: spec/features/feature-file-naming-for-bug-work-units.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { existsSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateScenarios } from '../example-mapping';

describe('Feature: Feature file naming for bug work units', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });
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
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Bug scenario added to existing matching feature file', () => {
    it('should add bug scenario to existing feature file when capability match found', async () => {
      // Given I have a bug work unit "BUG-001" with description "fspec help displays hardcoded version 0.0.1 instead of package.json version"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['BUG-001'] = {
        id: 'BUG-001',
        type: 'bug',
        title: 'fspec help displays hardcoded version',
        description:
          'fspec help displays hardcoded version 0.0.1 instead of package.json version',
        status: 'specifying',
        examples: [
          'Running fspec --help shows version 0.0.1',
          'Should display version from package.json',
        ],
        rules: ['Version must match package.json'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('BUG-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And an existing feature file "spec/features/help-command.feature" exists covering CLI help functionality
      const helpFeatureContent = `@phase1 @cli @help
Feature: Help Command

  Background: User Story
    As a developer using fspec
    I want to see helpful documentation
    So that I can use fspec effectively

  Scenario: Display help text
    Given I run "fspec --help"
    Then I should see command descriptions
`;
      const helpFeaturePath = join(featuresDir, 'help-command.feature');
      await writeFile(helpFeaturePath, helpFeatureContent);

      // When I run "fspec generate-scenarios BUG-001"
      const result = await generateScenarios('BUG-001', { cwd: testDir });

      // Then the system should identify "help-command.feature" as matching the capability
      // And a new scenario should be added to "spec/features/help-command.feature"
      const updatedFeatureContent = await readFile(helpFeaturePath, 'utf-8');
      expect(updatedFeatureContent).toContain('Scenario:');
      expect(updatedFeatureContent).toContain('@BUG-001');

      // And the scenario should describe the version display bug fix
      expect(updatedFeatureContent).toContain('version');

      // And the feature file name should remain "help-command.feature" (capability-oriented)
      expect(existsSync(helpFeaturePath)).toBe(true);

      // And no new feature file should be created
      const bugFeaturePath = join(
        featuresDir,
        'fspec-help-displays-hardcoded-version.feature'
      );
      expect(existsSync(bugFeaturePath)).toBe(false);
    });
  });

  describe('Scenario: New capability-oriented feature file created when no match found', () => {
    it('should create capability-oriented feature file when no existing match', async () => {
      // Given I have a bug work unit "BUG-002" with description "fspec help displays hardcoded version 0.0.1 instead of package.json version"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['BUG-002'] = {
        id: 'BUG-002',
        type: 'bug',
        title: 'fspec help displays hardcoded version',
        description:
          'fspec help displays hardcoded version 0.0.1 instead of package.json version',
        status: 'specifying',
        examples: [
          'Running fspec --help shows version 0.0.1',
          'Should display version from package.json',
        ],
        rules: ['Version must match package.json'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('BUG-002');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And no existing feature file covers CLI version display capability
      // (featuresDir is empty)

      // When I run "fspec generate-scenarios BUG-002"
      const result = await generateScenarios('BUG-002', { cwd: testDir });

      // Then the system should determine no existing feature matches the capability
      // And a new feature file should be created with capability-oriented name
      // And the feature file should be named "cli-version-display.feature" or similar capability name
      const capabilityFiles = [
        'cli-version-display.feature',
        'help-version-display.feature',
        'version-display.feature',
      ];

      const createdFile = capabilityFiles.find((file) =>
        existsSync(join(featuresDir, file))
      );
      expect(createdFile).toBeDefined();

      // And the feature file should NOT be named "fspec-help-displays-hardcoded-version.feature"
      const wrongPath = join(
        featuresDir,
        'fspec-help-displays-hardcoded-version.feature'
      );
      expect(existsSync(wrongPath)).toBe(false);

      // And the feature file should contain a scenario describing the version display capability
      if (createdFile) {
        const featureContent = await readFile(
          join(featuresDir, createdFile),
          'utf-8'
        );
        expect(featureContent).toContain('Scenario:');
        expect(featureContent).toContain('@BUG-002');
        expect(featureContent).toContain('version');
      }
    });
  });

  describe('Scenario: Edge case scenario added to existing validation feature', () => {
    it('should add edge case scenario to existing validation feature', async () => {
      // Given I have a bug work unit "BUG-003" with description "validate command crashes on empty file"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['BUG-003'] = {
        id: 'BUG-003',
        type: 'bug',
        title: 'validate command crashes on empty file',
        description: 'validate command crashes on empty file',
        status: 'specifying',
        examples: [
          'Running fspec validate on empty file crashes',
          'Should handle empty files gracefully',
        ],
        rules: ['Validation should not crash on empty files'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('BUG-003');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And an existing feature file "spec/features/gherkin-validation.feature" exists
      const validationFeatureContent = `@phase1 @parser @validation
Feature: Gherkin Syntax Validation

  Background: User Story
    As an AI agent writing Gherkin
    I want syntax validation
    So that I catch errors early

  Scenario: Validate valid feature file
    Given I have a valid feature file
    When I run "fspec validate"
    Then validation should pass
`;
      const validationFeaturePath = join(
        featuresDir,
        'gherkin-validation.feature'
      );
      await writeFile(validationFeaturePath, validationFeatureContent);

      // When I run "fspec generate-scenarios BUG-003"
      const result = await generateScenarios('BUG-003', { cwd: testDir });

      // Then the system should identify "gherkin-validation.feature" as matching the capability
      // And a new edge-case scenario should be added to "spec/features/gherkin-validation.feature"
      const updatedContent = await readFile(validationFeaturePath, 'utf-8');
      expect(updatedContent).toContain('@BUG-003');

      // And the scenario should describe validation behavior for empty files
      expect(updatedContent).toContain('empty');

      // And the feature file name should remain "gherkin-validation.feature"
      expect(existsSync(validationFeaturePath)).toBe(true);
    });
  });

  describe('Scenario: Regression scenario added to existing formatting feature', () => {
    it('should add regression scenario to existing formatting feature', async () => {
      // Given I have a bug work unit "BUG-004" with description "format command removes valid doc strings"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['BUG-004'] = {
        id: 'BUG-004',
        type: 'bug',
        title: 'format command removes valid doc strings',
        description: 'format command removes valid doc strings',
        status: 'specifying',
        examples: [
          'Running fspec format removes doc strings',
          'Should preserve valid doc strings',
        ],
        rules: ['Format should preserve doc strings'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('BUG-004');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And an existing feature file "spec/features/gherkin-formatting.feature" exists
      // And the feature already has a scenario covering doc string formatting
      const formattingFeatureContent = `@phase1 @formatter @formatting
Feature: Gherkin Feature File Formatting

  Background: User Story
    As a developer
    I want consistent formatting
    So that code reviews are easier

  Scenario: Format feature file with doc strings
    Given I have a feature file with doc strings
    When I run "fspec format"
    Then doc strings should be preserved
`;
      const formattingFeaturePath = join(
        featuresDir,
        'gherkin-formatting.feature'
      );
      await writeFile(formattingFeaturePath, formattingFeatureContent);

      // When I run "fspec generate-scenarios BUG-004"
      const result = await generateScenarios('BUG-004', { cwd: testDir });

      // Then the system should identify "gherkin-formatting.feature" as matching the capability
      // And either update the existing doc string scenario or add a new regression scenario
      const updatedContent = await readFile(formattingFeaturePath, 'utf-8');
      expect(updatedContent).toContain('@BUG-004');
      expect(updatedContent).toContain('doc string');

      // And the feature file name should remain "gherkin-formatting.feature"
      expect(existsSync(formattingFeaturePath)).toBe(true);

      // And the scenario should describe doc string preservation behavior
      expect(updatedContent).toContain('preserve');
    });
  });
});
