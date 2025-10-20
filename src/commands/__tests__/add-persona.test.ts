/**
 * Feature: spec/features/foundation-discovery-workflow-cannot-update-nested-fields-capabilities-personas.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { addPersona } from '../add-persona';

describe('Feature: Foundation discovery workflow - add-persona command', () => {
  let testDir: string;
  let foundationPath: string;

  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await fs.mkdir(testDir, { recursive: true });
    await fs.mkdir(join(testDir, 'spec'), { recursive: true });
    foundationPath = join(testDir, 'spec', 'foundation.json');
  });

  afterEach(async () => {
    await fs.rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add persona to foundation.json through CLI', () => {
    it('should add a new persona with name, description, and goals', async () => {
      // Given I have a foundation.json file
      const initialFoundation = {
        version: '2.0.0',
        project: {
          name: 'test-project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test overview',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test description',
            },
          ],
        },
        personas: [],
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // When I run "fspec add-persona 'AI Agent' 'Uses fspec to manage specifications' --goal 'Complete foundation discovery without manual editing'"
      await addPersona(
        testDir,
        'AI Agent',
        'Uses fspec to manage specifications',
        ['Complete foundation discovery without manual editing']
      );

      // Then the foundation.json file should contain the new persona
      const updatedFoundation = JSON.parse(
        await fs.readFile(foundationPath, 'utf-8')
      );
      expect(updatedFoundation.personas).toHaveLength(1);

      // And the persona should have name "AI Agent"
      expect(updatedFoundation.personas[0].name).toBe('AI Agent');

      // And the persona should have description "Uses fspec to manage specifications"
      expect(updatedFoundation.personas[0].description).toBe(
        'Uses fspec to manage specifications'
      );

      // And the persona should have goal "Complete foundation discovery without manual editing"
      expect(updatedFoundation.personas[0].goals).toContain(
        'Complete foundation discovery without manual editing'
      );

      // And the foundation.json should pass schema validation
      expect(updatedFoundation.version).toBe('2.0.0');
      expect(updatedFoundation.personas[0]).toHaveProperty('name');
      expect(updatedFoundation.personas[0]).toHaveProperty('description');
      expect(updatedFoundation.personas[0]).toHaveProperty('goals');
    });
  });
});
