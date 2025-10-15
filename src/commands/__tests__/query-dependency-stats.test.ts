/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Query dependency stats shows metrics across all work units
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { queryDependencyStats } from '../query-dependency-stats';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Query dependency stats shows metrics across all work units', () => {
    it('should calculate dependency statistics correctly', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And multiple work units exist with various dependencies
      // And 5 work units have blockers (blockedBy)
      // And 3 work units are blocking others (blocks)
      // And 8 work units have soft dependencies (dependsOn)
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          // 3 work units that are blocking others
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Blocker 1',
            status: 'implementing',
            blocks: ['UI-001', 'UI-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Blocker 2',
            status: 'implementing',
            blocks: ['UI-003'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'Blocker 3',
            status: 'implementing',
            blocks: ['UI-004', 'UI-005'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          // 5 work units with blockers
          'UI-001': {
            id: 'UI-001',
            title: 'Blocked 1',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-002': {
            id: 'UI-002',
            title: 'Blocked 2',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-003': {
            id: 'UI-003',
            title: 'Blocked 3',
            status: 'blocked',
            blockedBy: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-004': {
            id: 'UI-004',
            title: 'Blocked 4',
            status: 'blocked',
            blockedBy: ['API-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-005': {
            id: 'UI-005',
            title: 'Blocked 5',
            status: 'blocked',
            blockedBy: ['API-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          // 8 work units with soft dependencies
          'SOFT-001': {
            id: 'SOFT-001',
            title: 'Soft dep 1',
            status: 'backlog',
            dependsOn: ['DB-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-002': {
            id: 'SOFT-002',
            title: 'Soft dep 2',
            status: 'backlog',
            dependsOn: ['DB-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-003': {
            id: 'SOFT-003',
            title: 'Soft dep 3',
            status: 'backlog',
            dependsOn: ['DB-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-004': {
            id: 'SOFT-004',
            title: 'Soft dep 4',
            status: 'backlog',
            dependsOn: ['DB-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-005': {
            id: 'SOFT-005',
            title: 'Soft dep 5',
            status: 'backlog',
            dependsOn: ['DB-003'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-006': {
            id: 'SOFT-006',
            title: 'Soft dep 6',
            status: 'backlog',
            dependsOn: ['DB-003'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-007': {
            id: 'SOFT-007',
            title: 'Soft dep 7',
            status: 'backlog',
            dependsOn: ['DB-004'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOFT-008': {
            id: 'SOFT-008',
            title: 'Soft dep 8',
            status: 'backlog',
            dependsOn: ['DB-004'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          // Referenced work units
          'DB-001': {
            id: 'DB-001',
            title: 'Database 1',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-002': {
            id: 'DB-002',
            title: 'Database 2',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-003': {
            id: 'DB-003',
            title: 'Database 3',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-004': {
            id: 'DB-004',
            title: 'Database 4',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [
            'SOFT-001',
            'SOFT-002',
            'SOFT-003',
            'SOFT-004',
            'SOFT-005',
            'SOFT-006',
            'SOFT-007',
            'SOFT-008',
          ],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'AUTH-002', 'API-001'],
          validating: [],
          done: ['DB-001', 'DB-002', 'DB-003', 'DB-004'],
          blocked: ['UI-001', 'UI-002', 'UI-003', 'UI-004', 'UI-005'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec query dependency-stats --output=json"
      const result = await queryDependencyStats({
        cwd: testDir,
      });

      // Then the output should show "work units with blockers: 5"
      expect(result.workUnitsWithBlockers).toBe(5);

      // And the output should show "work units blocking others: 3"
      expect(result.workUnitsBlockingOthers).toBe(3);

      // And the output should show "work units with soft dependencies: 8"
      expect(result.workUnitsWithSoftDependencies).toBe(8);

      // And the output should show average dependencies per unit
      expect(result.averageDependenciesPerUnit).toBeGreaterThan(0);

      // And the output should show max dependency chain depth
      expect(result.maxDependencyChainDepth).toBeGreaterThanOrEqual(0);
    });
  });
});
