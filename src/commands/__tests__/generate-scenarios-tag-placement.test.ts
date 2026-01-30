/**
 * Feature: spec/features/remove-work-unit-id-tags-from-generate-scenarios.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { generateScenarios } from '../generate-scenarios';
import { setupWorkUnitTest, type WorkUnitTestSetup } from '../../test-helpers/universal-test-setup';

describe('Feature: Remove work unit ID tags from generate-scenarios', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('generate-scenarios-tag-placement');

    // Initialize work units file with test data
    await writeFile(
      setup.workUnitsFile,
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
    await setup.cleanup();
  });

  describe('Scenario: Generate scenarios adds work unit ID as feature-level tag only', () => {
    it('should add work unit ID as feature-level tag and not on scenarios', async () => {
      // Given I have a work unit with ID "TEST-001" in specifying status
      // And the work unit has example mapping data (rules, examples, questions answered)
      const workUnits = JSON.parse(await readFile(setup.workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-001'] = {
        id: 'TEST-001',
        title: 'Test Feature',
        status: 'specifying',
        userStory: {
          role: 'developer',
          action: 'test the feature',
          benefit: 'verification works',
        },
        examples: [
          'First test scenario',
          'Second test scenario',
          'Third test scenario',
        ],
        rules: ['Rule 1', 'Rule 2'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('TEST-001');
      await writeFile(setup.workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec generate-scenarios TEST-001`
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });

      // Then a feature file should be created with @TEST-001 as a feature-level tag
      const featureContent = await readFile(result.featureFile, 'utf-8');
      const lines = featureContent.split('\n');

      // Find the feature-level tags (before "Feature:" keyword)
      const featureLine = lines.findIndex(line => line.startsWith('Feature:'));
      const featureLevelTags = lines
        .slice(0, featureLine)
        .filter(line => line.trim().startsWith('@'));

      // Verify @TEST-001 is present at feature level
      expect(featureLevelTags.some(tag => tag.includes('@TEST-001'))).toBe(
        true
      );

      // And none of the generated scenarios should have @TEST-001 as a scenario-level tag
      const scenarioLines = lines
        .map((line, index) => ({ line, index }))
        .filter(({ line }) => line.trim().startsWith('Scenario:'));

      for (const { line, index } of scenarioLines) {
        // Check if there are any tags on the line(s) immediately before this scenario
        const linesBefore = lines.slice(Math.max(0, index - 5), index);
        const scenarioLevelTags = linesBefore.filter(
          l => l.trim().startsWith('@') && l.trim().startsWith('  @') // Scenario-level tags are indented
        );

        // Verify no scenario-level tags contain @TEST-001
        const hasWorkUnitIdTag = scenarioLevelTags.some(tag =>
          tag.includes('@TEST-001')
        );
        expect(hasWorkUnitIdTag).toBe(false);
      }
    });
  });
});
