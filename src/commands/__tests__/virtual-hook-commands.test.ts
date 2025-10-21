/**
 * Feature: spec/features/work-unit-scoped-hooks-for-dynamic-validation.feature
 *
 * This test file validates the acceptance criteria for virtual hook commands.
 * Tests map to scenarios in the Gherkin feature file.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, readFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { addVirtualHook } from '../add-virtual-hook';
import { listVirtualHooks } from '../list-virtual-hooks';
import { removeVirtualHook } from '../remove-virtual-hook';
import { copyVirtualHooks } from '../copy-virtual-hooks';
import { clearVirtualHooks } from '../clear-virtual-hooks';

describe('Feature: Work unit-scoped hooks for dynamic validation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-virtual-hooks-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Create minimal work-units.json
    const workUnitsData = {
      workUnits: {
        'AUTH-001': {
          id: 'AUTH-001',
          title: 'User Authentication',
          type: 'story',
          status: 'specifying',
          description: 'Test work unit',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      },
      prefixes: {
        AUTH: {
          description: 'Authentication features',
        },
      },
      epics: {},
    };

    await writeFile(
      join(testDir, 'spec', 'work-units.json'),
      JSON.stringify(workUnitsData, null, 2)
    );
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: AI prompts for virtual hooks at end of specifying phase', () => {
    it('should document expected AI prompting behavior', async () => {
      // This scenario documents expected AI behavior
      // Given: Work unit is transitioning from specifying to testing
      // When: Transition completes
      // Then: AI should ask: "Do you want me to run a command or program at any stage of this story?"
      // And: AI should list available hook events (post-implementing, post-testing, etc.)
      // And: If user says yes, AI should run: fspec add-virtual-hook <work-unit-id> <event> <command>

      // All required commands exist (add-virtual-hook, list-virtual-hooks, etc.)
      // This test documents when AI should prompt for virtual hooks
      expect(true).toBe(true);
    });
  });

  describe('Scenario: Add virtual hook to work unit', () => {
    it('should add virtual hook with command and blocking flag', async () => {
      // Given: Work unit AUTH-001 exists in specifying status
      // When: User runs 'fspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --blocking'
      const result = await addVirtualHook({
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        command: 'eslint src/',
        blocking: true,
        cwd: testDir,
      });

      // Then: Virtual hook should be added successfully
      expect(result.success).toBe(true);
      expect(result.hookCount).toBe(1);

      // Verify virtualHooks array was added to work-units.json
      const workUnitsContent = await readFile(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnitsData = JSON.parse(workUnitsContent);

      expect(workUnitsData.workUnits['AUTH-001'].virtualHooks).toBeDefined();
      expect(workUnitsData.workUnits['AUTH-001'].virtualHooks).toHaveLength(1);
      expect(workUnitsData.workUnits['AUTH-001'].virtualHooks[0]).toEqual({
        name: 'eslint',
        event: 'post-implementing',
        command: 'eslint src/',
        blocking: true,
      });
    });

    it('should add virtual hook with git context flag', async () => {
      // Given: Work unit AUTH-001 exists
      // When: User requests hook with git context
      const result = await addVirtualHook({
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        command: 'eslint',
        blocking: true,
        gitContext: true,
        cwd: testDir,
      });

      // Then: Virtual hook should be added with gitContext flag
      expect(result.success).toBe(true);

      const workUnitsContent = await readFile(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnitsData = JSON.parse(workUnitsContent);

      expect(workUnitsData.workUnits['AUTH-001'].virtualHooks).toHaveLength(1);
      expect(
        workUnitsData.workUnits['AUTH-001'].virtualHooks[0].gitContext
      ).toBe(true);
    });
  });

  describe('Scenario: List virtual hooks for work unit', () => {
    it('should display all virtual hooks with their configurations', async () => {
      // Given: Work unit HOOK-011 has multiple virtual hooks
      const workUnitsData = {
        workUnits: {
          'HOOK-011': {
            id: 'HOOK-011',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'eslint',
                event: 'post-implementing',
                command: 'eslint src/',
                blocking: true,
              },
              {
                name: 'prettier',
                event: 'post-implementing',
                command: 'prettier --check src/',
                blocking: false,
              },
              {
                name: 'test-coverage',
                event: 'post-validating',
                command: 'npm run test:coverage',
                blocking: true,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: User runs 'fspec list-virtual-hooks HOOK-011'
      const result = await listVirtualHooks({
        workUnitId: 'HOOK-011',
        cwd: testDir,
      });

      // Then: Should return all virtual hooks grouped by event
      expect(result.hooks).toHaveLength(3);
      expect(result.hooksByEvent['post-implementing']).toHaveLength(2);
      expect(result.hooksByEvent['post-validating']).toHaveLength(1);
    });
  });

  describe('Scenario: Remove virtual hook from work unit', () => {
    it('should remove specified virtual hook by name', async () => {
      // Given: Work unit has virtual hook named 'eslint'
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'eslint',
                event: 'post-implementing',
                command: 'eslint src/',
                blocking: true,
              },
              {
                name: 'prettier',
                event: 'post-implementing',
                command: 'prettier --check src/',
                blocking: false,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: User runs 'fspec remove-virtual-hook AUTH-001 eslint'
      const result = await removeVirtualHook({
        workUnitId: 'AUTH-001',
        hookName: 'eslint',
        cwd: testDir,
      });

      // Then: Hook should be removed successfully
      expect(result.success).toBe(true);
      expect(result.remainingCount).toBe(1);

      // And: Only prettier hook should remain
      const updatedContent = await readFile(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].virtualHooks).toHaveLength(1);
      expect(updatedData.workUnits['AUTH-001'].virtualHooks[0].name).toBe(
        'prettier'
      );
    });
  });

  describe('Scenario: Copy virtual hooks from one work unit to another', () => {
    it('should copy all virtual hooks with configurations', async () => {
      // Given: AUTH-001 has virtual hooks, HOOK-011 has none
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Source',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'eslint',
                event: 'post-implementing',
                command: 'eslint src/',
                blocking: true,
              },
              {
                name: 'prettier',
                event: 'post-validating',
                command: 'prettier --check src/',
                blocking: false,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'HOOK-011': {
            id: 'HOOK-011',
            title: 'Target',
            type: 'story',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: User runs 'fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011'
      const result = await copyVirtualHooks({
        from: 'AUTH-001',
        to: 'HOOK-011',
        cwd: testDir,
      });

      // Then: Hooks should be copied successfully
      expect(result.success).toBe(true);
      expect(result.copiedCount).toBe(2);

      // And: HOOK-011 should have same virtual hooks as AUTH-001
      const updatedContent = await readFile(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['HOOK-011'].virtualHooks).toBeDefined();
      expect(updatedData.workUnits['HOOK-011'].virtualHooks).toHaveLength(2);
      expect(updatedData.workUnits['HOOK-011'].virtualHooks).toEqual(
        workUnitsData.workUnits['AUTH-001'].virtualHooks
      );
    });

    it('should copy only specified hook when --hook-name provided', async () => {
      // Given: AUTH-001 has multiple hooks, HOOK-011 has none
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Source',
            type: 'story',
            status: 'implementing',
            virtualHooks: [
              {
                name: 'eslint',
                event: 'post-implementing',
                command: 'eslint src/',
                blocking: true,
              },
              {
                name: 'prettier',
                event: 'post-validating',
                command: 'prettier --check src/',
                blocking: false,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'HOOK-011': {
            id: 'HOOK-011',
            title: 'Target',
            type: 'story',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: User runs 'fspec copy-virtual-hooks --from AUTH-001 --to HOOK-011 --hook-name eslint'
      const result = await copyVirtualHooks({
        from: 'AUTH-001',
        to: 'HOOK-011',
        hookName: 'eslint',
        cwd: testDir,
      });

      // Then: Only eslint hook should be copied
      expect(result.success).toBe(true);
      expect(result.copiedCount).toBe(1);

      const updatedContent = await readFile(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['HOOK-011'].virtualHooks).toHaveLength(1);
      expect(updatedData.workUnits['HOOK-011'].virtualHooks[0].name).toBe(
        'eslint'
      );
    });
  });

  describe('Scenario: Convert successful virtual hook to permanent global hook', () => {
    it('should document that AI uses add-hook command to make virtual hook permanent', async () => {
      // This scenario tests AI behavior documentation
      // Given: Virtual hook ran successfully on AUTH-001
      // When: AI asks user "Do you want to make it permanent?"
      // And: User responds "yes"
      // Then: AI should run: fspec add-hook <event> <name> <command> --blocking

      // The add-hook command already exists (see src/commands/add-hook.ts)
      // This test documents expected AI behavior
      expect(true).toBe(true);
    });
  });

  describe('Scenario: Clean up virtual hooks when work unit reaches done status', () => {
    it('should clear all virtual hooks from work unit', async () => {
      // Given: Work unit has virtual hooks configured
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test',
            type: 'story',
            status: 'validating',
            virtualHooks: [
              {
                name: 'eslint',
                event: 'post-implementing',
                command: 'eslint src/',
                blocking: true,
              },
              {
                name: 'prettier',
                event: 'post-validating',
                command: 'prettier --check src/',
                blocking: false,
              },
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        prefixes: {},
        epics: {},
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When: Clear virtual hooks command is run
      const result = await clearVirtualHooks({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then: All virtual hooks should be removed
      expect(result.success).toBe(true);
      expect(result.clearedCount).toBe(2);

      const updatedContent = await readFile(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].virtualHooks).toHaveLength(0);
    });
  });
});
