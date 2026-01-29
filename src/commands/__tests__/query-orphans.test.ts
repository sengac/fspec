/**
 * Feature: spec/features/work-unit-dependency-management.feature
 * Scenario: Detect orphaned work units with no epic or dependencies (@COV-048)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir } from 'fs/promises';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';
import type { WorkUnit } from '../../types/work-unit';

describe('Feature: Work Unit Dependency Management', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    // Given I have a project with spec directory
    setup = await setupWorkUnitTest('query-orphans');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Detect orphaned work units with no epic or dependencies', () => {
    it('should list orphaned work units with no epic and no relationships', async () => {
      // And work units exist:
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test: Auth work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            epic: 'user-management',
            blocks: ['API-001'],
          },
          'API-001': {
            id: 'API-001',
            title: 'Test: API work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blockedBy: ['AUTH-001'],
          },
          'ORPHAN-1': {
            id: 'ORPHAN-1',
            title: 'Test: Orphaned work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'ORPHAN-2': {
            id: 'ORPHAN-2',
            title: 'Test: Orphaned work 2',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'UI-001': {
            id: 'UI-001',
            title: 'Test: UI work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            epic: 'user-interface',
          },
        },
        states: {
          backlog: ['ORPHAN-1', 'UI-001'],
          implementing: ['AUTH-001', 'API-001', 'ORPHAN-2'],
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // When I run "fspec query orphans --output=json"
      const { queryOrphans } = await import('../query-orphans');
      const result = await queryOrphans({ cwd: setup.testDir, output: 'json' });

      // Then the output should list "ORPHAN-1" and "ORPHAN-2" as orphaned
      expect(result.orphans).toBeDefined();
      expect(Array.isArray(result.orphans)).toBe(true);

      const orphanIds = result.orphans.map((o: { id: string }) => o.id);
      expect(orphanIds).toContain('ORPHAN-1');
      expect(orphanIds).toContain('ORPHAN-2');

      // And "AUTH-001" should not be listed (has epic)
      expect(orphanIds).not.toContain('AUTH-001');

      // And "API-001" should not be listed (has relationship)
      expect(orphanIds).not.toContain('API-001');

      // And "UI-001" should not be listed (has epic)
      expect(orphanIds).not.toContain('UI-001');

      // And each orphan should show suggested actions: "Assign epic", "Add relationship", "Delete"
      result.orphans.forEach((orphan: { suggestedActions: string[] }) => {
        expect(orphan.suggestedActions).toBeDefined();
        expect(orphan.suggestedActions).toContain('Assign epic');
        expect(orphan.suggestedActions).toContain('Add relationship');
        expect(orphan.suggestedActions).toContain('Delete');
      });
    });
  });
});
