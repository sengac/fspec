/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenarios: Bidirectional dependency relationships
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { addDependency } from '../add-dependency';
import { removeDependency } from '../remove-dependency';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add blocks relationship creates bidirectional link', () => {
    it('should create both blocks and blockedBy relationships', async () => {
      // Given I have two work units
      await mkdir(join(testDir, 'spec'), { recursive: true });

      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Blocker work unit',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Blocked work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-002'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I add "AUTH-001 blocks AUTH-002"
      const result = await addDependency({
        workUnitId: 'AUTH-001',
        blocks: 'AUTH-002',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then AUTH-001 should have blocks: ['AUTH-002']
      const fileContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(fileContent);

      expect(data.workUnits['AUTH-001'].blocks).toContain('AUTH-002');

      // And AUTH-002 should have blockedBy: ['AUTH-001']
      expect(data.workUnits['AUTH-002'].blockedBy).toContain('AUTH-001');

      // And AUTH-002 should be moved to blocked state
      expect(data.workUnits['AUTH-002'].status).toBe('blocked');
      expect(data.states.blocked).toContain('AUTH-002');
      expect(data.states.backlog).not.toContain('AUTH-002');
    });
  });

  describe('Scenario: Remove dependency cleans up both sides of relationship', () => {
    it('should remove both blocks and blockedBy when removing dependency', async () => {
      // Given I have two work units with a blocks relationship
      await mkdir(join(testDir, 'spec'), { recursive: true });

      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Blocker work unit',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            blocks: ['AUTH-002'],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Blocked work unit',
            status: 'blocked',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            blockedBy: ['AUTH-001'],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: ['AUTH-002'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I remove the blocks relationship
      const result = await removeDependency({
        workUnitId: 'AUTH-001',
        blocks: 'AUTH-002',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then AUTH-001 should no longer have AUTH-002 in blocks
      const fileContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(fileContent);

      expect(data.workUnits['AUTH-001'].blocks || []).not.toContain('AUTH-002');

      // And AUTH-002 should no longer have AUTH-001 in blockedBy
      expect(data.workUnits['AUTH-002'].blockedBy || []).not.toContain(
        'AUTH-001'
      );
    });
  });
});
