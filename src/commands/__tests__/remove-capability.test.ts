/**
 * Feature: spec/features/add-remove-persona-and-remove-capability-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests for remove-capability command.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { removeCapability } from '../remove-capability';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: remove-capability command', () => {
  let testDir: string;
  let foundationPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = join(process.cwd(), `test-temp-${Date.now()}`);
    foundationPath = join(testDir, 'spec/foundation.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Remove capability from foundation.json by name', () => {
    it('should remove capability from foundation.json when it exists', async () => {
      // Given I have a foundation.json file
      // And the file contains a capability named "Mind Mapping"
      const foundation = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Mind Mapping',
              description: 'Visual mind mapping',
            },
            {
              name: 'AI Chat',
              description: 'Chat with AI',
            },
          ],
        },
        personas: [],
      };
      await writeFile(
        foundationPath,
        JSON.stringify(foundation, null, 2),
        'utf-8',
      );

      // When I run `fspec remove-capability "Mind Mapping"`
      await removeCapability(testDir, 'Mind Mapping');

      // Then the capability "Mind Mapping" should be removed from spec/foundation.json
      const updatedFoundation = JSON.parse(
        await readFile(foundationPath, 'utf-8'),
      );
      expect(updatedFoundation.solutionSpace.capabilities).toHaveLength(1);
      expect(updatedFoundation.solutionSpace.capabilities[0].name).toBe(
        'AI Chat',
      );
    });
  });
});
