/**
 * Feature: spec/features/add-remove-persona-and-remove-capability-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests for remove-persona command.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { removePersona } from '../remove-persona';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: remove-persona command', () => {
  let testDir: string;
  let draftPath: string;
  let foundationPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = join(process.cwd(), `test-temp-${Date.now()}`);
    draftPath = join(testDir, 'spec/foundation.json.draft');
    foundationPath = join(testDir, 'spec/foundation.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Remove persona from draft by name', () => {
    it('should remove persona from draft when it exists', async () => {
      // Given I have a foundation.json.draft file
      // And the draft contains a persona named "Developer"
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
        personas: [
          {
            name: 'Developer',
            description: 'Software engineers',
            goals: ['Build features'],
          },
          {
            name: 'Researcher',
            description: 'Researchers',
            goals: ['Research topics'],
          },
        ],
      };
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run `fspec remove-persona "Developer"`
      await removePersona(testDir, 'Developer');

      // Then the persona "Developer" should be removed from spec/foundation.json.draft
      const updatedDraft = JSON.parse(await readFile(draftPath, 'utf-8'));
      expect(updatedDraft.personas).toHaveLength(1);
      expect(updatedDraft.personas[0].name).toBe('Researcher');
    });
  });

  describe('Scenario: Show error when persona not found', () => {
    it('should show error with list of available personas', async () => {
      // Given I have a foundation.json.draft file
      // And the draft contains personas "Developer" and "Researcher"
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
        personas: [
          {
            name: 'Developer',
            description: 'Software engineers',
            goals: ['Build features'],
          },
          {
            name: 'Researcher',
            description: 'Researchers',
            goals: ['Research topics'],
          },
        ],
      };
      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run `fspec remove-persona "NonExistent"`
      // Then the command should exit with code 1
      await expect(removePersona(testDir, 'NonExistent')).rejects.toThrow(
        'Persona "NonExistent" not found',
      );
    });
  });
});
