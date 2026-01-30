/**
 * Feature: spec/features/parent-work-unit-validation.feature (BUG-006)
 *
 * This test file validates that parent work units can move through workflow
 * without requiring scenarios tagged with @WORK-UNIT-ID.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { setupWorkUnitTest, type WorkUnitTestSetup } from '../../test-helpers/universal-test-setup';

describe('Feature: Parent Work Unit Validation (BUG-006)', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('parent-work-unit-validation');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Parent work unit moves to testing without scenarios', () => {
    it('should allow parent work unit to move to testing without scenario validation', async () => {
      // Given I have a parent work unit with children
      await writeFile(
        setup.workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'PARENT-001': {
                id: 'PARENT-001',
                title: 'Parent Feature',
                type: 'story',
                status: 'specifying',
                children: ['CHILD-001', 'CHILD-002'],
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                rules: [
                  'Parent work units can move to testing without scenario validation',
                ],
                examples: ['Parent with completed children moves to testing'],
                architectureNotes: [
                  'Implementation: Parents bypass scenario checks but still need ACDD data',
                ],
                attachments: ['spec/attachments/PARENT-001/ast-research.json'],
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
        cwd: setup.testDir,
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
        setup.workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'LEAF-001': {
                id: 'LEAF-001',
                title: 'Leaf Feature',
                type: 'story',
                status: 'specifying',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                rules: ['Leaf work units must have scenarios before testing'],
                examples: [
                  'Work unit with no children requires scenario validation',
                ],
                architectureNotes: [
                  'Implementation: Scenario validation applies to leaf work units only',
                ],
                attachments: ['spec/attachments/LEAF-001/ast-research.json'],
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
          cwd: setup.testDir,
        })
      ).rejects.toThrow(/No Gherkin scenarios found/);
    });
  });

  describe('Scenario: Parent work unit moves to done when all children done', () => {
    it('should allow parent to move to done when all children are done', async () => {
      // Given I have a parent work unit with all children done
      await writeFile(
        setup.workUnitsFile,
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
        cwd: setup.testDir,
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
        setup.workUnitsFile,
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
          cwd: setup.testDir,
        })
      ).rejects.toThrow(
        /Cannot mark parent as done while children are incomplete/
      );
    });
  });
});
