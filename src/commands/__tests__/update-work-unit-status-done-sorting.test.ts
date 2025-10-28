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
            updated: '2025-10-28T09:00:00Z',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T11:00:00Z',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-005': {
            id: 'BOARD-005',
            title: 'Work Unit 5',
            status: 'validating',
            type: 'story',
            updated: '2025-10-28T10:00:00Z',
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

      // @step Then BOARD-005 should be inserted at position 1 in states.done array
      const fileContent = await readFile(workUnitsFile, 'utf-8');
      const writtenData: WorkUnitsData = JSON.parse(fileContent);

      expect(writtenData.states.done).toEqual([
        'BOARD-003',
        'BOARD-005',
        'BOARD-001',
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
            updated: '2025-10-28T09:00:00Z',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T10:00:00Z',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-007': {
            id: 'BOARD-007',
            title: 'Work Unit 7',
            status: 'validating',
            type: 'story',
            updated: '2025-10-28T12:00:00Z', // Most recent
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

    it('should insert at end when work unit has oldest timestamp', async () => {
      // Work unit with timestamp BEFORE all existing done items
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
            updated: '2025-10-28T10:00:00Z',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Work Unit 3',
            status: 'done',
            type: 'story',
            updated: '2025-10-28T11:00:00Z',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Work Unit 2',
            status: 'validating',
            type: 'story',
            updated: '2025-10-28T08:00:00Z', // Oldest
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

      // Should be inserted at position 2 (end)
      expect(writtenData.states.done).toEqual([
        'BOARD-003',
        'BOARD-001',
        'BOARD-002',
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
  });
});
