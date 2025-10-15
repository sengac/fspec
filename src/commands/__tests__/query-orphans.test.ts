/**
 * Feature: spec/features/work-unit-dependency-management.feature
 * Scenario: Detect orphaned work units with no epic or dependencies (@COV-048)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { mkdir, writeFile } from 'fs/promises';
import type { WorkUnit } from '../../types/work-unit.js';

describe('Feature: Work Unit Dependency Management', () => {
  let tempDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Given I have a project with spec directory
    tempDir = mkdtempSync(join(tmpdir(), 'fspec-test-'));
    specDir = join(tempDir, 'spec');
    await mkdir(specDir, { recursive: true });
    workUnitsFile = join(specDir, 'work-units.json');
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Detect orphaned work units with no epic or dependencies', () => {
    it('should list orphaned work units with no epic and no relationships', async () => {
      // And work units exist:
      const workUnits: WorkUnit[] = [
        {
          id: 'AUTH-001',
          title: 'Test: Auth work',
          status: 'implementing',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          epic: 'user-management',
          blocks: ['API-001'],
        },
        {
          id: 'API-001',
          title: 'Test: API work',
          status: 'implementing',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          blockedBy: ['AUTH-001'],
        },
        {
          id: 'ORPHAN-1',
          title: 'Test: Orphaned work 1',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'ORPHAN-2',
          title: 'Test: Orphaned work 2',
          status: 'implementing',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'UI-001',
          title: 'Test: UI work',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          epic: 'user-interface',
        },
      ];

      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec query orphans --output=json"
      const { queryOrphans } = await import('../query-orphans.js');
      const result = await queryOrphans({ cwd: tempDir, output: 'json' });

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
