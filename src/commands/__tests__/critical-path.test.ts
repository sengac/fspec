/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Calculate critical path through dependencies
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { calculateCriticalPath } from '../dependencies';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Calculate critical path through dependencies', () => {
    it('should find the longest dependency path through the work units', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And multiple dependency paths exist
      // Path 1: FEAT-001 → UI-001 → AUTH-001 (depth 3)
      // Path 2: SERVICE-001 → DB-001 (depth 2)
      // Path 3: API-001 → AUTH-001 → DB-001 (depth 3)
      // Critical path should be one of the depth-3 paths
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth work unit',
            status: 'implementing',
            relationships: {
              dependsOn: ['DB-001'],
            },
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'UI work unit',
            status: 'backlog',
            relationships: {
              dependsOn: ['AUTH-001'],
            },
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'FEAT-001': {
            id: 'FEAT-001',
            title: 'Feature work unit',
            status: 'backlog',
            relationships: {
              dependsOn: ['UI-001'],
            },
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-001': {
            id: 'DB-001',
            title: 'Database work unit',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SERVICE-001': {
            id: 'SERVICE-001',
            title: 'Service work unit',
            status: 'backlog',
            relationships: {
              dependsOn: ['DB-001'],
            },
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
            status: 'backlog',
            relationships: {
              dependsOn: ['AUTH-001'],
            },
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['FEAT-001', 'UI-001', 'SERVICE-001', 'API-001'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'DB-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec calculate-critical-path"
      const result = await calculateCriticalPath({
        cwd: testDir,
      });

      // Then the output should show a critical path of length 4
      // The longest path is FEAT-001 → UI-001 → AUTH-001 → DB-001
      expect(result.length).toBe(4);

      // And the critical path should contain 4 work units
      expect(result.path.length).toBe(4);

      // Verify it's the expected path: FEAT-001 → UI-001 → AUTH-001 → DB-001
      expect(result.path).toContain('FEAT-001');
      expect(result.path).toContain('UI-001');
      expect(result.path).toContain('AUTH-001');
      expect(result.path).toContain('DB-001');

      // And the estimated effort should be calculated from story points
      expect(result.estimatedEffort).toBeDefined();
      expect(result.estimatedEffort).toBeGreaterThan(0);
    });

    it('should handle work units with no dependencies', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work units have no dependency relationships
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'SOLO-001': {
            id: 'SOLO-001',
            title: 'Solo work unit 1',
            status: 'implementing',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SOLO-002': {
            id: 'SOLO-002',
            title: 'Solo work unit 2',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['SOLO-002'],
          specifying: [],
          testing: [],
          implementing: ['SOLO-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec calculate-critical-path"
      const result = await calculateCriticalPath({
        cwd: testDir,
      });

      // Then the critical path should have length 1
      expect(result.length).toBe(1);

      // And the path should contain one work unit
      expect(result.path.length).toBe(1);
    });
  });
});
