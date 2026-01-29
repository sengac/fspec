/**
 * Feature: spec/features/work-unit-ids-reused-after-deletion-violating-immutability-principle.feature
 *
 * Tests for BUG-056: Work unit IDs reused after deletion violating immutability principle
 *
 * Bug: generateNextId() in create-bug.ts, create-story.ts, create-task.ts uses Math.max()
 * on currently existing IDs only, ignoring deleted work units.
 *
 * Solution: Add prefixCounters: Record<string, number> to WorkUnitsData type to track
 * high water marks. Similar pattern to nextRuleId/nextExampleId fields within work units (IDX-001).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData } from '../../types';
import { createBug } from '../create-bug';
import { createStory } from '../create-story';
import { createTask } from '../create-task';
import { deleteWorkUnit } from '../delete-work-unit';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
} from '../../test-helpers/test-file-operations';

/**
 * Helper: Write work-units.json
 */
async function writeWorkUnits(
  setup: WorkUnitTestSetup,
  data: WorkUnitsData
): Promise<void> {
  await writeJsonTestFile(setup.workUnitsFile, data);
}

/**
 * Helper: Read work-units.json
 */
async function readWorkUnits(setup: WorkUnitTestSetup): Promise<WorkUnitsData> {
  return await readJsonTestFile<WorkUnitsData>(setup.workUnitsFile);
}

/**
 * Helper: Write prefixes.json
 */
async function writePrefixes(
  setup: WorkUnitTestSetup,
  data: PrefixesData
): Promise<void> {
  const prefixesPath = join(setup.specDir, 'prefixes.json');
  await writeJsonTestFile(prefixesPath, data);
}

describe('Feature: Work unit ID generation with prefix counters', () => {
  describe('Scenario: Work unit ID preserved after deletion', () => {
    let setup: WorkUnitTestSetup;

    beforeEach(async () => {
      setup = await setupWorkUnitTest(
        'work-unit-id-generation-prefix-counters-1'
      );
    });

    afterEach(async () => {
      await setup.cleanup();
    });

    it('should generate BUG-004 after deleting BUG-003', async () => {
      // @step Given I have a project with work-units.json
      await createMinimalFoundation(setup.testDir);

      // @step And work units "BUG-001", "BUG-002", and "BUG-003" exist
      // @step And the prefixCounters.BUG is 3
      const workUnitsData: WorkUnitsData = {
        meta: { version: '0.7.0', lastUpdated: new Date().toISOString() },
        prefixCounters: { BUG: 3 },
        workUnits: {
          'BUG-001': {
            id: 'BUG-001',
            title: 'Bug 1',
            type: 'bug',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'BUG-002': {
            id: 'BUG-002',
            title: 'Bug 2',
            type: 'bug',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'BUG-003': {
            id: 'BUG-003',
            title: 'Bug 3',
            type: 'bug',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['BUG-001', 'BUG-002', 'BUG-003'],
          blocked: [],
        },
      };
      await writeWorkUnits(setup, workUnitsData);

      // Setup prefixes.json
      const prefixes: PrefixesData = {
        prefixes: {
          BUG: {
            id: 'BUG',
            description: 'Bug fixes',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writePrefixes(setup, prefixes);

      // @step When I delete work unit "BUG-003"
      const deleteResult = await deleteWorkUnit({
        workUnitId: 'BUG-003',
        skipConfirmation: true,
        cwd: setup.testDir,
      });
      expect(deleteResult.success).toBe(true);

      // @step And I create a new bug with title "New bug"
      const createResult = await createBug({
        cwd: setup.testDir,
        prefix: 'BUG',
        title: 'New bug',
        description: 'Test bug',
      });

      // @step Then the new bug should have ID "BUG-004"
      // NOTE: This test WILL FAIL until we implement prefixCounters
      // Currently the bug exists: IDs get reused after deletion
      expect(createResult.success).toBe(true);
      expect(createResult.workUnitId).toBe('BUG-004');

      // @step And the prefixCounters.BUG should be 4
      const updatedData = await readWorkUnits(setup);
      expect(updatedData.prefixCounters?.BUG).toBe(4);

      // @step And the work unit "BUG-003" should not exist
      expect(updatedData.workUnits['BUG-003']).toBeUndefined();
      expect(updatedData.workUnits['BUG-004']).toBeDefined();
      expect(updatedData.workUnits['BUG-004'].title).toBe('New bug');
    });
  });

  // TODO: Add more test scenarios after implementing prefixCounters:
  // - Multiple prefixes maintain separate high water marks
  // - Migration calculates high water marks from existing IDs
  // - Backward compatibility with existing projects
});
