/**
 * Feature: spec/features/add-persona-and-add-capability-don-t-work-with-foundation-json-draft.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests for add-persona command with draft support.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { addPersona } from '../add-persona';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: add-persona draft support', () => {
  let testDir: string;
  let draftPath: string;
  let foundationPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = await createTempTestDir('add-persona-draft-support');
    draftPath = join(testDir, 'spec/foundation.json.draft');
    foundationPath = join(testDir, 'spec/foundation.json');
  });

  afterEach(async () => {
    // Clean up
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Add persona to foundation.json.draft during discovery', () => {
    it('should add persona to draft when draft exists', async () => {
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

      // When I run `fspec add-persona "Developer" "Software engineers" --goal "Build features"`
      await addPersona(testDir, 'Developer', 'Software engineers', [
        'Build features',
      ]);

      // Then the persona should be added to spec/foundation.json.draft
      const updatedDraft = JSON.parse(await readFile(draftPath, 'utf-8'));
      expect(updatedDraft.personas).toHaveLength(1);
      expect(updatedDraft.personas[0]).toEqual({
        name: 'Developer',
        description: 'Software engineers',
        goals: ['Build features'],
      });

      // And spec/foundation.json should not be modified (should not exist)
      await expect(readFile(foundationPath, 'utf-8')).rejects.toThrow();
    });
  });

  describe('Scenario: Add persona to foundation.json when no draft exists', () => {
    it('should add persona to foundation.json for backward compatibility', async () => {
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

      // When I run `fspec add-persona "User" "End users" --goal "Use the app"`
      await addPersona(testDir, 'User', 'End users', ['Use the app']);

      // Then the persona should be added to spec/foundation.json
      const updatedFoundation = JSON.parse(
        await readFile(foundationPath, 'utf-8')
      );
      expect(updatedFoundation.personas).toHaveLength(1);
      expect(updatedFoundation.personas[0]).toEqual({
        name: 'User',
        description: 'End users',
        goals: ['Use the app'],
      });
    });
  });

  describe('Scenario: Show helpful error when neither file exists', () => {
    it('should show error with discover-foundation suggestion', async () => {
      // Given I do not have a foundation.json.draft file
      // And I do not have a foundation.json file

      // When I run `fspec add-persona "Developer" "Engineers" --goal "Build"`
      // Then the command should throw with helpful error
      await expect(
        addPersona(testDir, 'Developer', 'Engineers', ['Build'])
      ).rejects.toThrow('foundation.json not found');
    });
  });
});
