import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, EpicsData } from '../../types';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';

import { queryWorkUnits } from '../query-work-units';
import { generateSummaryReport } from '../generate-summary-report';
import { exportWorkUnits } from '../export-work-units';
import { displayBoard } from '../display-board';

describe('Feature: Work Unit Query and Reporting', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-work-unit-query-reporting');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Create foundation.json for all tests (required by commands)
    await createMinimalFoundation(testDir);
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Query work units by status', () => {
    it('should return only implementing work units in JSON', async () => {
      // Given I have a project with spec directory
      // And work units exist with various statuses
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'testing',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'implementing',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'API endpoint',
            status: 'done',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-002'],
          implementing: ['AUTH-001', 'DASH-001'],
          validating: [],
          done: ['API-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec query work-units --status=implementing --output=json"
      const result = await queryWorkUnits({
        status: 'implementing',
        output: 'json',
        cwd: testDir,
      });

      // Then the output should contain only implementing work units
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits).toHaveLength(2);
      expect(result.workUnits?.every(wu => wu.status === 'implementing')).toBe(
        true
      );

      // And the output should be valid JSON
      expect(result.workUnits?.some(wu => wu.id === 'AUTH-001')).toBe(true);
      expect(result.workUnits?.some(wu => wu.id === 'DASH-001')).toBe(true);
    });
  });

  describe('Scenario: Query work units by epic', () => {
    it('should return only work units in specified epic', async () => {
      // Given I have a project with spec directory
      // And work units exist in multiple epics
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            epic: 'epic-auth',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User permissions',
            status: 'testing',
            epic: 'epic-auth',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'implementing',
            epic: 'epic-dashboard',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-002'],
          implementing: ['AUTH-001', 'DASH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const epics: EpicsData = {
        epics: {
          'epic-auth': {
            id: 'epic-auth',
            title: 'Authentication',
            workUnits: ['AUTH-001', 'AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'epic-dashboard': {
            id: 'epic-dashboard',
            title: 'Dashboard',
            workUnits: ['DASH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/epics.json'),
        JSON.stringify(epics, null, 2)
      );

      // When I run "fspec query work-units --epic=epic-auth"
      const result = await queryWorkUnits({
        epic: 'epic-auth',
        cwd: testDir,
      });

      // Then the output should contain only work units in epic-auth
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits).toHaveLength(2);
      expect(result.workUnits?.every(wu => wu.epic === 'epic-auth')).toBe(true);
      expect(result.workUnits?.some(wu => wu.id === 'AUTH-001')).toBe(true);
      expect(result.workUnits?.some(wu => wu.id === 'AUTH-002')).toBe(true);
    });
  });

  describe('Scenario: Query with compound filters', () => {
    it('should return work units matching ALL criteria', async () => {
      // Given I have a project with spec directory
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            epic: 'epic-auth',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User permissions',
            status: 'testing',
            epic: 'epic-auth',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'implementing',
            epic: 'epic-dashboard',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-002'],
          implementing: ['AUTH-001', 'DASH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec query work-units --status=implementing --epic=epic-auth --output=json"
      const result = await queryWorkUnits({
        status: 'implementing',
        epic: 'epic-auth',
        output: 'json',
        cwd: testDir,
      });

      // Then the output should contain work units matching ALL criteria
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits).toHaveLength(1);
      expect(result.workUnits?.[0].id).toBe('AUTH-001');
      expect(result.workUnits?.[0].status).toBe('implementing');
      expect(result.workUnits?.[0].epic).toBe('epic-auth');
    });
  });

  describe('Scenario: Generate project summary report', () => {
    it('should show comprehensive project statistics', async () => {
      // Given I have a project with spec directory
      // And work units exist across all states
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Permissions',
            status: 'testing',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'API',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['API-001'],
          specifying: [],
          testing: ['AUTH-002'],
          implementing: ['AUTH-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec report summary"
      const result = await generateSummaryReport({
        cwd: testDir,
      });

      // Then the output should show total work units
      expect(result.totalWorkUnits).toBe(4);

      // And the output should show breakdown by status
      expect(result.byStatus).toBeDefined();
      expect(result.byStatus.implementing).toBe(1);
      expect(result.byStatus.testing).toBe(1);
      expect(result.byStatus.done).toBe(1);
      expect(result.byStatus.backlog).toBe(1);

      // And the output should show total story points
      expect(result.totalStoryPoints).toBe(21);

      // And the output should show velocity metrics
      expect(result.velocity.completedPoints).toBe(8);
    });
  });

  describe('Scenario: Summary report writes to file and returns output path', () => {
    it('should write report to file and return path in output', async () => {
      // Given I have a project with spec directory
      // And work units exist across multiple states
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec generate-summary-report --format=markdown"
      const result = await generateSummaryReport({
        format: 'markdown',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the file "spec/summary-report.md" should be created
      const expectedPath = join(testDir, 'spec', 'summary-report.md');
      const fileContent = await readFile(expectedPath, 'utf-8');
      expect(fileContent).toBeTruthy();
      expect(fileContent.length).toBeGreaterThan(0);

      // And the output should display "Report generated: spec/summary-report.md"
      expect(result.outputFile).toBe('spec/summary-report.md');
    });
  });

  describe('Scenario: Export work units to JSON', () => {
    it('should create valid JSON file with all work unit fields', async () => {
      // Given I have a project with spec directory
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            estimate: 5,
            epic: 'epic-auth',
            description: 'Implement OAuth',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec export work-units --format=json --output=work-units-export.json"
      const result = await exportWorkUnits({
        format: 'json',
        output: join(testDir, 'work-units-export.json'),
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then the file should contain valid JSON array
      const exported = JSON.parse(
        await readFile(join(testDir, 'work-units-export.json'), 'utf-8')
      );
      expect(Array.isArray(exported)).toBe(true);

      // And each work unit should have all fields
      expect(exported).toHaveLength(2);
      expect(exported[0]).toHaveProperty('id');
      expect(exported[0]).toHaveProperty('title');
      expect(exported[0]).toHaveProperty('status');
      expect(exported[0]).toHaveProperty('estimate');
    });
  });

  describe('Scenario: Display Kanban board view', () => {
    it('should show columns for each state with work units', async () => {
      // Given I have a project with spec directory
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User permissions',
            status: 'testing',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard view',
            status: 'done',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-002'],
          implementing: ['AUTH-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec board"
      const result = await displayBoard({
        cwd: testDir,
      });

      // Then the output should show columns for each state
      expect(result.columns).toBeDefined();
      expect(result.columns!.implementing).toBeDefined();
      expect(result.columns!.testing).toBeDefined();
      expect(result.columns!.done).toBeDefined();

      // And each column should list work units
      expect(result.columns!.implementing).toHaveLength(1);
      expect(result.columns!.testing).toHaveLength(1);
      expect(result.columns!.done).toHaveLength(1);

      // And work units should show ID, title, and estimate
      const implementingItem = result.columns!.implementing[0];
      expect(implementingItem).toHaveProperty('id');
      expect(implementingItem).toHaveProperty('title');
      expect(implementingItem).toHaveProperty('estimate');
      expect(implementingItem.id).toBe('AUTH-001');
      expect(implementingItem.estimate).toBe(5);
    });
  });
});
