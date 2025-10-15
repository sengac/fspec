/**
 * Feature: spec/features/work-unit-dependency-management.feature
 * Scenario: Identify bottleneck work units blocking the most work (@COV-046)
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

  describe('Scenario: Identify bottleneck work units blocking the most work', () => {
    it('should list work units ranked by bottleneck score', async () => {
      // And work units exist with blocking relationships:
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test: Auth work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blocks: ['API-001', 'API-002', 'API-003'],
          },
          'API-001': {
            id: 'API-001',
            title: 'Test: API work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blocks: ['UI-001'],
            blockedBy: ['AUTH-001'],
          },
          'DB-001': {
            id: 'DB-001',
            title: 'Test: DB work',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blocks: ['API-002'],
          },
          'CACHE-001': {
            id: 'CACHE-001',
            title: 'Test: Cache work',
            status: 'blocked',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blocks: ['API-003'],
          },
          'UI-002': {
            id: 'UI-002',
            title: 'Test: UI work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-002': {
            id: 'API-002',
            title: 'Test: API 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blockedBy: ['AUTH-001', 'DB-001'],
          },
          'API-003': {
            id: 'API-003',
            title: 'Test: API 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blockedBy: ['AUTH-001', 'CACHE-001'],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'Test: UI 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            blockedBy: ['API-001'],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'API-001', 'UI-002'],
          validating: [],
          done: ['DB-001'],
          blocked: ['CACHE-001'],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When I run "fspec query bottlenecks --output=json"
      const { queryBottlenecks } = await import('../query-bottlenecks.js');
      const result = await queryBottlenecks({ cwd: tempDir, output: 'json' });

      // Then the output should list work units ranked by bottleneck score
      expect(result).toBeDefined();
      expect(Array.isArray(result.bottlenecks)).toBe(true);

      // And only work units blocking 2+ other work units should be included
      expect(result.bottlenecks.length).toBeGreaterThan(0);
      result.bottlenecks.forEach((bottleneck: { score: number }) => {
        expect(bottleneck.score).toBeGreaterThanOrEqual(2);
      });

      // And "AUTH-001" should have highest bottleneck score of 4
      const auth001 = result.bottlenecks.find((b: { id: string }) => b.id === 'AUTH-001');
      expect(auth001).toBeDefined();
      expect(auth001.score).toBe(4); // Blocks API-001, API-002, API-003 directly, UI-001 transitively

      // And "DB-001" should not be included (status='done')
      const db001 = result.bottlenecks.find((b: { id: string }) => b.id === 'DB-001');
      expect(db001).toBeUndefined();

      // And "CACHE-001" should not be included (status='blocked')
      const cache001 = result.bottlenecks.find((b: { id: string }) => b.id === 'CACHE-001');
      expect(cache001).toBeUndefined();

      // And "UI-002" should not be included (blocks nothing)
      const ui002 = result.bottlenecks.find((b: { id: string }) => b.id === 'UI-002');
      expect(ui002).toBeUndefined();

      // And work units in 'done' status should be excluded
      result.bottlenecks.forEach((bottleneck: { status: string }) => {
        expect(bottleneck.status).not.toBe('done');
      });

      // And work units in 'blocked' status should be excluded
      result.bottlenecks.forEach((bottleneck: { status: string }) => {
        expect(bottleneck.status).not.toBe('blocked');
      });
    });
  });
});
