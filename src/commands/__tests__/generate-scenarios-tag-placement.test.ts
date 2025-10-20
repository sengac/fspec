/**
 * Feature: spec/features/remove-work-unit-id-tags-from-generate-scenarios.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateScenarios } from '../generate-scenarios';

describe('Feature: Remove work unit ID tags from generate-scenarios', () => {
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

  describe('Scenario: Generate scenarios adds work unit ID as feature-level tag only', () => {
    it('should add work unit ID as feature-level tag and not on scenarios', async () => {
      // Given I have a work unit with ID "TEST-001" in specifying status
      // And the work unit has example mapping data (rules, examples, questions answered)
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
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
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run `fspec generate-scenarios TEST-001`
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: testDir,
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
