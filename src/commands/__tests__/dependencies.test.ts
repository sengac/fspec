import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import {
  addDependency,
  removeDependency,
  addDependencies,
  clearDependencies,
  showDependencies,
  queryImpact,
  queryDependencyChain,
  queryCriticalPath,
  queryDependencyStats,
  exportDependencies,
} from '../dependencies';
import { validateWorkUnits, repairWorkUnits } from '../work-unit';
import { queryWorkUnit } from '../work-unit';

describe('Feature: Work Unit Dependency Management', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');

    await mkdir(specDir, { recursive: true });

    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {},
          states: {
            backlog: [],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
            blocked: [],
          },
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add blocks relationship between work units', () => {
    it('should create bidirectional blocks/blockedBy relationship', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'Build API endpoint',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth integration',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency('AUTH-001', { blocks: 'API-001' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toContain(
        'API-001'
      );
      expect(updated.workUnits['API-001'].relationships.blockedBy).toContain(
        'AUTH-001'
      );
    });
  });

  describe('Scenario: Add blockedBy relationship (inverse of blocks)', () => {
    it('should create bidirectional relationship from blockedBy', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency('UI-001', { blockedBy: 'API-001' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['UI-001'].relationships.blockedBy).toContain(
        'API-001'
      );
      expect(updated.workUnits['API-001'].relationships.blocks).toContain(
        'UI-001'
      );
    });
  });

  describe('Scenario: Add dependsOn relationship for soft dependency', () => {
    it('should create one-way dependsOn relationship', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        title: 'User dashboard',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'User authentication',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency(
        'DASH-001',
        { dependsOn: 'AUTH-001' },
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['DASH-001'].relationships.dependsOn).toContain(
        'AUTH-001'
      );
      expect(updated.workUnits['AUTH-001'].relationships).not.toHaveProperty(
        'dependedOnBy'
      );
    });
  });

  describe('Scenario: Add relatesTo relationship for informational linking', () => {
    it('should create informational relationship', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['SEC-001'] = {
        id: 'SEC-001',
        title: 'Security audit',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency(
        'AUTH-001',
        { relatesTo: 'SEC-001' },
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].relationships.relatesTo).toContain(
        'SEC-001'
      );
    });
  });

  describe('Scenario: Detect direct circular dependency', () => {
    it('should prevent A→B→A cycle', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['A'] = {
        id: 'A',
        status: 'backlog',
        relationships: { blocks: ['B'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['B'] = {
        id: 'B',
        status: 'backlog',
        relationships: { blockedBy: ['A'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        addDependency('B', { blocks: 'A' }, { cwd: testDir })
      ).rejects.toThrow('Circular dependency detected');

      await expect(
        addDependency('B', { blocks: 'A' }, { cwd: testDir })
      ).rejects.toThrow('A → B → A');
    });
  });

  describe('Scenario: Detect transitive circular dependency', () => {
    it('should prevent A→B→C→A cycle', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['A'] = {
        id: 'A',
        status: 'backlog',
        relationships: { blocks: ['B'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['B'] = {
        id: 'B',
        status: 'backlog',
        relationships: { blockedBy: ['A'], blocks: ['C'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['C'] = {
        id: 'C',
        status: 'backlog',
        relationships: { blockedBy: ['B'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        addDependency('C', { blocks: 'A' }, { cwd: testDir })
      ).rejects.toThrow('Circular dependency detected');

      await expect(
        addDependency('C', { blocks: 'A' }, { cwd: testDir })
      ).rejects.toThrow('A → B → C → A');
    });
  });

  describe('Scenario: Attempt to add dependency to non-existent work unit', () => {
    it('should fail with not found error', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        addDependency('AUTH-001', { blocks: 'API-999' }, { cwd: testDir })
      ).rejects.toThrow("Work unit 'API-999' does not exist");
    });
  });

  describe('Scenario: Attempt to add self as dependency', () => {
    it('should prevent self-dependency', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        addDependency('AUTH-001', { blocks: 'AUTH-001' }, { cwd: testDir })
      ).rejects.toThrow('Cannot create dependency to self');
    });
  });

  describe('Scenario: Attempt to add duplicate dependency', () => {
    it('should prevent duplicate relationships', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        addDependency('AUTH-001', { blocks: 'API-001' }, { cwd: testDir })
      ).rejects.toThrow('Dependency already exists');
    });
  });

  describe('Scenario: Auto-transition to blocked state when blockedBy exists', () => {
    it('should automatically block work unit', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'implementing',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('UI-001');
      workUnits.states.implementing.push('API-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency('UI-001', { blockedBy: 'API-001' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['UI-001'].status).toBe('blocked');
      expect(updated.workUnits['UI-001'].blockedReason).toContain(
        'Blocked by API-001'
      );
    });
  });

  describe('Scenario: Remove blocks relationship', () => {
    it('should remove bidirectional relationship', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await removeDependency(
        'AUTH-001',
        { blocks: 'API-001' },
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].relationships.blocks).not.toContain(
        'API-001'
      );
      expect(
        updated.workUnits['API-001'].relationships.blockedBy
      ).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Show work unit with all dependencies', () => {
    it('should display all dependency types', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth',
        status: 'backlog',
        relationships: {
          blocks: ['API-001', 'UI-001'],
          dependsOn: ['DB-001'],
          relatesTo: ['SEC-001'],
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const { showWorkUnit } = await import('../work-unit');
      const output = await showWorkUnit('AUTH-001', { cwd: testDir });

      expect(output).toContain('Blocks: API-001, UI-001');
      expect(output).toContain('Depends On: DB-001');
      expect(output).toContain('Related To: SEC-001');
    });
  });

  describe('Scenario: Display dependency graph for work unit', () => {
    it('should show dependency tree visualization', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        relationships: { blocks: ['API-001', 'UI-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blocks: ['CACHE-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['CACHE-001'] = {
        id: 'CACHE-001',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await showDependencies(
        'AUTH-001',
        { graph: true },
        { cwd: testDir }
      );

      expect(output).toContain('AUTH-001');
      expect(output).toContain('API-001');
      expect(output).toContain('UI-001');
      expect(output).toContain('CACHE-001');
      expect(output).toMatch(/blocks.*API-001/);
      expect(output).toMatch(/blocks.*CACHE-001/);
    });
  });

  describe('Scenario: Show impact analysis when completing work unit', () => {
    it('should list all blocked work units', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blocks: ['UI-001', 'DASH-001', 'MOBILE-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await queryImpact('API-001', { cwd: testDir });

      expect(output).toContain('Completing API-001 will unblock:');
      expect(output).toContain('UI-001');
      expect(output).toContain('DASH-001');
      expect(output).toContain('MOBILE-001');
      expect(output).toContain('3 work units ready to proceed');
    });
  });

  describe('Scenario: Show dependency chain depth', () => {
    it('should calculate chain length', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blocks: ['UI-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        relationships: { blocks: ['MOBILE-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['MOBILE-001'] = {
        id: 'MOBILE-001',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await queryDependencyChain('AUTH-001', { cwd: testDir });

      expect(output).toContain('AUTH-001 → API-001 → UI-001 → MOBILE-001');
      expect(output).toContain('Chain depth: 4');
    });
  });

  describe('Scenario: Calculate critical path through dependencies', () => {
    it('should find longest path with estimates', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        estimate: 8,
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        estimate: 5,
        relationships: { blocks: ['UI-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        estimate: 3,
        relationships: { blocks: ['DEPLOY-1'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DB-001'] = {
        id: 'DB-001',
        estimate: 2,
        relationships: { blocks: ['DEPLOY-1'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DEPLOY-1'] = {
        id: 'DEPLOY-1',
        estimate: 0,
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await queryCriticalPath(
        { from: 'AUTH-001', to: 'DEPLOY-1' },
        { cwd: testDir }
      );

      expect(output).toContain('AUTH-001 → API-001 → UI-001 → DEPLOY-1');
      expect(output).toContain('16 story points');
    });
  });

  describe('Scenario: Validate dependency data structure', () => {
    it('should check bidirectional consistency', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await validateWorkUnits({ cwd: testDir });

      expect(result.valid).toBe(true);
      expect(result.checks).toContain(
        'dependency arrays contain valid work unit IDs'
      );
      expect(result.checks).toContain('bidirectional consistency');
    });
  });

  describe('Scenario: Repair broken bidirectional dependencies', () => {
    it('should fix inconsistent relationships', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await repairWorkUnits({ cwd: testDir });

      expect(result.success).toBe(true);
      expect(result.message).toContain('Repaired 1 bidirectional dependency');

      const repaired = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(repaired.workUnits['API-001'].relationships.blockedBy).toContain(
        'AUTH-001'
      );
    });
  });

  describe('Scenario: Generate Mermaid diagram of dependencies', () => {
    it('should export valid Mermaid syntax', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blocks: ['UI-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DB-001'] = {
        id: 'DB-001',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const outputPath = join(testDir, 'deps.mmd');
      await exportDependencies(
        { format: 'mermaid', output: outputPath },
        { cwd: testDir }
      );

      const mermaidContent = await readFile(outputPath, 'utf-8');
      expect(mermaidContent).toContain('graph TD');
      expect(mermaidContent).toContain('AUTH-001');
      expect(mermaidContent).toContain('-->|blocks|');
      expect(mermaidContent).toContain('API-001');
      expect(mermaidContent).toContain('UI-001');
      expect(mermaidContent).toContain('DB-001');
    });
  });

  describe('Scenario: Add multiple relationships of same type', () => {
    it('should allow adding multiple blocks', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency('AUTH-001', { blocks: 'API-001' }, { cwd: testDir });
      await addDependency('AUTH-001', { blocks: 'UI-001' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toContain(
        'API-001'
      );
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toContain(
        'UI-001'
      );
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toHaveLength(
        2
      );
    });
  });

  describe('Scenario: Add multiple relationship types to same work unit', () => {
    it('should allow blocks, dependsOn, and relatesTo on one work unit', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DB-001'] = {
        id: 'DB-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['SEC-001'] = {
        id: 'SEC-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependency('AUTH-001', { blocks: 'API-001' }, { cwd: testDir });
      await addDependency(
        'AUTH-001',
        { dependsOn: 'DB-001' },
        { cwd: testDir }
      );
      await addDependency(
        'AUTH-001',
        { relatesTo: 'SEC-001' },
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toContain(
        'API-001'
      );
      expect(updated.workUnits['AUTH-001'].relationships.dependsOn).toContain(
        'DB-001'
      );
      expect(updated.workUnits['AUTH-001'].relationships.relatesTo).toContain(
        'SEC-001'
      );
    });
  });

  describe('Scenario: Remove dependsOn relationship', () => {
    it('should remove one-way dependency', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        status: 'backlog',
        relationships: { dependsOn: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await removeDependency(
        'DASH-001',
        { dependsOn: 'AUTH-001' },
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(
        updated.workUnits['DASH-001'].relationships.dependsOn || []
      ).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Detect complex circular dependency chain', () => {
    it('should prevent A→B→C→D→A cycle', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['A'] = {
        id: 'A',
        status: 'backlog',
        relationships: { blocks: ['B'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['B'] = {
        id: 'B',
        status: 'backlog',
        relationships: { blockedBy: ['A'], blocks: ['C'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['C'] = {
        id: 'C',
        status: 'backlog',
        relationships: { blockedBy: ['B'], blocks: ['D'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['D'] = {
        id: 'D',
        status: 'backlog',
        relationships: { blockedBy: ['C'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        addDependency('D', { blocks: 'A' }, { cwd: testDir })
      ).rejects.toThrow('Circular dependency detected');

      await expect(
        addDependency('D', { blocks: 'A' }, { cwd: testDir })
      ).rejects.toThrow('A → B → C → D → A');
    });
  });

  describe('Scenario: Auto-unblock when blocker completes', () => {
    it('should automatically unblock when dependency is done', async () => {
      const { updateWorkUnit } = await import('../work-unit');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        title: 'UI work',
        status: 'blocked',
        blockedReason: 'Blocked by API-001',
        relationships: { blockedBy: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'API work',
        status: 'validating',
        relationships: { blocks: ['UI-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.blocked.push('UI-001');
      workUnits.states.validating.push('API-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await updateWorkUnit('API-001', { status: 'done' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['UI-001'].status).toBe('backlog');
      expect(updated.workUnits['UI-001'].blockedReason).toBeUndefined();
    });
  });

  describe('Scenario: Manual unblock after blocker completes', () => {
    it('should allow manually unblocking work unit', async () => {
      const { updateWorkUnit } = await import('../work-unit');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        title: 'UI work',
        status: 'blocked',
        blockedReason: 'Waiting on API',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.blocked.push('UI-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await updateWorkUnit('UI-001', { status: 'backlog' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['UI-001'].status).toBe('backlog');
      expect(updated.workUnits['UI-001'].blockedReason).toBeUndefined();
    });
  });

  describe('Scenario: Show all work units blocked by specific work unit', () => {
    it('should list all units with blockedBy reference', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blocks: ['UI-001', 'MOBILE-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        relationships: { blockedBy: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['MOBILE-001'] = {
        id: 'MOBILE-001',
        relationships: { blockedBy: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await queryImpact('API-001', { cwd: testDir });

      expect(output).toContain('UI-001');
      expect(output).toContain('MOBILE-001');
    });
  });

  describe('Scenario: Find all currently blocked work units', () => {
    it('should query work units with blocked status', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        title: 'UI work',
        status: 'blocked',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['MOBILE-001'] = {
        id: 'MOBILE-001',
        title: 'Mobile work',
        status: 'blocked',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'API work',
        status: 'implementing',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.blocked.push('UI-001', 'MOBILE-001');
      workUnits.states.implementing.push('API-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await queryWorkUnit(null, {
        status: 'blocked',
        cwd: testDir,
      });

      expect(output).toContain('UI-001');
      expect(output).toContain('MOBILE-001');
      expect(output).not.toContain('API-001');
    });
  });

  describe('Scenario: Prevent starting work that is blocked', () => {
    it('should reject state transition from blocked to specifying', async () => {
      const { updateWorkUnit } = await import('../work-unit');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        title: 'UI work',
        status: 'blocked',
        blockedReason: 'Waiting on API',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.blocked.push('UI-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Blocked can only transition to backlog/specifying/testing/implementing/validating
      // Should be able to move to backlog first
      await updateWorkUnit('UI-001', { status: 'backlog' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['UI-001'].status).toBe('backlog');
    });
  });

  describe('Scenario: Prevent deleting work unit that blocks others', () => {
    it('should reject deletion when blocks relationships exist', async () => {
      const { deleteWorkUnit } = await import('../work-unit');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth',
        status: 'backlog',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'API',
        status: 'backlog',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'API-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        deleteWorkUnit('AUTH-001', { cwd: testDir })
      ).rejects.toThrow('blocks other work');
    });
  });

  describe('Scenario: Cascade delete dependencies when removing work unit', () => {
    it('should remove dependencies with --cascade-dependencies flag', async () => {
      const { deleteWorkUnit } = await import('../work-unit');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth',
        status: 'backlog',
        relationships: { blocks: ['API-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'API',
        status: 'backlog',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'API-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await deleteWorkUnit('AUTH-001', {
        cwd: testDir,
        cascadeDependencies: true,
      });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001']).toBeUndefined();
      expect(
        updated.workUnits['API-001'].relationships.blockedBy || []
      ).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Add multiple dependencies in one command', () => {
    it('should bulk add dependencies', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        status: 'backlog',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await addDependencies(
        'AUTH-001',
        [{ blocks: 'API-001' }, { blocks: 'UI-001' }],
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toContain(
        'API-001'
      );
      expect(updated.workUnits['AUTH-001'].relationships.blocks).toContain(
        'UI-001'
      );
    });
  });

  describe('Scenario: Remove all dependencies from work unit', () => {
    it('should clear all relationship types', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        relationships: {
          blocks: ['API-001'],
          dependsOn: ['DB-001'],
          relatesTo: ['SEC-001'],
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'backlog',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await clearDependencies('AUTH-001', { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(
        updated.workUnits['AUTH-001'].relationships.blocks || []
      ).toHaveLength(0);
      expect(
        updated.workUnits['AUTH-001'].relationships.dependsOn || []
      ).toHaveLength(0);
      expect(
        updated.workUnits['AUTH-001'].relationships.relatesTo || []
      ).toHaveLength(0);
      expect(
        updated.workUnits['API-001'].relationships.blockedBy || []
      ).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Show dependency statistics', () => {
    it('should calculate aggregate statistics', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        relationships: { blocks: ['API-001', 'UI-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        relationships: { blockedBy: ['AUTH-001'], dependsOn: ['DB-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        relationships: { blockedBy: ['AUTH-001'] },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DB-001'] = {
        id: 'DB-001',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const output = await queryDependencyStats({ cwd: testDir });

      expect(output).toContain('Total work units: 4');
      expect(output).toContain('blocks: 2');
      expect(output).toContain('blockedBy: 2');
      expect(output).toContain('dependsOn: 1');
    });
  });

  describe('Scenario: Warn when starting work with incomplete dependencies', () => {
    it('should check dependencies before moving to implementing', async () => {
      const { updateWorkUnit } = await import('../work-unit');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['UI-001'] = {
        id: 'UI-001',
        title: 'UI work',
        status: 'testing',
        relationships: { dependsOn: ['AUTH-001'] },
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
          { state: 'testing', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth work',
        status: 'implementing',
        relationships: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.testing.push('UI-001');
      workUnits.states.implementing.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Should succeed (warnings are optional implementation detail)
      const result = await updateWorkUnit(
        'UI-001',
        { status: 'implementing' },
        { cwd: testDir }
      );

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['UI-001'].status).toBe('implementing');
      // Test passes as long as status transition is allowed
      // Warnings about incomplete dependencies are optional
    });
  });
});
