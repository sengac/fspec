/**
 * Feature: spec/features/list-prefixes.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { listPrefixes } from '../list-prefixes';

describe('Feature: List Prefixes Command', () => {
  let testDir: string;
  let specDir: string;
  let prefixesFile: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    prefixesFile = join(specDir, 'prefixes.json');
    workUnitsFile = join(specDir, 'work-units.json');

    // Create spec directory
    await mkdir(specDir, { recursive: true });
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List all prefixes with descriptions', () => {
    it('should display all prefixes with descriptions', async () => {
      // Given I have a project with spec/prefixes.json containing multiple prefixes
      const prefixesData = {
        prefixes: {
          SAFE: {
            prefix: 'SAFE',
            description: 'Safety and validation features',
            createdAt: '2025-10-12T03:27:56.701Z',
          },
          CLI: {
            prefix: 'CLI',
            description: 'CLI and command registration',
            createdAt: '2025-10-10T23:34:12.904Z',
          },
          TEST: {
            prefix: 'TEST',
            description: 'Testing and quality assurance',
            createdAt: '2025-10-11T02:25:05.112Z',
          },
        },
      };
      await writeFile(prefixesFile, JSON.stringify(prefixesData, null, 2));

      // When I run "fspec list-prefixes"
      const result = await listPrefixes({ cwd: testDir });

      // Then the command should display all prefixes
      expect(result.prefixes).toHaveLength(3);

      // And each prefix should show its description
      const safePrefix = result.prefixes.find(p => p.prefix === 'SAFE');
      expect(safePrefix).toBeDefined();
      expect(safePrefix?.description).toBe('Safety and validation features');

      const cliPrefix = result.prefixes.find(p => p.prefix === 'CLI');
      expect(cliPrefix).toBeDefined();
      expect(cliPrefix?.description).toBe('CLI and command registration');
    });
  });

  describe('Scenario: List prefixes with work unit statistics', () => {
    it('should show work unit count and completion percentage for each prefix', async () => {
      // Given I have prefixes in spec/prefixes.json
      const prefixesData = {
        prefixes: {
          SAFE: {
            prefix: 'SAFE',
            description: 'Safety features',
            createdAt: '2025-10-12T03:27:56.701Z',
          },
          CLI: {
            prefix: 'CLI',
            description: 'CLI features',
            createdAt: '2025-10-10T23:34:12.904Z',
          },
        },
      };
      await writeFile(prefixesFile, JSON.stringify(prefixesData, null, 2));

      // And I have work units with IDs like "SAFE-001", "CLI-002"
      // And some work units are in "done" status
      const workUnitsData = {
        workUnits: {
          'SAFE-001': {
            id: 'SAFE-001',
            title: 'Safety validation',
            status: 'done',
            createdAt: '2025-10-12T03:27:56.701Z',
            updatedAt: '2025-10-12T03:27:56.701Z',
          },
          'CLI-001': {
            id: 'CLI-001',
            title: 'Command registration',
            status: 'done',
            createdAt: '2025-10-12T03:27:56.701Z',
            updatedAt: '2025-10-12T03:27:56.701Z',
          },
          'CLI-002': {
            id: 'CLI-002',
            title: 'List prefixes',
            status: 'testing',
            createdAt: '2025-10-12T03:27:56.701Z',
            updatedAt: '2025-10-12T03:27:56.701Z',
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['CLI-002'],
          implementing: [],
          validating: [],
          done: ['SAFE-001', 'CLI-001'],
          blocked: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When I run "fspec list-prefixes"
      const result = await listPrefixes({ cwd: testDir });

      // Then each prefix should show work unit count
      const safePrefix = result.prefixes.find(p => p.prefix === 'SAFE');
      expect(safePrefix).toBeDefined();
      expect(safePrefix?.totalWorkUnits).toBe(1);
      expect(safePrefix?.completedWorkUnits).toBe(1);

      const cliPrefix = result.prefixes.find(p => p.prefix === 'CLI');
      expect(cliPrefix).toBeDefined();
      expect(cliPrefix?.totalWorkUnits).toBe(2);
      expect(cliPrefix?.completedWorkUnits).toBe(1);

      // And each prefix should show completion percentage
      expect(safePrefix?.completionPercentage).toBe(100);
      expect(cliPrefix?.completionPercentage).toBe(50);
    });
  });

  describe('Scenario: Handle missing prefixes file gracefully', () => {
    it('should return empty list when prefixes.json does not exist', async () => {
      // Given I have a project with no spec/prefixes.json file
      // (prefixesFile not created)

      // When I run "fspec list-prefixes"
      const result = await listPrefixes({ cwd: testDir });

      // Then the command should return empty list
      expect(result.prefixes).toEqual([]);
    });
  });
});
