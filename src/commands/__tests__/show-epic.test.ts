/**
 * Feature: spec/features/fix-show-epic-command-returning-undefined.feature
 *
 * This test file validates the acceptance criteria for fixing the show-epic command bug.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { showEpic } from '../show-epic';

describe('Feature: Fix show-epic command returning undefined', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));

    // Create spec directory
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show error when epic does not exist', () => {
    it('should throw error with epic ID when epic not found', async () => {
      // Given I have a project with epics configured
      const epicsFile = join(testDir, 'spec', 'epics.json');
      await writeFile(
        epicsFile,
        JSON.stringify({
          epics: {
            'user-management': {
              id: 'user-management',
              title: 'User Management',
            },
          },
        })
      );

      // And the epic "invalid-epic" does not exist
      // When I run "fspec show-epic invalid-epic"
      // Then the command should exit with code 1
      // And the output should contain "Epic 'invalid-epic' not found"
      await expect(
        showEpic({ epicId: 'invalid-epic', cwd: testDir })
      ).rejects.toThrow('Epic invalid-epic not found');
    });

    it('should throw error when epics.json does not exist', async () => {
      // Given I have a project without epics.json
      // When I run "fspec show-epic any-epic"
      // Then the command should throw error
      await expect(
        showEpic({ epicId: 'any-epic', cwd: testDir })
      ).rejects.toThrow('Epic any-epic not found');
    });
  });

  describe('Scenario: Show epic details when epic exists', () => {
    it('should return epic details without undefined values', async () => {
      // Given I have a project with epics configured
      const epicsFile = join(testDir, 'spec', 'epics.json');
      await writeFile(
        epicsFile,
        JSON.stringify({
          epics: {
            'user-management': {
              id: 'user-management',
              title: 'User Management Features',
              description: 'Authentication and user sessions',
            },
          },
        })
      );

      // And an epic "user-management" exists with title "User Management Features"
      // When I run "fspec show-epic user-management"
      const result = await showEpic({
        epicId: 'user-management',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      // And the output should contain "Epic: user-management"
      expect(result.epic.id).toBe('user-management');
      expect(result.epic.title).toBe('User Management Features');

      // And the output should not contain "undefined"
      expect(result.epic.id).not.toBe(undefined);
      expect(result.epic.title).not.toBe(undefined);
      expect(result.totalWorkUnits).toBeDefined();
      expect(result.completedWorkUnits).toBeDefined();
      expect(result.completionPercentage).toBeDefined();
    });
  });
});
