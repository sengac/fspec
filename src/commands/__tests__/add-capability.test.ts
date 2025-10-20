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
import { addCapability } from '../add-capability';

describe('Feature: Foundation discovery workflow - add-capability command', () => {
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

  describe('Scenario: Add capability to foundation.json through CLI', () => {
    it('should add a new capability with name and description', async () => {
      // Given I have a foundation.json file with an empty capabilities array
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
          capabilities: [],
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // When I run "fspec add-capability 'Kanban Workflow' 'Enforces ACDD phases with visual board'"
      await addCapability(
        testDir,
        'Kanban Workflow',
        'Enforces ACDD phases with visual board'
      );

      // Then the foundation.json file should contain the new capability
      const updatedFoundation = JSON.parse(
        await fs.readFile(foundationPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.capabilities).toHaveLength(1);

      // And the capability should have name "Kanban Workflow"
      expect(updatedFoundation.solutionSpace.capabilities[0].name).toBe(
        'Kanban Workflow'
      );

      // And the capability should have description "Enforces ACDD phases with visual board"
      expect(updatedFoundation.solutionSpace.capabilities[0].description).toBe(
        'Enforces ACDD phases with visual board'
      );

      // And the foundation.json should pass schema validation
      expect(updatedFoundation.version).toBe('2.0.0');
      expect(updatedFoundation.solutionSpace.capabilities[0]).toHaveProperty(
        'name'
      );
      expect(updatedFoundation.solutionSpace.capabilities[0]).toHaveProperty(
        'description'
      );
    });
  });

  describe('Scenario: Add multiple capabilities iteratively', () => {
    it('should preserve existing capabilities when adding new ones', async () => {
      // Given I have a foundation.json file with one existing capability
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
              name: 'Existing Capability',
              description: 'Already present',
            },
          ],
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // When I run "fspec add-capability 'Example Mapping' 'Collaborative discovery with rules and examples'"
      await addCapability(
        testDir,
        'Example Mapping',
        'Collaborative discovery with rules and examples'
      );

      // And I run "fspec add-capability 'Coverage Tracking' 'Link scenarios to tests and implementation'"
      await addCapability(
        testDir,
        'Coverage Tracking',
        'Link scenarios to tests and implementation'
      );

      // Then the foundation.json file should contain 3 capabilities total
      const updatedFoundation = JSON.parse(
        await fs.readFile(foundationPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.capabilities).toHaveLength(3);

      // And all capabilities should be preserved
      expect(updatedFoundation.solutionSpace.capabilities[0].name).toBe(
        'Existing Capability'
      );
      expect(updatedFoundation.solutionSpace.capabilities[1].name).toBe(
        'Example Mapping'
      );
      expect(updatedFoundation.solutionSpace.capabilities[2].name).toBe(
        'Coverage Tracking'
      );
    });
  });
});
