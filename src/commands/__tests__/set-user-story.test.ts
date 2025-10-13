/**
 * Feature: spec/features/feature-file-prefill-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { setUserStory } from '../set-user-story';
import type { WorkUnitsData } from '../../types';

describe('Feature: User Story Management in Work Units', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    const specDir = join(tmpDir, 'spec');
    const { mkdir: mkdirp } = await import('fs/promises');
    await mkdirp(specDir, { recursive: true });
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
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Set user story fields for work unit', () => {
    it('should set user story with role, action, and benefit', async () => {
      // Given: A work unit exists
      // When: I set user story fields
      await setUserStory('TEST-001', {
        role: 'developer',
        action: 'track user stories',
        benefit: 'better specification quality',
        cwd: tmpDir,
      });

      // Then: The work unit should have userStory field
      const { readFile: read } = await import('fs/promises');
      const content = await read(
        join(tmpDir, 'spec', 'work-units.json'),
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
        cwd: tmpDir,
      });

      // Then: The updatedAt should be recent
      const { readFile: read } = await import('fs/promises');
      const content = await read(
        join(tmpDir, 'spec', 'work-units.json'),
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
          cwd: tmpDir,
        })
      ).rejects.toThrow("Work unit 'NONEXISTENT-001' does not exist");
    });
  });
});
