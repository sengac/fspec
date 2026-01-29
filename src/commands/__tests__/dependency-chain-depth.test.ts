/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Show dependency chain depth
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { queryDependencyChain } from '../dependencies';
import type { WorkUnitsData } from '../../types';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Work Unit Dependency Management', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('dependency-chain-depth');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Show dependency chain depth', () => {
    it('should calculate the depth of dependency chain from a work unit', async () => {
      // Given I have a project with spec directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // And a chain of dependencies exists: AUTH-001 → UI-001 → FEAT-001 → SERVICE-001
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
              blocks: ['SERVICE-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'SERVICE-001': {
            id: 'SERVICE-001',
            title: 'Service work unit',
            status: 'blocked',
            relationships: {
              blockedBy: ['FEAT-001'],
            },
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
          done: [],
          blocked: ['UI-001', 'FEAT-001', 'SERVICE-001'],
        },
      };

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // When I run "fspec query-dependency-chain AUTH-001"
      const result = await queryDependencyChain('AUTH-001', {
        cwd: setup.testDir,
      });

      // Then the output should show the chain: "AUTH-001 → UI-001 → FEAT-001 → SERVICE-001"
      expect(result).toContain('AUTH-001');
      expect(result).toContain('UI-001');
      expect(result).toContain('FEAT-001');
      expect(result).toContain('SERVICE-001');
      expect(result).toContain('→');

      // And the output should show "Chain depth: 4"
      expect(result).toContain('Chain depth: 4');
    });

    it('should show depth of 1 for work unit with no dependencies', async () => {
      // Given I have a project with spec directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // And a work unit with no dependencies
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'SOLO-001': {
            id: 'SOLO-001',
            title: 'Solo work unit',
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
          implementing: ['SOLO-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // When I run "fspec query-dependency-chain SOLO-001"
      const result = await queryDependencyChain('SOLO-001', {
        cwd: setup.testDir,
      });

      // Then the output should show "Chain depth: 1"
      expect(result).toContain('Chain depth: 1');
      expect(result).toContain('SOLO-001');
    });
  });
});
