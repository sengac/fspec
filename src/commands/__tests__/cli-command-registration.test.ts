import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import type { WorkUnitsData, PrefixesData } from '../../types';

// Import command registration checker
import { execSync } from 'child_process';

let testDir: string;

beforeEach(async () => {
  testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
  await mkdir(testDir, { recursive: true });
});

afterEach(async () => {
  await rm(testDir, { recursive: true, force: true });
});

describe('Feature: Complete CLI Command Registration', () => {
  describe('Scenario: Verify all 41 missing commands are registered', () => {
    it('should have all commands accessible via CLI', () => {
      // Given I have a list of 41 missing commands (excluding dependencies, estimation, query, workflow-automation)
      const missingCommands = [
        'add-assumption',
        'add-dependencies',
        'add-dependency',
        'add-example',
        'add-question',
        'add-rule',
        'answer-question',
        'auto-advance',
        'clear-dependencies',
        'create-prefix',
        'delete-epic',
        'delete-work-unit',
        'display-board',
        'export-dependencies',
        'export-example-map',
        'export-work-units',
        'generate-scenarios',
        'generate-summary-report',
        'import-example-map',
        'prioritize-work-unit',
        'query-dependency-stats',
        'query-estimate-accuracy',
        'query-estimation-guide',
        'query-example-mapping-stats',
        'query-metrics',
        'query-work-units',
        'record-iteration',
        'record-metric',
        'record-tokens',
        'remove-dependency',
        'remove-example',
        'remove-question',
        'remove-rule',
        'repair-work-units',
        'update-prefix',
        'update-work-unit',
        'update-work-unit-estimate',
        'update-work-unit-status',
        'validate-spec-alignment',
        'validate-work-units',
      ];

      // When I check src/index.ts for command registrations
      // Then all 46 commands should be registered
      const registeredCommands: string[] = [];
      const unregisteredCommands: string[] = [];

      for (const command of missingCommands) {
        try {
          // Try to get help for the command
          const result = execSync(`node dist/index.js ${command} --help 2>&1`, {
            encoding: 'utf-8',
            stdio: 'pipe',
          });

          // If we get here without error, command is registered
          registeredCommands.push(command);
        } catch (error: any) {
          // Command not found
          if (error.stdout && error.stdout.includes('unknown command')) {
            unregisteredCommands.push(command);
          } else {
            // Command exists but might have other errors (which is fine for this test)
            registeredCommands.push(command);
          }
        }
      }

      // Report results
      if (unregisteredCommands.length > 0) {
        console.log(`\nUnregistered commands (${unregisteredCommands.length}):`);
        unregisteredCommands.forEach(cmd => console.log(`  - ${cmd}`));
      }

      if (registeredCommands.length > 0) {
        console.log(`\nRegistered commands (${registeredCommands.length}):`);
        registeredCommands.forEach(cmd => console.log(`  - ${cmd}`));
      }

      // Expect all commands to be registered
      expect(unregisteredCommands).toEqual([]);
      expect(registeredCommands.length).toBe(missingCommands.length);
    });
  });

  describe('Scenario: Register prioritize-work-unit command', () => {
    it('should move work unit to top of backlog', async () => {
      // Given the function "prioritizeWorkUnit" exists
      const { prioritizeWorkUnit } = await import('../prioritize-work-unit');
      expect(prioritizeWorkUnit).toBeDefined();

      // Setup test data
      await mkdir(join(testDir, 'spec'), { recursive: true });

      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run prioritize-work-unit AUTH-001 --position=top
      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 'top',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 should be moved to top of backlog
      expect(result.success).toBe(true);

      // Verify the order
      const { readFile } = await import('fs/promises');
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
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
      await mkdir(join(testDir, 'spec'), { recursive: true });

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run update-work-unit AUTH-001 --title='New Title'
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        title: 'New Title',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 title should be updated
      expect(result.success).toBe(true);

      // Verify the title
      const { readFile } = await import('fs/promises');
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
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
      await mkdir(join(testDir, 'spec'), { recursive: true });

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run delete-work-unit AUTH-001
      const result = await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 should be deleted
      expect(result.success).toBe(true);

      // Verify deletion
      const { readFile } = await import('fs/promises');
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updated: WorkUnitsData = JSON.parse(updatedContent);

      expect(updated.workUnits['AUTH-001']).toBeUndefined();
      expect(updated.states.backlog).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Register update-work-unit-status command', () => {
    it('should update work unit status following ACDD workflow', async () => {
      // Given the function "updateWorkUnitStatus" exists
      const { updateWorkUnitStatus } = await import('../update-work-unit-status');
      expect(updateWorkUnitStatus).toBeDefined();

      // Setup test data
      await mkdir(join(testDir, 'spec'), { recursive: true });

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run update-work-unit-status AUTH-001 specifying (following ACDD: backlog -> specifying)
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd: testDir,
      });

      // Then the work unit AUTH-001 status should be "specifying"
      expect(result.success).toBe(true);

      // Verify status
      const { readFile } = await import('fs/promises');
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updated: WorkUnitsData = JSON.parse(updatedContent);

      expect(updated.workUnits['AUTH-001'].status).toBe('specifying');
      expect(updated.states.specifying).toContain('AUTH-001');
      expect(updated.states.backlog).not.toContain('AUTH-001');
    });
  });
});
