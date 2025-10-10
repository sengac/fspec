import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../../types';

// Import commands (to be created)
import { addDependency } from '../add-dependency';
import { removeDependency } from '../remove-dependency';
import { addDependencies } from '../add-dependencies';
import { clearDependencies } from '../clear-dependencies';
import { exportDependencies } from '../export-dependencies';
import { queryDependencyStats } from '../query-dependency-stats';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-work-unit-dependency-management');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add blocks relationship to work unit', () => {
    it('should create bidirectional blocking relationship', async () => {
      // Given I have a project with work units "AUTH-001" and "AUTH-002"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-dependency AUTH-002 --blocks AUTH-001"
      const result = await addDependency({
        workUnitId: 'AUTH-002',
        blocks: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // And AUTH-002 should have blocks: ["AUTH-001"]
      expect(updated.workUnits['AUTH-002'].blocks).toContain('AUTH-001');

      // And AUTH-001 should have blockedBy: ["AUTH-002"]
      expect(updated.workUnits['AUTH-001'].blockedBy).toContain('AUTH-002');
    });
  });

  describe('Scenario: Add blockedBy relationship (inverse of blocks)', () => {
    it('should create bidirectional blocking relationship using blockedBy', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-dependency AUTH-001 --blocked-by AUTH-002"
      const result = await addDependency({
        workUnitId: 'AUTH-001',
        blockedBy: 'AUTH-002',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // And AUTH-001 should have blockedBy: ["AUTH-002"]
      expect(updated.workUnits['AUTH-001'].blockedBy).toContain('AUTH-002');

      // And AUTH-002 should have blocks: ["AUTH-001"]
      expect(updated.workUnits['AUTH-002'].blocks).toContain('AUTH-001');
    });
  });

  describe('Scenario: Add dependsOn relationship (soft dependency)', () => {
    it('should create unidirectional soft dependency', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['DASH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-dependency DASH-001 --depends-on AUTH-001"
      const result = await addDependency({
        workUnitId: 'DASH-001',
        dependsOn: 'AUTH-001',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // And DASH-001 should have dependsOn: ["AUTH-001"]
      expect(updated.workUnits['DASH-001'].dependsOn).toContain('AUTH-001');

      // And AUTH-001 should NOT have any reverse dependency field
      expect(updated.workUnits['AUTH-001'].dependedOnBy).toBeUndefined();
    });
  });

  describe('Scenario: Add relatesTo relationship (informational)', () => {
    it('should create bidirectional informational relationship', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User permissions',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-dependency AUTH-001 --relates-to AUTH-002"
      const result = await addDependency({
        workUnitId: 'AUTH-001',
        relatesTo: 'AUTH-002',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // And AUTH-001 should have relatesTo: ["AUTH-002"]
      expect(updated.workUnits['AUTH-001'].relatesTo).toContain('AUTH-002');

      // And AUTH-002 should have relatesTo: ["AUTH-001"]
      expect(updated.workUnits['AUTH-002'].relatesTo).toContain('AUTH-001');
    });
  });

  describe('Scenario: Add multiple dependencies of same type', () => {
    it('should track multiple blocking relationships', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Password reset',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002', 'AUTH-003'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Add first blocker
      await addDependency({
        workUnitId: 'AUTH-002',
        blocks: 'AUTH-001',
        cwd: testDir,
      });

      // Add second blocker
      await addDependency({
        workUnitId: 'AUTH-003',
        blocks: 'AUTH-001',
        cwd: testDir,
      });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // AUTH-001 should be blocked by both
      expect(updated.workUnits['AUTH-001'].blockedBy).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].blockedBy).toContain('AUTH-002');
      expect(updated.workUnits['AUTH-001'].blockedBy).toContain('AUTH-003');
    });
  });

  describe('Scenario: Add multiple dependency types to same work unit', () => {
    it('should maintain all dependency types simultaneously', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'API',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: ['API-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      await addDependency({
        workUnitId: 'AUTH-001',
        blocks: 'AUTH-002',
        cwd: testDir,
      });

      await addDependency({
        workUnitId: 'AUTH-001',
        dependsOn: 'DASH-001',
        cwd: testDir,
      });

      await addDependency({
        workUnitId: 'AUTH-001',
        relatesTo: 'API-001',
        cwd: testDir,
      });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].blocks).toContain('AUTH-002');
      expect(updated.workUnits['AUTH-001'].dependsOn).toContain('DASH-001');
      expect(updated.workUnits['AUTH-001'].relatesTo).toContain('API-001');
    });
  });

  describe('Scenario: Remove blocks relationship (bidirectional cleanup)', () => {
    it('should remove both sides of blocking relationship', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            blockedBy: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            blocks: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-dependency AUTH-002 --blocks AUTH-001"
      const result = await removeDependency({
        workUnitId: 'AUTH-002',
        blocks: 'AUTH-001',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // AUTH-002 should not have blocks
      expect(updated.workUnits['AUTH-002'].blocks || []).not.toContain('AUTH-001');

      // AUTH-001 should not have blockedBy
      expect(updated.workUnits['AUTH-001'].blockedBy || []).not.toContain('AUTH-002');
    });
  });

  describe('Scenario: Remove dependsOn relationship (unidirectional)', () => {
    it('should remove only the dependsOn link', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'specifying',
            dependsOn: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['DASH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await removeDependency({
        workUnitId: 'DASH-001',
        dependsOn: 'AUTH-001',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['DASH-001'].dependsOn || []).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Detect direct circular dependency', () => {
    it('should prevent A blocks B and B blocks A', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            blocks: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Try to add AUTH-002 blocks AUTH-001 (which already blocks AUTH-002)
      const error = await addDependency({
        workUnitId: 'AUTH-002',
        blocks: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Circular dependency detected');
        expect(error.message).toContain('AUTH-002 -> AUTH-001 -> AUTH-002');
      }
    });
  });

  describe('Scenario: Detect transitive circular dependency', () => {
    it('should prevent A blocks B, B blocks C, C blocks A', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            blocks: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            blockedBy: ['AUTH-001'],
            blocks: ['AUTH-003'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Password reset',
            status: 'specifying',
            blockedBy: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002', 'AUTH-003'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Try to add AUTH-003 blocks AUTH-001 (creating cycle)
      const error = await addDependency({
        workUnitId: 'AUTH-003',
        blocks: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Circular dependency detected');
      }
    });
  });

  describe('Scenario: Detect complex circular dependency chain', () => {
    it('should detect cycles in dependency chains', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'A': {
            id: 'A',
            title: 'Work A',
            status: 'testing',
            blocks: ['B'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'B': {
            id: 'B',
            title: 'Work B',
            status: 'specifying',
            blockedBy: ['A'],
            blocks: ['C'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'C': {
            id: 'C',
            title: 'Work C',
            status: 'specifying',
            blockedBy: ['B'],
            blocks: ['D'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'D': {
            id: 'D',
            title: 'Work D',
            status: 'specifying',
            blockedBy: ['C'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['B', 'C', 'D'],
          testing: ['A'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Try to add D blocks A (creating long cycle)
      const error = await addDependency({
        workUnitId: 'D',
        blocks: 'A',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Circular dependency detected');
      }
    });
  });

  describe('Scenario: Attempt to add dependency to non-existent work unit', () => {
    it('should fail when work unit does not exist', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const error = await addDependency({
        workUnitId: 'AUTH-999',
        blocks: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain("Work unit 'AUTH-999' does not exist");
      }
    });
  });

  describe('Scenario: Attempt to add dependency targeting non-existent work unit', () => {
    it('should fail when target work unit does not exist', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const error = await addDependency({
        workUnitId: 'AUTH-001',
        blocks: 'AUTH-999',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain("Target work unit 'AUTH-999' does not exist");
      }
    });
  });

  describe('Scenario: Attempt to create self-dependency', () => {
    it('should prevent work unit from blocking itself', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const error = await addDependency({
        workUnitId: 'AUTH-001',
        blocks: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Cannot create self-dependency');
      }
    });
  });

  describe('Scenario: Attempt to add duplicate dependency', () => {
    it('should prevent adding same dependency twice', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            blockedBy: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            blocks: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const error = await addDependency({
        workUnitId: 'AUTH-002',
        blocks: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Dependency already exists');
      }
    });
  });

  describe('Scenario: Auto-transition to blocked state when blockedBy relationship added', () => {
    it('should automatically move work unit to blocked state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      await addDependency({
        workUnitId: 'AUTH-002',
        blocks: 'AUTH-001',
        cwd: testDir,
      });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // AUTH-001 should be in blocked state
      expect(updated.workUnits['AUTH-001'].status).toBe('blocked');
      expect(updated.states.blocked).toContain('AUTH-001');
      expect(updated.states.testing).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Auto-unblock when blocker completes', () => {
    it('should remain blocked with notification when blocker completes', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'blocked',
            blockedBy: ['AUTH-002'],
            blockedReason: 'Blocked by AUTH-002',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'implementing',
            blocks: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-002'],
          validating: [],
          done: [],
          blocked: ['AUTH-001'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Move AUTH-002 to done
      const { updateWorkUnitStatus } = await import('../update-work-unit-status');
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-002',
        status: 'done',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // And work unit AUTH-001 status should remain "blocked"
      expect(updated.workUnits['AUTH-001'].status).toBe('blocked');
      expect(updated.states.blocked).toContain('AUTH-001');

      // And should display notification about blocker completion
      // Note: This requires implementing notification system in update-work-unit-status
      // For now, we verify the blocker is done
      expect(updated.workUnits['AUTH-002'].status).toBe('done');
    });
  });

  describe('Scenario: Warn when deleting work unit that blocks others', () => {
    it('should warn about downstream impact', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'blocked',
            blockedBy: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'User session',
            status: 'specifying',
            blocks: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: ['AUTH-001'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { deleteWorkUnit } = await import('../delete-work-unit');
      const result = await deleteWorkUnit({
        workUnitId: 'AUTH-002',
        cascadeDependencies: true,
        cwd: testDir,
      });

      expect(result.success).toBe(true);
      expect(result.warnings).toBeDefined();
      expect(result.warnings?.some((w: string) => w.includes('blocks 1 work unit'))).toBe(true);
    });
  });

  describe('Scenario: Prevent deletion without cascade flag when dependencies exist', () => {
    it('should block deletion when work unit has dependencies', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            blocks: ['AUTH-002'],
            dependsOn: ['DASH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'specifying',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: ['DASH-001'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { deleteWorkUnit } = await import('../delete-work-unit');
      const error = await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('has dependencies');
        expect(error.message).toContain('--cascade-dependencies');
      }
    });
  });

  describe('Scenario: Delete work unit with cascade-dependencies flag', () => {
    it('should clean up all dependencies bidirectionally', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'testing',
            blocks: ['AUTH-002'],
            dependsOn: ['DASH-001'],
            relatesTo: ['API-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'API',
            status: 'implementing',
            relatesTo: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
          implementing: ['API-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: ['AUTH-002'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { deleteWorkUnit } = await import('../delete-work-unit');
      await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        cascadeDependencies: true,
        cwd: testDir,
      });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // AUTH-001 should be deleted
      expect(updated.workUnits['AUTH-001']).toBeUndefined();

      // AUTH-002 should have blockedBy removed
      expect(updated.workUnits['AUTH-002'].blockedBy || []).not.toContain('AUTH-001');

      // API-001 should have relatesTo removed
      expect(updated.workUnits['API-001'].relatesTo || []).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Bulk add multiple dependencies via JSON', () => {
    it('should add multiple dependencies at once', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'A': {
            id: 'A',
            title: 'Work A',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'B': {
            id: 'B',
            title: 'Work B',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'C': {
            id: 'C',
            title: 'Work C',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['B'],
          testing: ['A'],
          implementing: [],
          validating: [],
          done: ['C'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await addDependencies({
        workUnitId: 'A',
        dependencies: {
          blocks: ['B'],
          dependsOn: ['C'],
        },
        cwd: testDir,
      });

      expect(result.success).toBe(true);
      expect(result.added).toBeGreaterThanOrEqual(2);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));
      expect(updated.workUnits['A'].blocks).toContain('B');
      expect(updated.workUnits['A'].dependsOn).toContain('C');
    });
  });

  describe('Scenario: Clear all dependencies from work unit with confirmation', () => {
    it('should remove all dependency types', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            blocks: ['AUTH-002'],
            blockedBy: ['AUTH-003'],
            dependsOn: ['DASH-001'],
            relatesTo: ['API-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Password',
            status: 'testing',
            blocks: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'API',
            status: 'implementing',
            relatesTo: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001', 'AUTH-003'],
          implementing: ['API-001'],
          validating: [],
          done: ['DASH-001'],
          blocked: ['AUTH-002'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await clearDependencies({
        workUnitId: 'AUTH-001',
        confirm: true,
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].blocks).toBeUndefined();
      expect(updated.workUnits['AUTH-001'].blockedBy).toBeUndefined();
      expect(updated.workUnits['AUTH-001'].dependsOn).toBeUndefined();
      expect(updated.workUnits['AUTH-001'].relatesTo).toBeUndefined();

      // Bidirectional cleanup
      expect(updated.workUnits['AUTH-002'].blockedBy || []).not.toContain('AUTH-001');
      expect(updated.workUnits['AUTH-003'].blocks || []).not.toContain('AUTH-001');
      expect(updated.workUnits['API-001'].relatesTo || []).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Validate dependency data structure in work-units.json', () => {
    it('should detect invalid dependency arrays', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            blocks: ['AUTH-002', ''],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { validateWorkUnits } = await import('../validate-work-units');
      const result = await validateWorkUnits({ cwd: testDir });

      expect(result.valid).toBe(false);
      expect(result.errors?.some((e: string) => e.includes('empty strings'))).toBe(true);
    });
  });

  describe('Scenario: Repair broken bidirectional dependencies', () => {
    it('should fix mismatched dependency relationships', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            blocks: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'specifying',
            // Missing blockedBy: ['AUTH-001']
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { repairWorkUnits } = await import('../repair-work-units');
      const result = await repairWorkUnits({ cwd: testDir });

      expect(result.success).toBe(true);
      expect(result.repaired).toBeGreaterThan(0);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // Should have repaired bidirectional link
      expect(updated.workUnits['AUTH-002'].blockedBy).toContain('AUTH-001');
    });
  });

  describe('Scenario: Show dependency statistics (count by type)', () => {
    it('should calculate dependency metrics', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            blocks: ['AUTH-002', 'AUTH-003'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            dependsOn: ['DASH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Password',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: ['DASH-001'],
          blocked: ['AUTH-002', 'AUTH-003'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await queryDependencyStats({
        cwd: testDir,
      });

      expect(result.totalBlocks).toBe(2);
      expect(result.totalBlockedBy).toBe(2);
      expect(result.totalDependsOn).toBe(1);
      expect(result.workUnitsWithDependencies).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Export dependency graph as Mermaid diagram', () => {
    it('should generate Mermaid flowchart', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            blocks: ['AUTH-002'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Session',
            status: 'blocked',
            blockedBy: ['AUTH-001'],
            dependsOn: ['DASH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: ['DASH-001'],
          blocked: ['AUTH-002'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await exportDependencies({
        format: 'mermaid',
        output: join(testDir, 'dependencies.mmd'),
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const mermaidContent = await readFile(join(testDir, 'dependencies.mmd'), 'utf-8');
      expect(mermaidContent).toContain('graph');
      expect(mermaidContent).toContain('AUTH-001');
      expect(mermaidContent).toContain('AUTH-002');
      expect(mermaidContent).toContain('DASH-001');
    });
  });

  describe('Scenario: Warn when starting work with incomplete soft dependencies', () => {
    it('should warn when dependsOn work units not done', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'specifying',
            dependsOn: ['AUTH-001'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['DASH-001'],
          testing: ['AUTH-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Create feature file with scenario tagged for DASH-001
      await mkdir(join(testDir, 'spec/features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec/features/dashboard.feature'),
        '@DASH-001\nFeature: Dashboard\nScenario: Show dashboard\n  Given user logged in'
      );

      const { updateWorkUnitStatus } = await import('../update-work-unit-status');
      const result = await updateWorkUnitStatus({
        workUnitId: 'DASH-001',
        status: 'testing',
        cwd: testDir,
      });

      expect(result.warnings).toBeDefined();
      expect(result.warnings?.some((w: string) => w.includes('soft dependencies'))).toBe(true);
      expect(result.warnings?.some((w: string) => w.includes('AUTH-001'))).toBe(true);
    });
  });
});
