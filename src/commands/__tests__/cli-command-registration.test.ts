/**
 * Feature: Complete CLI Command Registration
 *
 * Tests that all commands are properly implemented and accessible.
 * Instead of spawning CLI processes, we verify that command functions exist
 * and can be imported.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { Command } from 'commander';
import type { WorkUnitsData, PrefixesData } from '../../types';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

let testDir: string;

beforeEach(async () => {
  testDir = await createTempTestDir('cli-command-reg');
});

afterEach(async () => {
  await removeTempTestDir(testDir);
});

describe.sequential('Feature: Complete CLI Command Registration', () => {
  describe('Scenario: Verify command registration functions exist', () => {
    it('should have all command registration functions importable', async () => {
      // Instead of spawning processes, verify the registration functions can be imported
      // This validates that the command modules exist and export properly

      const commandModules = [
        '../add-assumption',
        '../add-dependency',
        '../add-example',
        '../add-question',
        '../add-rule',
        '../answer-question',
        '../display-board',
        '../create-prefix',
        '../delete-epic',
        '../delete-work-unit',
        '../export-work-units',
        '../generate-scenarios',
        '../import-example-map',
        '../prioritize-work-unit',
        '../query-metrics',
        '../query-work-units',
        '../repair-work-units',
        '../update-prefix',
        '../update-work-unit',
        '../update-work-unit-estimate',
        '../update-work-unit-status',
        '../validate',
      ];

      for (const modulePath of commandModules) {
        // Each module should be importable without errors
        const module = await import(modulePath);

        // Each module should have at least one exported function
        const exports = Object.keys(module);
        expect(
          exports.length,
          `Module ${modulePath} should have exports`
        ).toBeGreaterThan(0);

        // Verify there's a register function
        const hasRegister = exports.some(
          exp => exp.startsWith('register') && exp.endsWith('Command')
        );
        const hasMainFunction = exports.some(
          exp =>
            typeof module[exp] === 'function' && !exp.startsWith('register')
        );

        expect(
          hasRegister || hasMainFunction,
          `Module ${modulePath} should have a register function or main function`
        ).toBe(true);
      }
    });
  });

  describe('Scenario: Register prioritize-work-unit command', () => {
    it('should move work unit to top of backlog', async () => {
      // Given the function "prioritizeWorkUnit" exists
      const { prioritizeWorkUnit } = await import('../prioritize-work-unit');
      expect(prioritizeWorkUnit).toBeDefined();

      // Setup test data
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'First work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Second work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Third work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-002', 'AUTH-003', 'AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run prioritize-work-unit AUTH-001 --position=top
      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 'top',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 should be moved to top of backlog
      expect(result.success).toBe(true);

      // Verify the order
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updated: WorkUnitsData = JSON.parse(updatedContent);

      expect(updated.states.backlog[0]).toBe('AUTH-001');
    });
  });

  describe('Scenario: Register update-work-unit command', () => {
    it('should update work unit title', async () => {
      // Given the function "updateWorkUnit" exists
      const { updateWorkUnit } = await import('../update-work-unit');
      expect(updateWorkUnit).toBeDefined();

      // Setup test data
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Old Title',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run update-work-unit AUTH-001 --title='New Title'
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        title: 'New Title',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 title should be updated
      expect(result.success).toBe(true);

      // Verify the title
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updated: WorkUnitsData = JSON.parse(updatedContent);

      expect(updated.workUnits['AUTH-001'].title).toBe('New Title');
    });
  });

  describe('Scenario: Register delete-work-unit command', () => {
    it('should delete work unit', async () => {
      // Given the function "deleteWorkUnit" exists
      const { deleteWorkUnit } = await import('../delete-work-unit');
      expect(deleteWorkUnit).toBeDefined();

      // Setup test data
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'To be deleted',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run delete-work-unit AUTH-001
      const result = await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 should be deleted
      expect(result.success).toBe(true);

      // Verify deletion
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updated: WorkUnitsData = JSON.parse(updatedContent);

      expect(updated.workUnits['AUTH-001']).toBeUndefined();
      expect(updated.states.backlog).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Register update-work-unit-status command', () => {
    it('should update work unit status following ACDD workflow', async () => {
      // Given the function "updateWorkUnitStatus" exists
      const { updateWorkUnitStatus } = await import(
        '../update-work-unit-status'
      );
      expect(updateWorkUnitStatus).toBeDefined();

      // Setup test data
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run update-work-unit-status AUTH-001 specifying (following ACDD: backlog -> specifying)
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 status should be "specifying"
      expect(result.success).toBe(true);

      // Verify status
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updated: WorkUnitsData = JSON.parse(updatedContent);

      expect(updated.workUnits['AUTH-001'].status).toBe('specifying');
      expect(updated.states.specifying).toContain('AUTH-001');
      expect(updated.states.backlog).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: All command modules are importable', () => {
    it('should have all major command modules importable without error', async () => {
      // List of all major command modules to verify
      const allCommandModules = [
        '../add-assumption',
        '../add-background',
        '../add-dependency',
        '../add-diagram',
        '../add-example',
        '../add-question',
        '../add-rule',
        '../add-scenario',
        '../add-step',
        '../answer-question',
        '../display-board',
        '../create-epic',
        '../create-feature',
        '../create-prefix',
        '../create-story',
        '../delete-diagram',
        '../delete-epic',
        '../delete-scenario',
        '../delete-step',
        '../delete-work-unit',
        '../export-work-units',
        '../generate-scenarios',
        '../init',
        '../list-epics',
        '../list-features',
        '../list-prefixes',
        '../list-work-units',
        '../prioritize-work-unit',
        '../query-metrics',
        '../query-work-units',
        '../repair-work-units',
        '../show-work-unit',
        '../update-prefix',
        '../update-scenario',
        '../update-step',
        '../update-work-unit',
        '../update-work-unit-estimate',
        '../update-work-unit-status',
        '../validate',
      ];

      const importErrors: string[] = [];

      for (const modulePath of allCommandModules) {
        try {
          await import(modulePath);
        } catch (error) {
          importErrors.push(
            `Failed to import ${modulePath}: ${(error as Error).message}`
          );
        }
      }

      // All modules should be importable
      if (importErrors.length > 0) {
        console.error('Import errors:', importErrors);
      }
      expect(importErrors).toEqual([]);
    });
  });
});
