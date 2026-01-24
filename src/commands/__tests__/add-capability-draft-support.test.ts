/**
 * Feature: spec/features/add-persona-and-add-capability-don-t-work-with-foundation-json-draft.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests for add-capability command with draft support.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { addCapability } from '../add-capability';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: add-capability draft support', () => {
  let testDir: string;
  let draftPath: string;
  let foundationPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = await createTempTestDir('add-capability-draft-support');
    draftPath = join(testDir, 'spec/foundation.json.draft');
    foundationPath = join(testDir, 'spec/foundation.json');
  });

  afterEach(async () => {
    // Clean up
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Add capability to foundation.json.draft during discovery', () => {
    it('should add capability to draft when draft exists', async () => {
      // Given I have a foundation.json.draft file
      const draft = {
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
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run `fspec add-capability "User Authentication" "Login and registration"`
      await addCapability(
        testDir,
        'User Authentication',
        'Login and registration'
      );

      // Then the capability should be added to spec/foundation.json.draft
      const updatedDraft = JSON.parse(await readFile(draftPath, 'utf-8'));
      expect(updatedDraft.solutionSpace.capabilities).toHaveLength(1);
      expect(updatedDraft.solutionSpace.capabilities[0]).toEqual({
        name: 'User Authentication',
        description: 'Login and registration',
      });

      // And spec/foundation.json should not be modified (should not exist)
      await expect(readFile(foundationPath, 'utf-8')).rejects.toThrow();
    });
  });

  describe('Scenario: Add capability to foundation.json when no draft exists', () => {
    it('should add capability to foundation.json for backward compatibility', async () => {
      // Given I have a foundation.json file
      // And I do not have a foundation.json.draft file
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
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(
        foundationPath,
        JSON.stringify(foundation, null, 2),
        'utf-8'
      );

      // When I run `fspec add-capability "Feature" "Description"`
      await addCapability(testDir, 'Feature', 'Description');

      // Then the capability should be added to spec/foundation.json
      const updatedFoundation = JSON.parse(
        await readFile(foundationPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.capabilities).toHaveLength(1);
      expect(updatedFoundation.solutionSpace.capabilities[0]).toEqual({
        name: 'Feature',
        description: 'Description',
      });
    });
  });

  describe('Scenario: Show helpful error when neither file exists', () => {
    it('should show error with discover-foundation suggestion', async () => {
      // Given I do not have a foundation.json.draft file
      // And I do not have a foundation.json file

      // When I run `fspec add-capability "Feature" "Description"`
      // Then the command should throw with helpful error
      await expect(
        addCapability(testDir, 'Feature', 'Description')
      ).rejects.toThrow('foundation.json not found');
    });
  });
});
