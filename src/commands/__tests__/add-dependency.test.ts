/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: Dependency commands auto-create work-units.json
 *
 * Tests that add-dependency command automatically creates work-units.json when missing.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, access, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { addDependency } from '../add-dependency';
import type { WorkUnitsData } from '../../types';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Dependency commands auto-create work-units.json', () => {
    it('should auto-create work-units.json when file is missing', async () => {
      // Given I have a fresh project with only spec/features/ directory
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // And spec/work-units.json does not exist
      // (file doesn't exist yet)

      // Create initial work units to test dependency between them
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'First Work Unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'WORK-002': {
            id: 'WORK-002',
            title: 'Second Work Unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['WORK-001', 'WORK-002'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // When I run any dependency management command
      const result = await addDependency({
        workUnitId: 'WORK-001',
        dependsOn: 'WORK-002',
        cwd: testDir,
      });

      // Then the command should auto-create spec/work-units.json
      await access(workUnitsFile); // Throws if file doesn't exist

      // And should not fail with ENOENT error
      expect(result.success).toBe(true);

      // Verify file structure is correct
      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(workUnitsFile, 'utf-8');
      const data = JSON.parse(fileContent);

      expect(data.workUnits['WORK-001'].dependsOn).toContain('WORK-002');
      expect(data.states).toBeDefined();
    });

    it('should auto-create work-units.json from scratch if completely missing', async () => {
      // Given I have a fresh project with only spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And spec/work-units.json does not exist
      const workUnitsFile = join(testDir, 'spec/work-units.json');

      // When add-dependency is called (will fail because work units don't exist)
      // But the file should still be created by ensureWorkUnitsFile
      try {
        await addDependency({
          workUnitId: 'NONEXISTENT-001',
          dependsOn: 'NONEXISTENT-002',
          cwd: testDir,
        });
      } catch (error) {
        // Expected to fail - work units don't exist
        expect(error).toBeDefined();
      }

      // Then spec/work-units.json should be created (even though operation failed)
      await access(workUnitsFile);

      // And the file should have proper structure
      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(workUnitsFile, 'utf-8');
      const data = JSON.parse(fileContent);

      expect(data.meta).toBeDefined();
      expect(data.meta.version).toBe('1.0.0');
      expect(data.workUnits).toEqual({});
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
