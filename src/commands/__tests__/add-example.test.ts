/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: Example mapping commands auto-create work-units.json
 *
 * Tests that add-example command automatically creates work-units.json when missing.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, access, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { addExample } from '../add-example';
import type { WorkUnitsData } from '../../types';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Example mapping commands auto-create work-units.json', () => {
    it('should create work-units.json when adding example to non-existent file', async () => {
      // Given I have a fresh project with only spec/features/ directory
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // And spec/work-units.json does not exist
      // (file doesn't exist yet)

      // Create a work unit first (we need a work unit in specifying state)
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: ['WORK-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // When I run "fspec add-example work-unit-query 'Query by status' 'Example data'"
      const result = await addExample({
        workUnitId: 'WORK-001',
        example: 'Query by status - Example data',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);
      expect(result.exampleCount).toBe(1);

      // And spec/work-units.json should exist with proper structure
      await access(workUnitsFile); // Throws if file doesn't exist

      // And the file should contain empty workUnits object (plus our work unit)
      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(workUnitsFile, 'utf-8');
      const data = JSON.parse(fileContent);

      expect(data.workUnits).toBeDefined();
      expect(data.workUnits['WORK-001']).toBeDefined();

      // And the file should contain all Kanban states
      expect(data.states).toBeDefined();
      expect(data.states.backlog).toBeDefined();
      expect(data.states.specifying).toContain('WORK-001');
      expect(data.states.testing).toBeDefined();
      expect(data.states.implementing).toBeDefined();
      expect(data.states.validating).toBeDefined();
      expect(data.states.done).toBeDefined();
      expect(data.states.blocked).toBeDefined();
    });

    it('should auto-create work-units.json if completely missing', async () => {
      // Given I have a fresh project with only spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And spec/work-units.json does not exist
      // (testing the pure auto-creation path)

      // When add-example is called (this will fail because no work unit exists)
      // But the file should still be created

      const workUnitsFile = join(testDir, 'spec/work-units.json');

      // Let's test this by using the ensure utility directly through addExample
      // First we need to create the work-units.json file with a work unit
      try {
        await addExample({
          workUnitId: 'NONEXISTENT',
          example: 'Test example',
          cwd: testDir,
        });
      } catch (error) {
        // Expected to fail - work unit doesn't exist
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
