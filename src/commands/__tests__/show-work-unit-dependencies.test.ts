/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Show work unit with all dependencies
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { showWorkUnit } from '../show-work-unit';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show work unit with all dependencies', () => {
    it('should display all dependency types for work unit', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And a work unit "AUTH-001" has dependencies
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
            blocks: ['API-001', 'UI-001'],
            dependsOn: ['DB-001'],
            relatesTo: ['SEC-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
            status: 'blocked',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'UI-001': {
            id: 'UI-001',
            title: 'UI work unit',
            status: 'blocked',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'DB-001': {
            id: 'DB-001',
            title: 'Database work unit',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SEC-001': {
            id: 'SEC-001',
            title: 'Security work unit',
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
          implementing: ['AUTH-001', 'SEC-001'],
          validating: [],
          done: ['DB-001'],
          blocked: ['API-001', 'UI-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec show-work-unit AUTH-001"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the output should display all dependency types
      expect(result).toBeDefined();

      // And the output should show "Blocks: API-001, UI-001"
      expect(result.blocks).toContain('API-001');
      expect(result.blocks).toContain('UI-001');
      expect(result.blocks?.length).toBe(2);

      // And the output should show "Depends On: DB-001"
      expect(result.dependsOn).toContain('DB-001');
      expect(result.dependsOn?.length).toBe(1);

      // And the output should show "Related To: SEC-001"
      expect(result.relatesTo).toContain('SEC-001');
      expect(result.relatesTo?.length).toBe(1);
    });
  });
});
