/**
 * Feature: spec/features/add-remove-persona-and-remove-capability-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests for remove-capability command.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { removeCapability } from '../remove-capability';
import { join } from 'path';

import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
  ensureTestDirectory,
} from '../../test-helpers/test-file-operations';
describe('Feature: remove-capability command', () => {
  let setup: TestDirectorySetup;
  let foundationPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    setup = await setupTestDirectory('remove-capability');
    foundationPath = join(setup.testDir, 'spec/foundation.json');
    await ensureTestDirectory(join(setup.testDir, 'spec'));
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
      await writeJsonTestFile(foundationPath, foundation);

      // When I run `fspec remove-capability "Mind Mapping"`
      await removeCapability(setup.testDir, 'Mind Mapping');

      // Then the capability "Mind Mapping" should be removed from spec/foundation.json
      const updatedFoundation = await readJsonTestFile(foundationPath);
      expect(updatedFoundation.solutionSpace.capabilities).toHaveLength(1);
      expect(updatedFoundation.solutionSpace.capabilities[0].name).toBe(
        'AI Chat'
      );
    });
  });
});
