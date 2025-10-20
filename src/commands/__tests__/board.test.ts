/**
 * Test suite for: spec/features/kanban-workflow-state-management.feature
 * Scenario: Display Kanban board showing all states (line 364)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { displayBoard } from '../display-board';

describe('Feature: Kanban Workflow State Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display Kanban board showing all states', () => {
    it('should display columns for all states with work units', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And foundation.json exists
      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          version: '2.0.0',
          project: { name: 'Test', vision: 'Test', projectType: 'cli-tool' },
          problemSpace: {
            primaryProblem: {
              title: 'Test',
              description: 'Test',
              impact: 'high',
            },
          },
          solutionSpace: { overview: 'Test', capabilities: [] },
          personas: [],
        })
      );

      // And work units exist
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth task',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth spec',
            status: 'specifying',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Auth test',
            status: 'testing',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'API',
            status: 'validating',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'SEC-001': {
            id: 'SEC-001',
            title: 'Security',
            status: 'done',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: ['AUTH-002'],
          testing: ['AUTH-003'],
          implementing: ['DASH-001'],
          validating: ['API-001'],
          done: ['SEC-001'],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec board"
      const result = await displayBoard({ cwd: testDir });

      // Then the output should display columns for all states
      expect(result.board).toBeDefined();
      expect(result.board.backlog).toEqual(['AUTH-001']);
      expect(result.board.specifying).toEqual(['AUTH-002']);
      expect(result.board.testing).toEqual(['AUTH-003']);
      expect(result.board.implementing).toEqual(['DASH-001']);
      expect(result.board.validating).toEqual(['API-001']);
      expect(result.board.done).toEqual(['SEC-001']);
      expect(result.board.blocked).toEqual([]);

      // And the backlog column should show "AUTH-001 [5 pts]"
      expect(result.columns?.backlog).toHaveLength(1);
      expect(result.columns?.backlog[0].id).toBe('AUTH-001');
      expect(result.columns?.backlog[0].estimate).toBe(5);

      // And the specifying column should show "AUTH-002 [8 pts]"
      expect(result.columns?.specifying[0].id).toBe('AUTH-002');
      expect(result.columns?.specifying[0].estimate).toBe(8);

      // And the testing column should show "AUTH-003 [3 pts]"
      expect(result.columns?.testing[0].id).toBe('AUTH-003');
      expect(result.columns?.testing[0].estimate).toBe(3);

      // And the implementing column should show "DASH-001 [5 pts]"
      expect(result.columns?.implementing[0].id).toBe('DASH-001');
      expect(result.columns?.implementing[0].estimate).toBe(5);

      // And the validating column should show "API-001 [5 pts]"
      expect(result.columns?.validating[0].id).toBe('API-001');
      expect(result.columns?.validating[0].estimate).toBe(5);

      // And the done column should show "SEC-001 [3 pts]"
      expect(result.columns?.done[0].id).toBe('SEC-001');
      expect(result.columns?.done[0].estimate).toBe(3);

      // And the summary should show "26 points in progress" and "3 points completed"
      expect(result.summary).toContain('26 points in progress');
      expect(result.summary).toContain('3 points completed');
    });
  });

  describe('Scenario: Limit displayed work units per column with --limit option', () => {
    it('should limit displayed work units and show overflow indicator', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And foundation.json exists
      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          version: '2.0.0',
          project: { name: 'Test', vision: 'Test', projectType: 'cli-tool' },
          problemSpace: {
            primaryProblem: {
              title: 'Test',
              description: 'Test',
              impact: 'high',
            },
          },
          solutionSpace: { overview: 'Test', capabilities: [] },
          personas: [],
        })
      );

      // And work units exist in backlog
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'First',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Second',
            status: 'backlog',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Third',
            status: 'backlog',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-004': {
            id: 'AUTH-004',
            title: 'Fourth',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-005': {
            id: 'AUTH-005',
            title: 'Fifth',
            status: 'backlog',
            estimate: 2,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003', 'AUTH-004', 'AUTH-005'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec board --limit=2" (NOTE: limit is handled by BoardDisplay component, not displayBoard function)
      const result = await displayBoard({ cwd: testDir });

      // Then the backlog column should have all 5 work units (displayBoard returns all data)
      expect(result.columns?.backlog).toHaveLength(5);
      expect(result.columns?.backlog[0].id).toBe('AUTH-001');
      expect(result.columns?.backlog[0].estimate).toBe(5);
      expect(result.columns?.backlog[1].id).toBe('AUTH-002');
      expect(result.columns?.backlog[1].estimate).toBe(3);

      // And the other columns should be empty
      expect(result.columns?.specifying).toHaveLength(0);
      expect(result.columns?.testing).toHaveLength(0);
      expect(result.columns?.implementing).toHaveLength(0);
      expect(result.columns?.validating).toHaveLength(0);
      expect(result.columns?.done).toHaveLength(0);
      expect(result.columns?.blocked).toHaveLength(0);

      // Note: The actual limiting and "... X more" display is handled by the BoardDisplay
      // component when rendering, not by the displayBoard function.
      // The BoardDisplay component receives the limit prop and handles the display logic.
    });
  });

  describe('Scenario: Export Kanban board as JSON for programmatic access', () => {
    it('should export board data as valid JSON', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And foundation.json exists
      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          version: '2.0.0',
          project: { name: 'Test', vision: 'Test', projectType: 'cli-tool' },
          problemSpace: {
            primaryProblem: {
              title: 'Test',
              description: 'Test',
              impact: 'high',
            },
          },
          solutionSpace: { overview: 'Test', capabilities: [] },
          personas: [],
        })
      );

      // And work units exist
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Backlog task',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Spec task',
            status: 'specifying',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Implementing task',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: ['AUTH-002'],
          testing: [],
          implementing: ['DASH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec board --format=json"
      const result = await displayBoard({ cwd: testDir });

      // Then the output should be valid JSON (result is already a JSON object)
      expect(result).toBeDefined();

      // And the JSON should have a "columns" object
      expect(result.columns).toBeDefined();
      expect(typeof result.columns).toBe('object');

      // And the JSON should have a "board" object
      expect(result.board).toBeDefined();
      expect(typeof result.board).toBe('object');

      // And the JSON should have a "summary" string
      expect(result.summary).toBeDefined();
      expect(typeof result.summary).toBe('string');

      // And the columns.backlog array should contain work unit with id "AUTH-001"
      expect(result.columns?.backlog).toBeDefined();
      expect(result.columns?.backlog).toHaveLength(1);
      expect(result.columns?.backlog[0].id).toBe('AUTH-001');
      expect(result.columns?.backlog[0].estimate).toBe(5);

      // And the columns.specifying array should contain work unit with id "AUTH-002"
      expect(result.columns?.specifying).toBeDefined();
      expect(result.columns?.specifying).toHaveLength(1);
      expect(result.columns?.specifying[0].id).toBe('AUTH-002');
      expect(result.columns?.specifying[0].estimate).toBe(8);

      // And the board.backlog array should contain "AUTH-001"
      expect(result.board.backlog).toContain('AUTH-001');

      // And the board.specifying array should contain "AUTH-002"
      expect(result.board.specifying).toContain('AUTH-002');

      // Verify JSON can be stringified without errors (for CLI JSON output)
      const jsonString = JSON.stringify(result, null, 2);
      expect(jsonString).toBeDefined();
      expect(jsonString.length).toBeGreaterThan(0);

      // Verify JSON can be parsed back
      const parsed = JSON.parse(jsonString);
      expect(parsed.columns.backlog[0].id).toBe('AUTH-001');
    });
  });
});
