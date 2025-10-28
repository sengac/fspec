/**
 * Feature: spec/features/work-unit-details-panel-shows-incorrect-work-unit-after-reordering.feature
 *
 * Tests for done column automatic sorting when moving work units to done status.
 * This ensures work units are inserted at the correct position based on 'updated' timestamp.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work unit details panel shows incorrect work unit after reordering', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Moving work unit to done inserts at correct sorted position', () => {
    it('should insert work unit at correct position based on updated timestamp', async () => {
      // @step Given BOARD-005 is in implementing status with updated timestamp 2025-10-28T10:00:00Z
      // @step And the states.done array contains [BOARD-003, BOARD-001]
      // @step And BOARD-003 has updated timestamp 2025-10-28T11:00:00Z
      // @step And BOARD-001 has updated timestamp 2025-10-28T09:00:00Z
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T09:00:00Z', // Yesterday 9am (oldest)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T11:00:00Z', // Yesterday 11am
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-005': {
            id: 'BOARD-005',
            title: 'Work Unit 5',
            status: 'validating',
            type: 'story',
            updated: '2025-10-27T10:00:00Z', // Yesterday 10am (will be updated to current time)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-005'],
          done: ['BOARD-003', 'BOARD-001'], // Already sorted by timestamp (most recent first)
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // @step When I run "fspec update-work-unit-status BOARD-005 done"
      await updateWorkUnitStatus({
        workUnitId: 'BOARD-005',
        status: 'done',
        cwd: testDir,
      });

      // @step Then BOARD-005 should be inserted and ENTIRE array sorted
      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      // With full array sorting, BOARD-005 gets current timestamp (most recent)
      // So it should be at position 0, followed by BOARD-003 (11:00), BOARD-001 (09:00)
      expect(writtenData.states.done).toEqual([
        'BOARD-005', // Most recent (current timestamp)
        'BOARD-003', // 11:00
        'BOARD-001', // 09:00
      ]);

      // @step And BOARD-005 updated field should be set to current timestamp
      expect(writtenData.workUnits['BOARD-005'].updated).toBeDefined();
      expect(writtenData.workUnits['BOARD-005'].status).toBe('done');

      // Verify timestamp is ISO format and recent (within last minute)
      const updatedTime = new Date(
        writtenData.workUnits['BOARD-005'].updated
      ).getTime();
      const now = Date.now();
      expect(updatedTime).toBeGreaterThan(now - 60000); // Within last minute
      expect(updatedTime).toBeLessThanOrEqual(now);
    });

    it('should insert at beginning when work unit has most recent timestamp', async () => {
      // Work unit with timestamp AFTER all existing done items
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T09:00:00Z', // Yesterday 9am
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T10:00:00Z', // Yesterday 10am
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-007': {
            id: 'BOARD-007',
            title: 'Work Unit 7',
            status: 'validating',
            type: 'story',
            updated: '2025-10-27T12:00:00Z', // Yesterday 12pm (will get current timestamp - most recent)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-007'],
          done: ['BOARD-003', 'BOARD-001'],
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      await updateWorkUnitStatus({
        workUnitId: 'BOARD-007',
        status: 'done',
        cwd: testDir,
      });

      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      // Should be inserted at position 0 (beginning)
      expect(writtenData.states.done).toEqual([
        'BOARD-007',
        'BOARD-003',
        'BOARD-001',
      ]);
    });

    it('should sort entire array when moving work unit to done', async () => {
      // With full array sorting, the newly moved work unit gets current timestamp (most recent)
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T10:00:00Z', // Yesterday 10am
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T11:00:00Z', // Yesterday 11am
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Work Unit 2',
            status: 'validating',
            type: 'story',
            updated: '2025-10-27T08:00:00Z', // Yesterday 8am (will get current timestamp)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-002'],
          done: ['BOARD-003', 'BOARD-001'],
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      await updateWorkUnitStatus({
        workUnitId: 'BOARD-002',
        status: 'done',
        cwd: testDir,
      });

      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      // With full sorting, BOARD-002 gets current timestamp (most recent)
      // So it should be at position 0
      expect(writtenData.states.done).toEqual([
        'BOARD-002', // Most recent (current timestamp)
        'BOARD-003', // Yesterday 11am
        'BOARD-001', // Yesterday 10am
      ]);
    });

    it('should handle empty done array correctly', async () => {
      // First work unit moving to done
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'validating',
            type: 'story',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-001'],
          done: [], // Empty done array
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      await updateWorkUnitStatus({
        workUnitId: 'BOARD-001',
        status: 'done',
        cwd: testDir,
      });

      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      // Should be inserted as the only item
      expect(writtenData.states.done).toEqual(['BOARD-001']);
      expect(writtenData.workUnits['BOARD-001'].updated).toBeDefined();
    });

    it('should sort entire done array when moving new work unit to done', async () => {
      // Done array has mis-ordered work units
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T09:00:00Z', // Yesterday 9am (oldest)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T11:00:00Z', // Yesterday 11am
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-005': {
            id: 'BOARD-005',
            title: 'Work Unit 5',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T12:00:00Z', // Yesterday 12pm (most recent)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-007': {
            id: 'BOARD-007',
            title: 'Work Unit 7',
            status: 'validating',
            type: 'story',
            updated: '2025-10-27T10:00:00Z', // Yesterday 10am (will get current timestamp)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-007'],
          done: ['BOARD-003', 'BOARD-001', 'BOARD-005'], // Mis-ordered!
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // Move BOARD-007 to done
      await updateWorkUnitStatus({
        workUnitId: 'BOARD-007',
        status: 'done',
        cwd: testDir,
      });

      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      // ENTIRE array should be sorted by 'updated' timestamp (most recent first)
      // BOARD-007 gets current timestamp (most recent)
      // BOARD-005 (12:00), BOARD-003 (11:00), BOARD-001 (09:00)
      expect(writtenData.states.done).toEqual([
        'BOARD-007', // Most recent (current timestamp)
        'BOARD-005', // 12:00
        'BOARD-003', // 11:00
        'BOARD-001', // 09:00 (oldest)
      ]);

      // Verify BOARD-007 has the most recent timestamp
      expect(writtenData.workUnits['BOARD-007'].updated).toBeDefined();
      const board007Time = new Date(
        writtenData.workUnits['BOARD-007'].updated
      ).getTime();
      const now = Date.now();
      expect(board007Time).toBeGreaterThan(now - 60000); // Within last minute
      expect(board007Time).toBeLessThanOrEqual(now);
    });

    it('should sort using createdAt as fallback when updated field is missing', async () => {
      // Mix of work units: some with updated, some without
      const workUnitsFile = join(testDir, 'spec/work-units.json');
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Work Unit 1',
            status: 'done',
            type: 'story',
            // NO updated field - falls back to createdAt
            createdAt: '2025-10-27T09:00:00Z', // Yesterday 9am (oldest)
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-27T11:00:00Z', // Yesterday 11am (has updated)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Work Unit 2',
            status: 'done',
            type: 'story',
            // NO updated field - falls back to createdAt
            createdAt: '2025-10-27T10:00:00Z', // Yesterday 10am
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-005': {
            id: 'BOARD-005',
            title: 'Work Unit 5',
            status: 'validating',
            type: 'story',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-005'],
          done: ['BOARD-001', 'BOARD-003', 'BOARD-002'], // Unsorted mix
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // Move BOARD-005 to done (will get current timestamp via updated field)
      await updateWorkUnitStatus({
        workUnitId: 'BOARD-005',
        status: 'done',
        cwd: testDir,
      });

      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      // Expected order with fallback:
      // - BOARD-005: updated = current (most recent)
      // - BOARD-003: updated = 11:00
      // - BOARD-002: createdAt = 10:00 (fallback)
      // - BOARD-001: createdAt = 09:00 (fallback, oldest)
      expect(writtenData.states.done).toEqual([
        'BOARD-005', // Most recent (current timestamp via updated)
        'BOARD-003', // Yesterday 11am (via updated)
        'BOARD-002', // Yesterday 10am (via createdAt fallback)
        'BOARD-001', // Yesterday 9am (via createdAt fallback, oldest)
      ]);

      // Verify BOARD-005 has updated field
      expect(writtenData.workUnits['BOARD-005'].updated).toBeDefined();
    });
  });
});
