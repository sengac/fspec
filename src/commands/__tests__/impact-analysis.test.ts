/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Show impact analysis when completing work unit
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { analyzeImpact } from '../dependencies';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show impact analysis when completing work unit', () => {
    it('should show directly and transitively affected work units', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work unit "AUTH-001" has dependencies forming a chain
      // AUTH-001 blocks UI-001
      // UI-001 blocks FEAT-001
      // FEAT-001 depends on SERVICE-001
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
              blocks: ['UI-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'UI work unit',
            status: 'blocked',
            relationships: {
              blockedBy: ['AUTH-001'],
              blocks: ['FEAT-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'FEAT-001': {
            id: 'FEAT-001',
            title: 'Feature work unit',
            status: 'blocked',
            relationships: {
              blockedBy: ['UI-001'],
              dependsOn: ['SERVICE-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SERVICE-001': {
            id: 'SERVICE-001',
            title: 'Service work unit',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'SERVICE-001'],
          validating: [],
          done: [],
          blocked: ['UI-001', 'FEAT-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec analyze-impact AUTH-001"
      const result = await analyzeImpact('AUTH-001', {
        cwd: testDir,
      });

      // Then the output should show "Directly affected: UI-001"
      expect(result.directlyAffected).toBeDefined();
      expect(result.directlyAffected).toContain('UI-001');

      // And the output should show "Transitively affected: FEAT-001, SERVICE-001"
      expect(result.transitivelyAffected).toBeDefined();
      expect(result.transitivelyAffected).toContain('FEAT-001');
      expect(result.transitivelyAffected).toContain('SERVICE-001');

      // And the output should show "Total affected: 3 work units"
      expect(result.totalAffected).toBe(3);

      // Verify counts match
      expect(result.directlyAffected.length).toBe(1);
      expect(result.transitivelyAffected.length).toBe(2);
    });
  });
});
