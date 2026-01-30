/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Show all work units blocked by specific work unit
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { listDependencies } from '../dependencies';
import type { WorkUnitsData } from '../../types';
import { setupWorkUnitTest, type WorkUnitTestSetup } from '../../test-helpers/universal-test-setup';

describe('Feature: Work Unit Dependency Management', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('query-work-units-blocked-by');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Show all work units blocked by specific work unit', () => {
    it('should show all work units that are blocked by a specific blocker', async () => {
      // Given I have a project with spec directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // And work unit "AUTH-001" blocks 3 other work units
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth blocker',
            status: 'implementing',
            relationships: {
              blocks: ['UI-001', 'API-001', 'FEAT-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'Blocked UI work',
            status: 'blocked',
            relationships: {
              blockedBy: ['AUTH-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'Blocked API work',
            status: 'blocked',
            relationships: {
              blockedBy: ['AUTH-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'FEAT-001': {
            id: 'FEAT-001',
            title: 'Blocked feature work',
            status: 'blocked',
            relationships: {
              blockedBy: ['AUTH-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-001': {
            id: 'DB-001',
            title: 'Unrelated work',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: ['DB-001'],
          blocked: ['UI-001', 'API-001', 'FEAT-001'],
        },
      };

      await writeFile(
        join(setup.testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec list-dependencies AUTH-001 --type=blocks"
      const result = await listDependencies('AUTH-001', {
        cwd: setup.testDir,
        type: 'blocks',
      });

      // Then the output should show 3 blocked work units
      expect(result.blocks).toBeDefined();
      expect(result.blocks.length).toBe(3);

      // And the output should include "UI-001"
      expect(result.blocks).toContain('UI-001');

      // And the output should include "API-001"
      expect(result.blocks).toContain('API-001');

      // And the output should include "FEAT-001"
      expect(result.blocks).toContain('FEAT-001');

      // And other relationship types should be empty since we filtered by type
      expect(result.blockedBy).toEqual([]);
      expect(result.dependsOn).toEqual([]);
      expect(result.relatesTo).toEqual([]);
    });
  });
});
