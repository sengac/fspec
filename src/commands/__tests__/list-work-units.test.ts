/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: List work units command auto-creates spec/work-units.json when missing
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { listWorkUnits } from '../list-work-units';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List work units command auto-creates spec/work-units.json when missing', () => {
    it('should create work-units.json with proper structure when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And spec/work-units.json does not exist
      const workUnitsFile = join(testDir, 'spec/work-units.json');

      // When I run "fspec list-work-units"
      const result = await listWorkUnits({ cwd: testDir });

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
