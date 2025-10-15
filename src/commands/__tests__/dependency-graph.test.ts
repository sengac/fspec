/**
 * Test suite for: spec/features/work-unit-dependency-management.feature
 * Scenario: Display dependency graph for work unit
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { getDependencyGraph, showDependencies } from '../dependencies';
import type { WorkUnitsData } from '../../types';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display dependency graph for work unit', () => {
    it('should display dependency graph in JSON format', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And multiple work units with various dependencies
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
              dependsOn: ['DB-001'],
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
              relatesTo: ['FEAT-001'],
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'FEAT-001': {
            id: 'FEAT-001',
            title: 'Feature work unit',
            status: 'backlog',
            relationships: {
              relatesTo: ['UI-001'],
            },
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
        },
        states: {
          backlog: ['FEAT-001'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: ['DB-001'],
          blocked: ['UI-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec get-dependency-graph --format=json"
      const result = await getDependencyGraph({
        cwd: testDir,
        format: 'json',
      });

      // Then the output should be valid JSON
      const graph = JSON.parse(result);
      expect(graph).toBeDefined();

      // And the graph should contain all work units
      expect(graph['AUTH-001']).toBeDefined();
      expect(graph['UI-001']).toBeDefined();
      expect(graph['FEAT-001']).toBeDefined();
      expect(graph['DB-001']).toBeDefined();

      // And the graph should show AUTH-001 blocks UI-001
      expect(graph['AUTH-001'].blocks).toContain('UI-001');

      // And the graph should show AUTH-001 depends on DB-001
      expect(graph['AUTH-001'].dependsOn).toContain('DB-001');

      // And the graph should show UI-001 is blocked by AUTH-001
      expect(graph['UI-001'].blockedBy).toContain('AUTH-001');

      // And the graph should show UI-001 relates to FEAT-001
      expect(graph['UI-001'].relatesTo).toContain('FEAT-001');
    });

    it('should display dependency graph in Mermaid format', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And multiple work units with dependencies
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
          blocked: ['UI-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec get-dependency-graph --format=mermaid"
      const result = await getDependencyGraph({
        cwd: testDir,
        format: 'mermaid',
      });

      // Then the output should contain Mermaid syntax
      expect(result).toContain('graph TD');

      // And the output should show the dependency relationship
      expect(result).toContain('AUTH-001');
      expect(result).toContain('UI-001');
      expect(result).toContain('blocks');
    });

    it('should show dependencies for a specific work unit with graph visualization', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And a work unit with multiple dependency types
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
              blocks: ['UI-001', 'API-001'],
              dependsOn: ['DB-001'],
              relatesTo: ['SEC-001'],
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
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
          'API-001': {
            id: 'API-001',
            title: 'API work unit',
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
            relationships: {
              relatesTo: ['AUTH-001'],
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
          implementing: ['AUTH-001', 'SEC-001'],
          validating: [],
          done: ['DB-001'],
          blocked: ['UI-001', 'API-001'],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec show-dependencies AUTH-001 --graph"
      const result = await showDependencies(
        'AUTH-001',
        { graph: true },
        { cwd: testDir }
      );

      // Then the output should contain AUTH-001
      expect(result).toContain('AUTH-001');

      // And the output should show work units it blocks
      expect(result).toContain('UI-001');
      expect(result).toContain('API-001');
    });
  });
});
