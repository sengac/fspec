/**
 * Feature: spec/features/stable-indices-critical-bug-fixes.feature
 *
 * Tests for critical bug fixes in stable indices system.
 * These tests demonstrate and verify fixes for:
 * - Bug #1: nextNoteId vs nextArchitectureNoteId field name mismatch
 * - Bug #2: State sorting loss during auto-compact
 *
 * ACDD Phase: TESTING (write failing tests BEFORE implementation)
 * These tests will FAIL until bugs are fixed.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { vol } from 'memfs';
import { join } from 'path';
import type { WorkUnitsData } from '../../types';

// Type definitions for memfs and proper-lockfile
interface MemfsModule {
  fs: {
    promises: typeof import('fs/promises');
    [key: string]: unknown;
  };
}

interface LockfileModule {
  lock: (file: string, options?: LockOptions) => Promise<() => Promise<void>>;
  lockSync: (file: string, options?: LockOptions) => () => void;
  unlock: (file: string, options?: LockOptions) => Promise<void>;
  unlockSync: (file: string, options?: LockOptions) => void;
  check: (file: string, options?: LockOptions) => Promise<boolean>;
  checkSync: (file: string, options?: LockOptions) => boolean;
}

interface LockOptions {
  fs?: unknown;
  realpath?: boolean;
  [key: string]: unknown;
}

// Mock fs modules with memfs
vi.mock('fs/promises', async importOriginal => {
  const memfs = await vi.importActual<MemfsModule>('memfs');
  const actual = await importOriginal();
  return {
    ...actual,
    ...memfs.fs.promises,
    default: memfs.fs.promises,
  };
});

vi.mock('fs', async importOriginal => {
  const memfs = await vi.importActual<MemfsModule>('memfs');
  const actual = await importOriginal();
  return {
    ...actual,
    ...memfs.fs,
    default: memfs.fs,
  };
});

// Mock proper-lockfile to use memfs
vi.mock('proper-lockfile', async () => {
  const actualLockfile =
    await vi.importActual<LockfileModule>('proper-lockfile');
  const memfs = await vi.importActual<MemfsModule>('memfs');

  const lock = (file: string, options: LockOptions = {}) => {
    return actualLockfile.lock(file, {
      ...options,
      fs: memfs.fs,
      realpath: false,
    });
  };

  const lockSync = (file: string, options: LockOptions = {}) => {
    return actualLockfile.lockSync(file, {
      ...options,
      fs: memfs.fs,
      realpath: false,
    });
  };

  const unlock = (file: string, options: LockOptions = {}) => {
    return actualLockfile.unlock(file, {
      ...options,
      fs: memfs.fs,
      realpath: false,
    });
  };

  const unlockSync = (file: string, options: LockOptions = {}) => {
    return actualLockfile.unlockSync(file, {
      ...options,
      fs: memfs.fs,
      realpath: false,
    });
  };

  const check = (file: string, options: LockOptions = {}) => {
    return actualLockfile.check(file, {
      ...options,
      fs: memfs.fs,
      realpath: false,
    });
  };

  const checkSync = (file: string, options: LockOptions = {}) => {
    return actualLockfile.checkSync(file, {
      ...options,
      fs: memfs.fs,
      realpath: false,
    });
  };

  return {
    lock,
    lockSync,
    unlock,
    unlockSync,
    check,
    checkSync,
    default: {
      lock,
      lockSync,
      unlock,
      unlockSync,
      check,
      checkSync,
    },
  };
});

describe('Feature: Stable Indices Critical Bug Fixes', () => {
  const cwd = '/test';
  const workUnitsPath = join(cwd, 'spec/work-units.json');

  beforeEach(async () => {
    vol.reset();
    vol.mkdirSync(join(cwd, 'spec'), { recursive: true });
  });

  describe('Scenario: Prevent ID collision after compaction (line 32)', () => {
    it('should use correct nextNoteId field after compaction to prevent ID collisions', async () => {
      // Given a work unit AUTH-001 has 5 architecture notes
      const initialData: WorkUnitsData = {
        meta: { version: '0.7.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            status: 'implementing',
            architectureNotes: [
              {
                id: 0,
                text: 'Note A',
                deleted: false,
                createdAt: '2025-01-01T00:00:00.000Z',
              },
              {
                id: 1,
                text: 'Note B',
                deleted: true,
                createdAt: '2025-01-01T00:01:00.000Z',
                deletedAt: '2025-01-01T01:00:00.000Z',
              },
              {
                id: 2,
                text: 'Note C',
                deleted: false,
                createdAt: '2025-01-01T00:02:00.000Z',
              },
              {
                id: 3,
                text: 'Note D',
                deleted: true,
                createdAt: '2025-01-01T00:03:00.000Z',
                deletedAt: '2025-01-01T01:01:00.000Z',
              },
              {
                id: 4,
                text: 'Note E',
                deleted: false,
                createdAt: '2025-01-01T00:04:00.000Z',
              },
            ],
            nextNoteId: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      vol.writeFileSync(workUnitsPath, JSON.stringify(initialData, null, 2));

      // @step And notes at indices 1 and 3 are soft-deleted

      // When compact-work-unit removes deleted items and renumbers remaining notes
      const { compactWorkUnit } = await import('../compact-work-unit');
      await compactWorkUnit({ workUnitId: 'AUTH-001', force: true, cwd });

      // And compact-work-unit sets nextNoteId = 3 (correct field)
      const dataAfterCompact = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfterCompact = dataAfterCompact.workUnits['AUTH-001'];

      // CRITICAL: Must be nextNoteId, not nextArchitectureNoteId
      expect(workUnitAfterCompact.nextNoteId).toBe(3);
      expect(workUnitAfterCompact).not.toHaveProperty('nextArchitectureNoteId');

      // And user adds new architecture note "Latest architectural decision"
      const { addArchitectureNote } = await import('../add-architecture-note');
      await addArchitectureNote({
        workUnitId: 'AUTH-001',
        note: 'Latest architectural decision',
        cwd,
      });

      // Then new note should get id = 3 (not id = 0)
      const dataAfterAdd = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfterAdd = dataAfterAdd.workUnits['AUTH-001'];

      const newNote = workUnitAfterAdd.architectureNotes.find(
        n => n.text === 'Latest architectural decision'
      );
      expect(newNote).toBeDefined();
      expect(newNote!.id).toBe(3); // Should be 3, not 0

      // And no ID collision should occur with existing notes at indices 0, 1, 2
      const existingIds = workUnitAfterAdd.architectureNotes.map(n => n.id);
      expect(existingIds).toEqual([0, 1, 2, 3]);

      // Verify no duplicate IDs
      const uniqueIds = [...new Set(existingIds)];
      expect(uniqueIds.length).toBe(existingIds.length);
    });
  });

  describe('Scenario: Preserve state sorting through auto-compact (line 49)', () => {
    it('should preserve state sorting when auto-compact is triggered during status transition', async () => {
      // Given work units BOARD-001, BOARD-002, BOARD-003 in done state
      // And done state is sorted by completion time (most recent first)
      const initialData: WorkUnitsData = {
        meta: { version: '0.7.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'First',
            status: 'done',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T10:00:00.000Z',
          },
          'BOARD-002': {
            id: 'BOARD-002',
            title: 'Second',
            status: 'done',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T11:00:00.000Z',
          },
          'BOARD-003': {
            id: 'BOARD-003',
            title: 'Third',
            status: 'done',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T12:00:00.000Z',
          },
          'BOARD-004': {
            id: 'BOARD-004',
            title: 'Fourth with deleted items',
            status: 'validating',
            rules: [
              {
                id: 0,
                text: 'Rule A',
                deleted: false,
                createdAt: '2025-01-01T00:00:00.000Z',
              },
              {
                id: 1,
                text: 'Rule B',
                deleted: true,
                createdAt: '2025-01-01T00:01:00.000Z',
                deletedAt: '2025-01-01T01:00:00.000Z',
              },
              {
                id: 2,
                text: 'Rule C',
                deleted: false,
                createdAt: '2025-01-01T00:02:00.000Z',
              },
            ],
            nextRuleId: 3,
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T13:00:00.000Z',
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BOARD-004'],
          done: ['BOARD-003', 'BOARD-002', 'BOARD-001'], // Sorted most recent first
          blocked: [],
        },
      };

      vol.writeFileSync(workUnitsPath, JSON.stringify(initialData, null, 2));

      // When BOARD-004 is moved to done status with auto-compact
      const { updateWorkUnitStatus } = await import(
        '../update-work-unit-status'
      );
      await updateWorkUnitStatus({
        workUnitId: 'BOARD-004',
        status: 'done',
        cwd,
      });

      // @step And state sorting is applied AFTER auto-compact (not before)

      // Then state sorting should be preserved in final work-units.json
      const dataAfter = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;

      // And done state array should maintain user-defined sort order
      // BOARD-004 should be at the front (most recent)
      // Original order should be preserved: BOARD-003, BOARD-002, BOARD-001
      expect(dataAfter.states.done).toEqual([
        'BOARD-004', // New one added at front
        'BOARD-003',
        'BOARD-002',
        'BOARD-001',
      ]);

      // Verify auto-compact actually happened (deleted rule removed)
      const workUnit004 = dataAfter.workUnits['BOARD-004'];
      expect(workUnit004.rules).toHaveLength(2); // 1 deleted item removed
      expect(workUnit004.rules.every(r => !r.deleted)).toBe(true);
    });
  });
});
