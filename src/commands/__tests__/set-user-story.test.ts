/**
 * Feature: spec/features/feature-file-prefill-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'fs/promises';
import { join } from 'path';
import { setUserStory } from '../set-user-story';
import type { WorkUnitsData } from '../../types';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: User Story Management in Work Units', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('set-user-story');
    const specDir = join(setup.testDir, 'spec');
    await writeFile(
      join(specDir, 'work-units.json'),
      JSON.stringify({
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          specifying: ['TEST-001'],
        },
      })
    );
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Set user story fields for work unit', () => {
    it('should set user story with role, action, and benefit', async () => {
      // Given: A work unit exists
      // When: I set user story fields
      await setUserStory('TEST-001', {
        role: 'developer',
        action: 'track user stories',
        benefit: 'better specification quality',
        cwd: setup.testDir,
      });

      // Then: The work unit should have userStory field
      const { readFile: read } = await import('fs/promises');
      const content = await read(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);

      expect(data.workUnits['TEST-001'].userStory).toEqual({
        role: 'developer',
        action: 'track user stories',
        benefit: 'better specification quality',
      });
    });

    it('should update updatedAt timestamp', async () => {
      // Given: A work unit with an old timestamp
      const before = new Date().toISOString();

      // When: I set user story
      await setUserStory('TEST-001', {
        role: 'developer',
        action: 'test timestamps',
        benefit: 'accurate tracking',
        cwd: setup.testDir,
      });

      // Then: The updatedAt should be recent
      const { readFile: read } = await import('fs/promises');
      const content = await read(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      const after = data.workUnits['TEST-001'].updatedAt;

      expect(new Date(after).getTime()).toBeGreaterThanOrEqual(
        new Date(before).getTime()
      );
    });

    it('should throw error for non-existent work unit', async () => {
      // Given: A non-existent work unit
      // When/Then: Setting user story should throw error
      await expect(
        setUserStory('NONEXISTENT-001', {
          role: 'test',
          action: 'test',
          benefit: 'test',
          cwd: setup.testDir,
        })
      ).rejects.toThrow("Work unit 'NONEXISTENT-001' does not exist");
    });
  });
});
