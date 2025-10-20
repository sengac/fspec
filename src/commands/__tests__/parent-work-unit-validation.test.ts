/**
 * Feature: spec/features/parent-work-unit-validation.feature (BUG-006)
 *
 * This test file validates that parent work units can move through workflow
 * without requiring scenarios tagged with @WORK-UNIT-ID.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Parent Work Unit Validation (BUG-006)', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Parent work unit moves to testing without scenarios', () => {
    it('should allow parent work unit to move to testing without scenario validation', async () => {
      // Given I have a parent work unit with children
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'PARENT-001': {
                id: 'PARENT-001',
                title: 'Parent Feature',
                status: 'specifying',
                children: ['CHILD-001', 'CHILD-002'],
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
              'CHILD-001': {
                id: 'CHILD-001',
                title: 'Child 1',
                status: 'done',
                parent: 'PARENT-001',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
              'CHILD-002': {
                id: 'CHILD-002',
                title: 'Child 2',
                status: 'done',
                parent: 'PARENT-001',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            },
            states: {
              backlog: [],
              specifying: ['PARENT-001'],
              testing: [],
              implementing: [],
              validating: [],
              done: ['CHILD-001', 'CHILD-002'],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // And no feature file exists with @PARENT-001 tag
      // (Don't create any feature files)

      // When I move parent work unit to testing
      const result = await updateWorkUnitStatus({
        workUnitId: 'PARENT-001',
        status: 'testing',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);
      expect(result.message).toContain('PARENT-001');
      expect(result.message).toContain('testing');
    });
  });

  describe('Scenario: Leaf work unit requires scenarios', () => {
    it('should fail when leaf work unit has no scenarios', async () => {
      // Given I have a leaf work unit (no children)
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'LEAF-001': {
                id: 'LEAF-001',
                title: 'Leaf Feature',
                status: 'specifying',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            },
            states: {
              backlog: [],
              specifying: ['LEAF-001'],
              testing: [],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // And no feature file exists with @LEAF-001 tag

      // When I try to move leaf work unit to testing
      // Then it should fail with scenario validation error
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'LEAF-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow(/No Gherkin scenarios found/);
    });
  });

  describe('Scenario: Parent work unit moves to done when all children done', () => {
    it('should allow parent to move to done when all children are done', async () => {
      // Given I have a parent work unit with all children done
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'PARENT-001': {
                id: 'PARENT-001',
                title: 'Parent Feature',
                status: 'validating',
                children: ['CHILD-001', 'CHILD-002'],
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
              'CHILD-001': {
                id: 'CHILD-001',
                title: 'Child 1',
                status: 'done',
                parent: 'PARENT-001',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
              'CHILD-002': {
                id: 'CHILD-002',
                title: 'Child 2',
                status: 'done',
                parent: 'PARENT-001',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: [],
              implementing: [],
              validating: ['PARENT-001'],
              done: ['CHILD-001', 'CHILD-002'],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // When I move parent to done
      const result = await updateWorkUnitStatus({
        workUnitId: 'PARENT-001',
        status: 'done',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);
      expect(result.message).toContain('PARENT-001');
      expect(result.message).toContain('done');
    });
  });

  describe('Scenario: Parent work unit blocked when children incomplete', () => {
    it('should fail when trying to mark parent done with incomplete children', async () => {
      // Given I have a parent work unit with incomplete children
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'PARENT-001': {
                id: 'PARENT-001',
                title: 'Parent Feature',
                status: 'validating',
                children: ['CHILD-001', 'CHILD-002'],
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
              'CHILD-001': {
                id: 'CHILD-001',
                title: 'Child 1',
                status: 'done',
                parent: 'PARENT-001',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
              'CHILD-002': {
                id: 'CHILD-002',
                title: 'Child 2',
                status: 'implementing',
                parent: 'PARENT-001',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: [],
              implementing: ['CHILD-002'],
              validating: ['PARENT-001'],
              done: ['CHILD-001'],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // When I try to move parent to done
      // Then it should fail with incomplete children error
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'PARENT-001',
          status: 'done',
          cwd: testDir,
        })
      ).rejects.toThrow(
        /Cannot mark parent as done while children are incomplete/
      );
    });
  });
});
