/**
 * Test suite for: spec/features/work-unit-query-and-reporting.feature
 * Scenarios: Advanced query operations
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { queryWorkUnits } from '../query-work-units';
import { generateSummaryReport } from '../generate-summary-report';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Query and Reporting', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Query by status and prefix', () => {
    it('should filter work units by both status and prefix', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work units exist with various statuses and prefixes
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth work unit 1',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth work unit 2',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Auth work unit 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-003'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'AUTH-002', 'API-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run query with both status and prefix filters
      const result = await queryWorkUnits({
        status: 'implementing',
        prefix: 'AUTH',
        cwd: testDir,
      });

      // Then the output should contain only work units matching both criteria
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits?.length).toBe(2);

      // And the output should include "AUTH-001" and "AUTH-002"
      const ids = result.workUnits?.map((wu) => wu.id);
      expect(ids).toContain('AUTH-001');
      expect(ids).toContain('AUTH-002');

      // And the output should not include "API-001"
      expect(ids).not.toContain('API-001');

      // And the output should not include "AUTH-003" (wrong status)
      expect(ids).not.toContain('AUTH-003');
    });
  });

  describe('Scenario: Query with sorting by updated date', () => {
    it('should sort work units by updatedAt in descending order', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work units with different update times
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'First',
            status: 'implementing',
            createdAt: '2025-10-09T10:00:00Z',
            updatedAt: '2025-10-10T10:00:00Z',
            children: [],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Second (most recent)',
            status: 'implementing',
            createdAt: '2025-10-09T10:00:00Z',
            updatedAt: '2025-10-11T10:00:00Z',
            children: [],
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Third (oldest)',
            status: 'implementing',
            createdAt: '2025-10-09T10:00:00Z',
            updatedAt: '2025-10-09T10:00:00Z',
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run query with sort by updatedAt descending
      const result = await queryWorkUnits({
        sort: 'updatedAt',
        order: 'desc',
        cwd: testDir,
      });

      // Then the output should list work units in descending order by updatedAt
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits?.length).toBe(3);

      // And the first result should be "AUTH-002" (most recent)
      expect(result.workUnits?.[0].id).toBe('AUTH-002');

      // And the second result should be "AUTH-001"
      expect(result.workUnits?.[1].id).toBe('AUTH-001');

      // And the third result should be "AUTH-003" (oldest)
      expect(result.workUnits?.[2].id).toBe('AUTH-003');
    });
  });

  describe('Scenario: Export filtered results to CSV', () => {
    it('should export filtered work units to CSV format', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work units exist with status "implementing"
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Implementing task 1',
            status: 'implementing',
            createdAt: '2025-10-10T10:00:00Z',
            updatedAt: '2025-10-10T12:00:00Z',
            children: [],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Implementing task 2',
            status: 'implementing',
            createdAt: '2025-10-10T11:00:00Z',
            updatedAt: '2025-10-10T13:00:00Z',
            children: [],
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Done task',
            status: 'done',
            createdAt: '2025-10-09T10:00:00Z',
            updatedAt: '2025-10-09T15:00:00Z',
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001', 'AUTH-002'],
          validating: [],
          done: ['AUTH-003'],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run query with CSV format
      const result = await queryWorkUnits({
        status: 'implementing',
        format: 'csv',
        output: join(testDir, 'implementing.csv'),
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the file should be created
      const csvContent = await readFile(join(testDir, 'implementing.csv'), 'utf-8');

      // And the file should contain CSV headers
      expect(csvContent).toContain('id,title,status,createdAt,updatedAt');

      // And the file should contain only work units with status "implementing"
      expect(csvContent).toContain('AUTH-001');
      expect(csvContent).toContain('AUTH-002');
      expect(csvContent).not.toContain('AUTH-003');

      // And the CSV should be valid
      const lines = csvContent.trim().split('\n');
      expect(lines.length).toBe(3); // Header + 2 data rows
    });
  });

  describe('Scenario: Generate summary report with statistics', () => {
    it('should generate report showing total work units and breakdown by status', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And work units exist across all Kanban states
      // And 3 work units are in "backlog"
      // And 2 work units are in "implementing"
      // And 5 work units are in "done"
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Backlog 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Backlog 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Backlog 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-004': {
            id: 'AUTH-004',
            title: 'Implementing 1',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-005': {
            id: 'AUTH-005',
            title: 'Implementing 2',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-006': {
            id: 'AUTH-006',
            title: 'Done 1',
            status: 'done',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-007': {
            id: 'AUTH-007',
            title: 'Done 2',
            status: 'done',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-008': {
            id: 'AUTH-008',
            title: 'Done 3',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-009': {
            id: 'AUTH-009',
            title: 'Done 4',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'AUTH-010': {
            id: 'AUTH-010',
            title: 'Done 5',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-004', 'AUTH-005'],
          validating: [],
          done: ['AUTH-006', 'AUTH-007', 'AUTH-008', 'AUTH-009', 'AUTH-010'],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec report summary"
      const result = await generateSummaryReport({
        cwd: testDir,
        format: 'markdown',
      });

      // Then the output should show total work units: 10
      expect(result.totalWorkUnits).toBe(10);

      // And the output should show breakdown by status
      expect(result.byStatus).toBeDefined();

      // And the output should show "backlog: 3"
      expect(result.byStatus.backlog).toBe(3);

      // And the output should show "implementing: 2"
      expect(result.byStatus.implementing).toBe(2);

      // And the output should show "done: 5"
      expect(result.byStatus.done).toBe(5);

      // And the output should show total story points if estimates exist
      expect(result.totalStoryPoints).toBe(8); // 3 + 5 from AUTH-006 and AUTH-007
      expect(result.velocity.completedPoints).toBe(8);
      expect(result.velocity.completedWorkUnits).toBe(5);

      // Verify report file was created
      expect(result.outputFile).toBeDefined();
      const reportContent = await readFile(
        join(testDir, result.outputFile),
        'utf-8'
      );
      expect(reportContent).toContain('Total Work Units');
      expect(reportContent).toContain('backlog');
      expect(reportContent).toContain('implementing');
      expect(reportContent).toContain('done');
    });
  });
});
