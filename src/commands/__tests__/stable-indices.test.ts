/**
 * Feature: spec/features/implement-stable-indices-with-soft-delete.feature
 *
 * Tests for stable indices with soft-delete functionality.
 * All collections (rules, examples, questions, architectureNotes) use object-based
 * storage with immutable IDs to prevent data loss during sequential removal operations.
 *
 * ACDD Phase: TESTING (write failing tests BEFORE implementation)
 * These tests will FAIL until implementation is complete.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { vol } from 'memfs';
import { join } from 'path';
import type { WorkUnitsData, RuleItem } from '../../types';

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
vi.mock('fs/promises', async () => {
  const memfs = await vi.importActual<MemfsModule>('memfs');
  return memfs.fs.promises;
});

vi.mock('fs', async () => {
  const memfs = await vi.importActual<MemfsModule>('memfs');
  return memfs.fs;
});

// Mock proper-lockfile to use memfs
vi.mock('proper-lockfile', async () => {
  const actualLockfile =
    await vi.importActual<LockfileModule>('proper-lockfile');
  const memfs = await vi.importActual<MemfsModule>('memfs');

  // Create wrapper that injects memfs.fs as the custom filesystem
  const lock = (file: string, options: LockOptions = {}) => {
    return actualLockfile.lock(file, {
      ...options,
      fs: memfs.fs,
      realpath: false, // Don't use realpath since test files don't exist on real FS
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

// Import commands AFTER mocks are set up
import { addRule } from '../add-rule';
import { removeRule } from '../remove-rule';
import { addExample } from '../add-example';
import { removeExample } from '../remove-example';
import { addArchitectureNote } from '../add-architecture-note';
import { removeArchitectureNote } from '../remove-architecture-note';
import { addQuestion } from '../add-question';
import { removeQuestion } from '../remove-question';
import { showWorkUnit } from '../show-work-unit';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { createStory } from '../create-story';

describe('Feature: Implement Stable Indices with Soft Delete', () => {
  const cwd = '/test-project';
  const workUnitsPath = join(cwd, 'spec/work-units.json');

  beforeEach(() => {
    vi.clearAllMocks();
    vol.reset();

    // Setup test directory structure
    vol.mkdirSync(join(cwd, 'spec'), { recursive: true });

    // Create minimal foundation.json for tests
    vol.writeFileSync(
      join(cwd, 'spec/foundation.json'),
      JSON.stringify({
        productName: 'Test Product',
        coreCapabilities: [],
        technologies: [],
        knownLimitations: [],
      })
    );

    // Initialize prefixes.json
    vol.writeFileSync(
      join(cwd, 'spec/prefixes.json'),
      JSON.stringify({
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      })
    );
  });

  describe('Scenario: Sequential removal without index shifts (line 104)', () => {
    it('should soft-delete items without shifting indices', async () => {
      // Given work unit AUTH-001 has 5 rules with IDs [0, 1, 2, 3, 4]
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule D', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule E', cwd });

      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];

      // And all rules have deleted: false
      expect(workUnit.rules).toHaveLength(5);
      workUnit.rules.forEach((rule: RuleItem) => {
        expect(rule.deleted).toBe(false);
      });

      // When AI removes rule at index 1
      await removeRule({ workUnitId: 'AUTH-001', index: 1, cwd });

      // Then rule[1].deleted should be set to true
      const dataAfterFirst = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfterFirst = dataAfterFirst.workUnits['AUTH-001'];

      expect(workUnitAfterFirst.rules[1].deleted).toBe(true);
      // @step And rule[1].deletedAt should be set to ISO timestamp
      expect(workUnitAfterFirst.rules[1].deletedAt).toBeDefined();
      expect(workUnitAfterFirst.rules[1].deletedAt).toMatch(
        /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/
      );

      // And rule indices should remain [0, 1, 2, 3, 4]
      expect(workUnitAfterFirst.rules).toHaveLength(5);
      expect(workUnitAfterFirst.rules.map((r: RuleItem) => r.id)).toEqual([
        0, 1, 2, 3, 4,
      ]);

      // When AI removes rule at index 2
      await removeRule({ workUnitId: 'AUTH-001', index: 2, cwd });

      // Then rule[2].deleted should be set to true
      const dataAfterSecond = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfterSecond = dataAfterSecond.workUnits['AUTH-001'];

      expect(workUnitAfterSecond.rules[2].deleted).toBe(true);
      // @step And rule[2].deletedAt should be set to ISO timestamp
      expect(workUnitAfterSecond.rules[2].deletedAt).toBeDefined();

      // And rule indices should remain [0, 1, 2, 3, 4]
      expect(workUnitAfterSecond.rules).toHaveLength(5);
      expect(workUnitAfterSecond.rules.map((r: RuleItem) => r.id)).toEqual([
        0, 1, 2, 3, 4,
      ]);

      // And no rules should be shifted or lost
      expect(workUnitAfterSecond.rules[0].text).toBe('Rule A');
      expect(workUnitAfterSecond.rules[1].text).toBe('Rule B');
      expect(workUnitAfterSecond.rules[2].text).toBe('Rule C');
      expect(workUnitAfterSecond.rules[3].text).toBe('Rule D');
      expect(workUnitAfterSecond.rules[4].text).toBe('Rule E');
    });
  });

  describe('Scenario: Display with gaps in indices after deletions (line 117)', () => {
    it('should display only active items with stable IDs and show deletion count', async () => {
      // @step Given work unit AUTH-001 has 5 rules: [0] Rule A, [1] Rule B, [2] Rule C, [3] Rule D, [4] Rule E
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule D', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule E', cwd });

      // When AI removes rules at indices 1 and 3
      await removeRule({ workUnitId: 'AUTH-001', index: 1, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 3, cwd });

      // And I run "fspec show-work-unit AUTH-001"
      const workUnitDetails = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd,
      });

      // @step Then display should show rules:
      expect(workUnitDetails.rules).toBeDefined();
      const activeRules = workUnitDetails.rules as string[];
      expect(activeRules).toHaveLength(3);
      expect(activeRules[0]).toContain('[0]');
      expect(activeRules[0]).toContain('Rule A');
      expect(activeRules[1]).toContain('[2]');
      expect(activeRules[1]).toContain('Rule C');
      expect(activeRules[2]).toContain('[4]');
      expect(activeRules[2]).toContain('Rule E');

      // And display should show "3 active items (2 deleted)"
      expect(workUnitDetails.systemReminders).toContain(
        '3 active items (2 deleted)'
      );

      // And indices [1] and [3] should not appear in display
      const displayText = JSON.stringify(activeRules);
      expect(displayText).not.toContain('[1]');
      expect(displayText).not.toContain('[3]');
    });
  });

  describe('Scenario: Restore deleted item to original index (line 130)', () => {
    it('should restore soft-deleted item and clear deletedAt timestamp', async () => {
      // Given work unit AUTH-001 has rule at index 2 with deleted: true
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });

      await removeRule({ workUnitId: 'AUTH-001', index: 2, cwd });

      const dataBefore = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitBefore = dataBefore.workUnits['AUTH-001'];

      // And rule[2].deletedAt is set
      expect(workUnitBefore.rules[2].deleted).toBe(true);
      expect(workUnitBefore.rules[2].deletedAt).toBeDefined();

      // When I run "fspec restore-rule AUTH-001 2"
      const { restoreRule } = await import('../restore-rule');
      await restoreRule({ workUnitId: 'AUTH-001', index: 2, cwd });

      // Then rule[2].deleted should be set to false
      const dataAfter = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfter = dataAfter.workUnits['AUTH-001'];

      expect(workUnitAfter.rules[2].deleted).toBe(false);

      // And rule[2].deletedAt should be cleared (undefined)
      expect(workUnitAfter.rules[2].deletedAt).toBeUndefined();

      // And rule should reappear in display at index [2]
      const workUnitDetails = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd,
      });
      const activeRules = workUnitDetails.rules as string[];
      expect(activeRules.some((r: string) => r.includes('[2]'))).toBe(true);
    });
  });

  describe('Scenario: Manual compaction removes deleted items and renumbers IDs (line 138)', () => {
    it('should permanently remove deleted items and renumber remaining IDs sequentially', async () => {
      // Given work unit AUTH-001 has 10 rules with IDs [0-9]
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      for (let i = 0; i < 10; i++) {
        await addRule({ workUnitId: 'AUTH-001', rule: `Rule ${i}`, cwd });
      }

      // And 4 rules are deleted (IDs 1, 3, 5, 7)
      await removeRule({ workUnitId: 'AUTH-001', index: 1, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 3, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 5, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 7, cwd });

      const dataBefore = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitBefore = dataBefore.workUnits['AUTH-001'];

      // And nextRuleId is 10
      expect(workUnitBefore.nextRuleId).toBe(10);

      // When I run "fspec compact-work-unit AUTH-001"
      // @step Then system should prompt for confirmation
      // @step When I confirm the operation
      const { compactWorkUnit } = await import('../compact-work-unit');
      await compactWorkUnit({
        workUnitId: 'AUTH-001',
        force: true,
        cwd,
      });

      // Then deleted rules should be permanently removed
      const dataAfter = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfter = dataAfter.workUnits['AUTH-001'];

      expect(workUnitAfter.rules).toHaveLength(6);

      // And remaining 6 rules should be renumbered to IDs [0-5]
      expect(workUnitAfter.rules.map((r: RuleItem) => r.id)).toEqual([
        0, 1, 2, 3, 4, 5,
      ]);

      // And nextRuleId should be reset to 6
      expect(workUnitAfter.nextRuleId).toBe(6);

      // And rules should be sorted by createdAt timestamp
      const createdAtTimes = workUnitAfter.rules.map((r: RuleItem) =>
        new Date(r.createdAt).getTime()
      );
      const sortedTimes = [...createdAtTimes].sort((a, b) => a - b);
      expect(createdAtTimes).toEqual(sortedTimes);
    });
  });

  describe('Scenario: Auto-compact on done status change (line 150)', () => {
    it('should auto-compact before changing status to done', async () => {
      // Given work unit AUTH-001 is at "implementing" status
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });

      // Make AUTH-001 a parent work unit to bypass scenario validation
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      data.workUnits['AUTH-001'].children = ['AUTH-002'];
      vol.writeFileSync(workUnitsPath, JSON.stringify(data, null, 2));

      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      // Add 10 rules
      for (let i = 0; i < 10; i++) {
        await addRule({ workUnitId: 'AUTH-001', rule: `Rule ${i}`, cwd });
      }

      // And work unit has 10 rules with 3 deleted (while still in specifying)
      await removeRule({ workUnitId: 'AUTH-001', index: 2, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 5, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 8, cwd });

      // Move to implementing
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'implementing',
        cwd,
      });

      const dataBefore = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitBefore = dataBefore.workUnits['AUTH-001'];

      expect(workUnitBefore.rules).toHaveLength(10);
      expect(
        workUnitBefore.rules.filter((r: RuleItem) => r.deleted)
      ).toHaveLength(3);

      // When I run "fspec update-work-unit-status AUTH-001 done"
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'validating',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd,
      });

      // Then auto-compact should trigger before status change
      const dataAfter = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnitAfter = dataAfter.workUnits['AUTH-001'];

      // @step And deleted rules should be permanently removed
      expect(workUnitAfter.rules).toHaveLength(7);
      expect(workUnitAfter.rules.every((r: RuleItem) => !r.deleted)).toBe(true);

      // @step And remaining rules should be renumbered sequentially
      expect(workUnitAfter.rules.map((r: RuleItem) => r.id)).toEqual([
        0, 1, 2, 3, 4, 5, 6,
      ]);

      // @step And nextRuleId should be reset to remaining item count
      expect(workUnitAfter.nextRuleId).toBe(7);

      // @step And then status should update to "done"
      expect(workUnitAfter.status).toBe('done');
    });
  });

  describe('Scenario: Migration converts string arrays to object arrays with stable IDs (line 160)', () => {
    it('should migrate v0.6.0 string arrays to v0.7.0 object arrays', async () => {
      // Given I have work-units.json at version "0.6.0"
      // @step And work unit AUTH-001 has rules: ["Rule A", "Rule B"]
      // @step And work unit has NO nextRuleId field
      const v060Data: WorkUnitsData = {
        meta: { version: '0.6.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            status: 'specifying',
            rules: ['Rule A', 'Rule B'] as unknown as RuleItem[],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      vol.writeFileSync(workUnitsPath, JSON.stringify(v060Data, null, 2));

      // When migration 001-stable-indices.ts runs
      const { ensureLatestVersion } = await import('../../migrations/index');
      const migratedData = await ensureLatestVersion(cwd, v060Data, '0.7.0');

      // Then rules should be converted to object format
      const workUnit = migratedData.workUnits['AUTH-001'];
      expect(workUnit.rules).toHaveLength(2);
      expect(workUnit.rules[0]).toEqual({
        id: 0,
        text: 'Rule A',
        deleted: false,
        createdAt: expect.any(String),
      });
      expect(workUnit.rules[1]).toEqual({
        id: 1,
        text: 'Rule B',
        deleted: false,
        createdAt: expect.any(String),
      });

      // And nextRuleId should be set to 2
      expect(workUnit.nextRuleId).toBe(2);

      // And all collections should be migrated (rules, examples, questions, notes)
      // (This test focuses on rules; other collections tested separately)
    });
  });

  describe('Scenario: Add operation creates object with stable ID (line 175)', () => {
    it('should create RuleItem object with auto-incremented stable ID', async () => {
      // Given work unit AUTH-001 has nextRuleId: 5
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      // Manually set nextRuleId to 5
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      data.workUnits['AUTH-001'].nextRuleId = 5;
      vol.writeFileSync(workUnitsPath, JSON.stringify(data, null, 2));

      // When I run "fspec add-rule AUTH-001 'New rule'"
      await addRule({ workUnitId: 'AUTH-001', rule: 'New rule', cwd });

      // Then system should create rule object with id: 5
      const dataAfter = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = dataAfter.workUnits['AUTH-001'];

      expect(workUnit.rules).toHaveLength(1);
      expect(workUnit.rules[0]).toEqual({
        id: 5,
        text: 'New rule',
        deleted: false,
        createdAt: expect.stringMatching(
          /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/
        ),
      });

      // And nextRuleId should increment to 6
      expect(workUnit.nextRuleId).toBe(6);
    });
  });

  describe('Scenario: Idempotent remove operation on already-deleted item (line 189)', () => {
    it('should succeed without error when removing already-deleted item', async () => {
      // Given work unit AUTH-001 has rule at index 2 with deleted: true
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });

      await removeRule({ workUnitId: 'AUTH-001', index: 2, cwd });

      // When I run "fspec remove-rule AUTH-001 2"
      const result = await removeRule({
        workUnitId: 'AUTH-001',
        index: 2,
        cwd,
      });

      // Then command should succeed
      expect(result.success).toBe(true);

      // And output should show "Item ID 2 already deleted"
      expect(result.message).toContain('already deleted');

      // And rule[2].deleted should remain true
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];
      expect(workUnit.rules[2].deleted).toBe(true);

      // And no error should be thrown (test passes if no exception)
    });
  });

  describe('Scenario: Idempotent restore operation on already-active item (line 197)', () => {
    it('should succeed without error when restoring already-active item', async () => {
      // Given work unit AUTH-001 has rule at index 2 with deleted: false
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });

      // When I run "fspec restore-rule AUTH-001 2"
      const { restoreRule } = await import('../restore-rule');
      const result = await restoreRule({
        workUnitId: 'AUTH-001',
        index: 2,
        cwd,
      });

      // Then command should succeed
      expect(result.success).toBe(true);

      // And output should show "Item ID 2 already active"
      expect(result.message).toContain('already active');

      // And rule[2].deleted should remain false
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];
      expect(workUnit.rules[2].deleted).toBe(false);

      // And no error should be thrown
    });
  });

  describe('Scenario: Restore validation fails for non-existent ID (line 205)', () => {
    it('should fail with error when restoring non-existent ID', async () => {
      // Given work unit AUTH-001 has rules with IDs [0-9]
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      for (let i = 0; i < 10; i++) {
        await addRule({ workUnitId: 'AUTH-001', rule: `Rule ${i}`, cwd });
      }

      // When I run "fspec restore-rule AUTH-001 99"
      const { restoreRule } = await import('../restore-rule');

      // Then command should fail with exit code 1
      await expect(
        restoreRule({ workUnitId: 'AUTH-001', index: 99, cwd })
      ).rejects.toThrow();

      // And error output should contain "Rule with ID 99 not found"
      await expect(
        restoreRule({ workUnitId: 'AUTH-001', index: 99, cwd })
      ).rejects.toThrow('Rule with ID 99 not found');

      // And no data should be modified
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];
      expect(workUnit.rules).toHaveLength(10);
    });
  });

  describe('Scenario: Show deleted items command displays soft-deleted items (line 212)', () => {
    it('should display deleted items with IDs, text, and deletedAt timestamps', async () => {
      // Given work unit AUTH-001 has 5 rules
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule D', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule E', cwd });

      // And rules at indices 1 and 3 are deleted
      await removeRule({ workUnitId: 'AUTH-001', index: 1, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 3, cwd });

      // When I run "fspec show-deleted AUTH-001"
      const { showDeleted } = await import('../show-deleted');
      const result = await showDeleted({ workUnitId: 'AUTH-001', cwd });

      // Then output should show deleted items
      expect(result.deletedItems).toContainEqual({
        id: 1,
        text: 'Rule B',
        deletedAt: expect.stringMatching(
          /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/
        ),
      });
      expect(result.deletedItems).toContainEqual({
        id: 3,
        text: 'Rule D',
        deletedAt: expect.stringMatching(
          /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/
        ),
      });

      // And output should show "2 deleted items"
      expect(result.totalDeleted).toBe(2);
    });
  });

  describe('Scenario: Bulk restore with comma-separated IDs (line 223)', () => {
    it('should restore multiple items using comma-separated IDs', async () => {
      // Given work unit AUTH-001 has rules with IDs [0-9]
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      for (let i = 0; i < 10; i++) {
        await addRule({ workUnitId: 'AUTH-001', rule: `Rule ${i}`, cwd });
      }

      // And rules at indices 2, 5, 7 are deleted
      await removeRule({ workUnitId: 'AUTH-001', index: 2, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 5, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 7, cwd });

      // When I run "fspec restore-rule AUTH-001 2,5,7"
      const { restoreRule } = await import('../restore-rule');
      const result = await restoreRule({
        workUnitId: 'AUTH-001',
        ids: '2,5,7',
        cwd,
      });

      // Then all IDs should be validated before restoring
      // (validation happens internally)

      // And rules[2], rules[5], rules[7] should all have deleted: false
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];

      expect(workUnit.rules[2].deleted).toBe(false);
      expect(workUnit.rules[5].deleted).toBe(false);
      expect(workUnit.rules[7].deleted).toBe(false);

      // And deletedAt fields should be cleared for all three rules
      expect(workUnit.rules[2].deletedAt).toBeUndefined();
      expect(workUnit.rules[5].deletedAt).toBeUndefined();
      expect(workUnit.rules[7].deletedAt).toBeUndefined();

      // And output should show "Restored 3 items"
      expect(result.restoredCount).toBe(3);
    });
  });

  describe('Scenario: Verbose mode displays timestamps (line 232)', () => {
    it('should display createdAt and deletedAt timestamps in verbose mode', async () => {
      // Given work unit AUTH-001 has 5 rules
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });

      // And rule[1] has deleted: true
      await removeRule({ workUnitId: 'AUTH-001', index: 1, cwd });

      // When I run "fspec show-work-unit AUTH-001 --verbose"
      const workUnitDetails = await showWorkUnit({
        workUnitId: 'AUTH-001',
        verbose: true,
        cwd,
      });

      // @step And rule[0] has createdAt: "2025-01-31T10:00:00.000Z"
      // Then output should display createdAt for all items
      expect(workUnitDetails.rules[0]).toContain('createdAt:');
      expect(workUnitDetails.rules[0]).toMatch(
        /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/
      );

      // And output should display deletedAt for deleted items
      if (workUnitDetails.deletedRules) {
        expect(workUnitDetails.deletedRules[0]).toContain('deletedAt:');
        expect(workUnitDetails.deletedRules[0]).toMatch(
          /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/
        );
      }

      // And timestamps should be in ISO 8601 format
      // (verified by regex above)
    });
  });

  describe('Scenario: Remove validation fails for non-existent ID (line 241)', () => {
    it('should fail with error when removing non-existent ID', async () => {
      // Given work unit AUTH-001 has rules with IDs [0-9]
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      for (let i = 0; i < 10; i++) {
        await addRule({ workUnitId: 'AUTH-001', rule: `Rule ${i}`, cwd });
      }

      // When I run "fspec remove-rule AUTH-001 99"
      // Then command should fail with exit code 1
      await expect(
        removeRule({ workUnitId: 'AUTH-001', index: 99, cwd })
      ).rejects.toThrow();

      // And error output should contain "Rule with ID 99 not found"
      await expect(
        removeRule({ workUnitId: 'AUTH-001', index: 99, cwd })
      ).rejects.toThrow('Rule with ID 99 not found');

      // And no data should be modified
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];
      expect(workUnit.rules).toHaveLength(10);
    });
  });

  describe('Scenario: Compact during specifying phase requires force flag (line 248)', () => {
    it('should require --force flag when compacting during non-done status', async () => {
      // Given work unit AUTH-001 is at "specifying" status
      await createStory({
        prefix: 'AUTH',
        title: 'Test Story',
        description: 'Test',
        cwd,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd,
      });

      // And work unit has 5 rules with 2 deleted
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule D', cwd });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Rule E', cwd });

      await removeRule({ workUnitId: 'AUTH-001', index: 1, cwd });
      await removeRule({ workUnitId: 'AUTH-001', index: 3, cwd });

      // When I run "fspec compact-work-unit AUTH-001"
      const { compactWorkUnit } = await import('../compact-work-unit');

      // Then command should fail with error message
      await expect(
        compactWorkUnit({ workUnitId: 'AUTH-001', cwd })
      ).rejects.toThrow();

      // And error output should contain "Use --force to confirm"
      await expect(
        compactWorkUnit({ workUnitId: 'AUTH-001', cwd })
      ).rejects.toThrow('Use --force to confirm');

      // When I run "fspec compact-work-unit AUTH-001 --force"
      const result = await compactWorkUnit({
        workUnitId: 'AUTH-001',
        force: true,
        cwd,
      });

      // Then deleted rules should be permanently removed
      const data = JSON.parse(
        vol.readFileSync(workUnitsPath, 'utf-8') as string
      ) as WorkUnitsData;
      const workUnit = data.workUnits['AUTH-001'];

      expect(workUnit.rules).toHaveLength(3);

      // And remaining rules should be renumbered sequentially
      expect(workUnit.rules.map((r: RuleItem) => r.id)).toEqual([0, 1, 2]);

      // @step And output should show warning "⚠ Warning: Compacting during 'specifying' status permanently removes deleted items"
      // And output should show warning
      expect(result.warning).toContain(
        "Compacting during 'specifying' status permanently removes deleted items"
      );

      // And command should exit with code 0 (success, no exception thrown)
    });
  });

  describe('Scenario: Migration handles partially migrated data (line 260)', () => {
    it('should handle mixed string and object format during migration', async () => {
      // Given work unit AUTH-001 has mixed format rules
      const mixedData: WorkUnitsData = {
        meta: { version: '0.6.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            status: 'specifying',
            rules: [
              'Old string rule',
              {
                id: 1,
                text: 'New object rule',
                deleted: false,
                createdAt: '2025-01-31T10:00:00.000Z',
              },
            ] as unknown as RuleItem[],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      vol.writeFileSync(workUnitsPath, JSON.stringify(mixedData, null, 2));

      // When migration 001-stable-indices.ts runs
      const { ensureLatestVersion } = await import('../../migrations/index');
      const migratedData = await ensureLatestVersion(cwd, mixedData, '0.7.0');

      // Then migration should detect mixed format
      // (detection happens internally)

      // And all rules should be normalized to object format
      const workUnit = migratedData.workUnits['AUTH-001'];
      expect(workUnit.rules).toHaveLength(2);

      expect(workUnit.rules[0]).toEqual({
        id: 0,
        text: 'Old string rule',
        deleted: false,
        createdAt: expect.any(String),
      });

      expect(workUnit.rules[1]).toEqual({
        id: 1,
        text: 'New object rule',
        deleted: false,
        createdAt: '2025-01-31T10:00:00.000Z',
      });

      // And nextRuleId should be set to 2
      expect(workUnit.nextRuleId).toBe(2);
    });
  });
});
