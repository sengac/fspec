/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: Update work unit command uses ensureWorkUnitsFile instead of direct readFile
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { access } from 'fs/promises';
import { updateWorkUnit } from '../update-work-unit';
import type { WorkUnitsData } from '../../types';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Automatic JSON File Initialization', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('update-work-unit-ensure');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Update work unit command uses ensureWorkUnitsFile instead of direct readFile', () => {
    it('should create work-units.json if missing and update work unit', async () => {
      // Given I have a fresh project with spec/ directory
      // Already created by setupWorkUnitTest

      // And spec/work-units.json does not exist initially
      // But I have created a work unit "AUTH-001" (via ensureWorkUnitsFile in the command)

      // First, create the work unit
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Old Title',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeJsonTestFile(setup.workUnitsFile, initialData);

      // When I run "fspec update-work-unit AUTH-001 --title='New Title'"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        title: 'New Title',
        cwd: setup.testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And spec/work-units.json should exist
      await access(setup.workUnitsFile);

      // And the work unit "AUTH-001" title should be "New Title"
      // Using readJsonTestFile instead of manual fs operations
      // Read updated data
      const data: WorkUnitsData = await readJsonTestFile(setup.workUnitsFile);

      expect(data.workUnits['AUTH-001'].title).toBe('New Title');

      // And no ENOENT error should occur (file was auto-created by ensureWorkUnitsFile)
      expect(data.meta).toBeDefined();
      expect(data.states).toBeDefined();
    });
  });
});
