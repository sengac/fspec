/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: List work units command auto-creates spec/work-units.json when missing
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { access } from 'fs/promises';
import { join } from 'path';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { ensureTestDirectory } from '../../test-helpers/test-file-operations';
import { listWorkUnits } from '../list-work-units';

describe('Feature: Automatic JSON File Initialization', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('list-work-units');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: List work units command auto-creates spec/work-units.json when missing', () => {
    it('should create work-units.json with proper structure when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await ensureTestDirectory(join(setup.testDir, 'spec'));

      // And spec/work-units.json does not exist
      const workUnitsFile = join(setup.testDir, 'spec/work-units.json');

      // When I run "fspec list-work-units"
      const result = await listWorkUnits({ cwd: setup.testDir });

      // Then the command should succeed
      expect(result).toBeDefined();
      expect(result.workUnits).toBeInstanceOf(Array);

      // And spec/work-units.json should be created with proper structure
      await access(workUnitsFile); // Throws if file doesn't exist

      // And the file should contain empty workUnits object
      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(workUnitsFile, 'utf-8');
      const data = JSON.parse(fileContent);

      expect(data.workUnits).toBeDefined();
      expect(data.workUnits).toEqual({});

      // And the file should contain all 7 Kanban states
      expect(data.states).toBeDefined();
      expect(data.states.backlog).toEqual([]);
      expect(data.states.specifying).toEqual([]);
      expect(data.states.testing).toEqual([]);
      expect(data.states.implementing).toEqual([]);
      expect(data.states.validating).toEqual([]);
      expect(data.states.done).toEqual([]);
      expect(data.states.blocked).toEqual([]);
    });
  });
});
