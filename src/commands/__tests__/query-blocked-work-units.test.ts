/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Find all currently blocked work units
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { queryWorkUnits } from '../query-work-units';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Find all currently blocked work units', () => {
    it('should find all work units with blocked status', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And multiple work units exist with various statuses
      // And 3 work units are in "blocked" status
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'UI-001': {
            id: 'UI-001',
            title: 'Blocked UI work',
            status: 'blocked',
            blockedBy: ['API-001'],
            blockedReason: 'Blocked by API-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'FEAT-001': {
            id: 'FEAT-001',
            title: 'Blocked feature work',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            blockedReason: 'Blocked by AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-001': {
            id: 'DB-001',
            title: 'Blocked database work',
            status: 'blocked',
            blockedBy: ['INFRA-001'],
            blockedReason: 'Blocked by INFRA-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'In progress API work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Done auth work',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'TEST-001': {
            id: 'TEST-001',
            title: 'Testing work',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['TEST-001'],
          implementing: ['API-001'],
          validating: [],
          done: ['AUTH-001'],
          blocked: ['UI-001', 'FEAT-001', 'DB-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec query-work-units --status=blocked"
      const result = await queryWorkUnits({
        status: 'blocked',
        cwd: testDir,
      });

      // Then the output should show 3 blocked work units
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits?.length).toBe(3);

      // And the output should include "UI-001"
      const workUnitIds = result.workUnits?.map(wu => wu.id);
      expect(workUnitIds).toContain('UI-001');

      // And the output should include "FEAT-001"
      expect(workUnitIds).toContain('FEAT-001');

      // And the output should include "DB-001"
      expect(workUnitIds).toContain('DB-001');

      // And each blocked work unit should have a blocked reason
      const ui001 = result.workUnits?.find(wu => wu.id === 'UI-001');
      expect(ui001?.blockedReason).toBe('Blocked by API-001');

      const feat001 = result.workUnits?.find(wu => wu.id === 'FEAT-001');
      expect(feat001?.blockedReason).toBe('Blocked by AUTH-001');

      const db001 = result.workUnits?.find(wu => wu.id === 'DB-001');
      expect(db001?.blockedReason).toBe('Blocked by INFRA-001');
    });
  });
});
