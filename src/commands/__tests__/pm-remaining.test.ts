import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import {
  assignEstimate,
  recordTokens,
  incrementIteration,
  calculateCycleTime,
  queryEstimateAccuracy,
  queryEstimateAccuracyByPrefix,
  queryEstimationGuide
} from '../estimation';
import {
  createEpic,
  createPrefix,
  showEpicProgress,
  listEpics,
  deleteEpic
} from '../epics';
import {
  queryWorkUnitsCompound,
  generateStatisticalReport,
  exportWorkUnits,
  displayKanbanBoard
} from '../query';
import {
  recordWorkUnitIteration,
  recordWorkUnitTokens,
  autoAdvanceWorkUnitState,
  validateWorkUnitSpecAlignment
} from '../workflow-automation';

describe('Feature: Work Unit Estimation and Metrics', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    await mkdir(specDir, { recursive: true });
    await writeFile(workUnitsFile, JSON.stringify({ workUnits: {}, states: {} }, null, 2));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Assign story points to work unit', () => {
    it('should set estimate field', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await assignEstimate('AUTH-001', 5, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].estimate).toBe(5);
    });
  });

  describe('Scenario: Use Fibonacci sequence for estimates', () => {
    it('should accept valid Fibonacci values', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      for (const value of [1, 2, 3, 5, 8, 13, 21]) {
        await expect(assignEstimate('AUTH-001', value, { cwd: testDir })).resolves.not.toThrow();
      }
    });
  });

  describe('Scenario: Reject non-Fibonacci estimate', () => {
    it('should fail for invalid values', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await expect(
        assignEstimate('AUTH-001', 7, { cwd: testDir })
      ).rejects.toThrow('must be Fibonacci');

      await expect(
        assignEstimate('AUTH-001', 7, { cwd: testDir })
      ).rejects.toThrow('1, 2, 3, 5, 8, 13, 21');
    });
  });

  describe('Scenario: Record tokens consumed during implementation', () => {
    it('should track actualTokens field', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'implementing',
        metrics: { actualTokens: 0 },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await recordTokens('AUTH-001', 15000, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].metrics.actualTokens).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Increment iteration count', () => {
    it('should increment iterations field for rework tracking', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'implementing',
        metrics: { iterations: 1 },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await incrementIteration('AUTH-001', { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].metrics.iterations).toBe(2);
    });
  });

  describe('Scenario: Calculate cycle time from state history', () => {
    it('should compute time from backlog to done', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'done',
        stateHistory: [
          { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
          { state: 'specifying', timestamp: '2025-01-15T11:00:00Z' },
          { state: 'testing', timestamp: '2025-01-15T13:00:00Z' },
          { state: 'implementing', timestamp: '2025-01-15T14:00:00Z' },
          { state: 'done', timestamp: '2025-01-15T18:00:00Z' }
        ],
        createdAt: '2025-01-15T10:00:00Z',
        updatedAt: '2025-01-15T18:00:00Z'
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const cycleTime = await calculateCycleTime('AUTH-001', { cwd: testDir });

      expect(cycleTime).toBe(8);
    });
  });

  describe('Scenario: Compare estimate vs actual for completed work unit', () => {
    it('should show estimated vs actual comparison', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'done',
        estimate: 5,
        actualTokens: 95000,
        iterations: 2,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryEstimateAccuracy('AUTH-001', { cwd: testDir });

      expect(result.estimated).toBe(5);
      expect(result.actualTokens).toBe(95000);
      expect(result.iterations).toBe(2);
      expect(result.comparison).toContain('expected range');
    });
  });

  describe('Scenario: Analyze estimate accuracy across all completed work', () => {
    it('should calculate average tokens per story point', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'done',
        estimate: 1,
        actualTokens: 22000,
        iterations: 1
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'done',
        estimate: 1,
        actualTokens: 28000,
        iterations: 2
      };
      workUnits.workUnits['AUTH-003'] = {
        id: 'AUTH-003',
        status: 'done',
        estimate: 3,
        actualTokens: 70000,
        iterations: 2
      };
      workUnits.workUnits['AUTH-004'] = {
        id: 'AUTH-004',
        status: 'done',
        estimate: 3,
        actualTokens: 80000,
        iterations: 3
      };
      workUnits.workUnits['AUTH-005'] = {
        id: 'AUTH-005',
        status: 'done',
        estimate: 5,
        actualTokens: 95000,
        iterations: 2
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryEstimateAccuracy(null, { cwd: testDir, output: 'json' });
      const data = JSON.parse(result);

      expect(data['1-point'].avgTokens).toBe(25000);
      expect(data['1-point'].avgIterations).toBe(1.5);
      expect(data['1-point'].samples).toBe(2);

      expect(data['3-point'].avgTokens).toBe(75000);
      expect(data['3-point'].avgIterations).toBe(2.5);
      expect(data['3-point'].samples).toBe(2);

      expect(data['5-point'].avgTokens).toBe(95000);
      expect(data['5-point'].avgIterations).toBe(2);
      expect(data['5-point'].samples).toBe(1);
    });
  });

  describe('Scenario: Analyze estimate accuracy by prefix', () => {
    it('should show accuracy and recommendations per prefix', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      // AUTH: expected 160k (8*20k), actual 165k = 3% over (well-calibrated)
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'done',
        estimate: 5,
        actualTokens: 95000
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'done',
        estimate: 3,
        actualTokens: 70000
      };
      // SEC: expected 160k (8*20k), actual 235k = 47% over (estimates too low)
      workUnits.workUnits['SEC-001'] = {
        id: 'SEC-001',
        status: 'done',
        estimate: 5,
        actualTokens: 140000
      };
      workUnits.workUnits['SEC-002'] = {
        id: 'SEC-002',
        status: 'done',
        estimate: 3,
        actualTokens: 95000
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryEstimateAccuracyByPrefix({ cwd: testDir, output: 'json' });
      const data = JSON.parse(result);

      // AUTH is well-calibrated (within 10%)
      expect(data.AUTH.avgAccuracy).toMatch(/\d+% (low|high)/);
      expect(data.AUTH.recommendation).toContain('well-calibrated');

      // SEC estimates are too low by ~47%
      expect(data.SEC.avgAccuracy).toMatch(/4[0-9]% low/);
      expect(data.SEC.recommendation).toContain('increase estimates');
    });
  });

  describe('Scenario: Get estimation recommendations for new work', () => {
    it('should provide guidance based on historical patterns', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      // Add multiple completed work units with VARIED actual tokens to create ranges
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'done',
        estimate: 1,
        actualTokens: 20000,
        iterations: 1
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'done',
        estimate: 1,
        actualTokens: 25000,
        iterations: 2
      };
      workUnits.workUnits['AUTH-003'] = {
        id: 'AUTH-003',
        status: 'done',
        estimate: 1,
        actualTokens: 30000,
        iterations: 2
      };
      workUnits.workUnits['AUTH-004'] = {
        id: 'AUTH-004',
        status: 'done',
        estimate: 3,
        actualTokens: 60000,
        iterations: 2
      };
      workUnits.workUnits['AUTH-005'] = {
        id: 'AUTH-005',
        status: 'done',
        estimate: 3,
        actualTokens: 75000,
        iterations: 2
      };
      workUnits.workUnits['AUTH-006'] = {
        id: 'AUTH-006',
        status: 'done',
        estimate: 3,
        actualTokens: 90000,
        iterations: 3
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryEstimationGuide({ cwd: testDir });

      expect(result).toContain('1 point');
      expect(result).toContain('20k-30k');
      expect(result).toContain('1-2');
      expect(result).toContain('high');

      expect(result).toContain('3 point');
      expect(result).toContain('60k-90k');
      expect(result).toContain('2-3');
    });
  });
});

describe('Feature: Epic and Prefix Management', () => {
  let testDir: string;
  let specDir: string;
  let epicsFile: string;
  let prefixesFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    epicsFile = join(specDir, 'epics.json');
    prefixesFile = join(specDir, 'prefixes.json');
    await mkdir(specDir, { recursive: true });
    await writeFile(epicsFile, JSON.stringify({ epics: {} }, null, 2));
    await writeFile(prefixesFile, JSON.stringify({ prefixes: {} }, null, 2));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create epic', () => {
    it('should add epic to epics.json', async () => {
      await createEpic('epic-user-management', 'User Management', { cwd: testDir });

      const epics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      expect(epics.epics['epic-user-management']).toBeDefined();
      expect(epics.epics['epic-user-management'].title).toBe('User Management');
    });
  });

  describe('Scenario: Create prefix for work unit IDs', () => {
    it('should add prefix to prefixes.json', async () => {
      await createPrefix('AUTH', 'Authentication features', { cwd: testDir });

      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      expect(prefixes.prefixes.AUTH).toBeDefined();
      expect(prefixes.prefixes.AUTH.description).toBe('Authentication features');
    });
  });

  describe('Scenario: Reject invalid epic ID format', () => {
    it('should require lowercase-with-hyphens', async () => {
      await expect(
        createEpic('EpicUserManagement', 'User Management', { cwd: testDir })
      ).rejects.toThrow('must be lowercase with hyphens');

      await expect(
        createEpic('epic_user_management', 'User Management', { cwd: testDir })
      ).rejects.toThrow('must be lowercase with hyphens');
    });
  });

  describe('Scenario: Reject invalid prefix format', () => {
    it('should require 2-6 uppercase letters', async () => {
      await expect(
        createPrefix('a', 'Too short', { cwd: testDir })
      ).rejects.toThrow('2-6 uppercase letters');

      await expect(
        createPrefix('auth', 'Lowercase', { cwd: testDir })
      ).rejects.toThrow('2-6 uppercase letters');

      await expect(
        createPrefix('TOOLONG', 'Too long', { cwd: testDir })
      ).rejects.toThrow('2-6 uppercase letters');
    });
  });

  describe('Scenario: Show epic progress', () => {
    it('should display work units and completion percentage', async () => {
      const epics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      epics.epics['epic-user-management'] = {
        id: 'epic-user-management',
        title: 'User Management',
        workUnits: ['AUTH-001', 'AUTH-002', 'AUTH-003']
      };
      await writeFile(epicsFile, JSON.stringify(epics, null, 2));

      const workUnitsFile = join(specDir, 'work-units.json');
      await writeFile(workUnitsFile, JSON.stringify({
        workUnits: {
          'AUTH-001': { id: 'AUTH-001', status: 'done', estimate: 5 },
          'AUTH-002': { id: 'AUTH-002', status: 'implementing', estimate: 8 },
          'AUTH-003': { id: 'AUTH-003', status: 'backlog', estimate: 3 }
        }
      }, null, 2));

      const output = await showEpicProgress('epic-user-management', { cwd: testDir });

      expect(output).toContain('User Management');
      expect(output).toContain('1 of 3 complete');
      expect(output).toContain('33%');
      expect(output).toContain('5 of 16 points');
    });
  });

  describe('Scenario: List all epics with progress', () => {
    it('should show all epics with completion status', async () => {
      const epics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      epics.epics['epic-user-management'] = {
        id: 'epic-user-management',
        title: 'User Management',
        workUnits: ['AUTH-001']
      };
      epics.epics['epic-security'] = {
        id: 'epic-security',
        title: 'Security Improvements',
        workUnits: ['SEC-001', 'SEC-002']
      };
      await writeFile(epicsFile, JSON.stringify(epics, null, 2));

      const output = await listEpics({ cwd: testDir });

      expect(output).toContain('epic-user-management');
      expect(output).toContain('User Management');
      expect(output).toContain('epic-security');
      expect(output).toContain('Security Improvements');
    });
  });

  describe('Scenario: Link prefix to epic', () => {
    it('should link prefix to epic', async () => {
      const { updatePrefix } = await import('../epics');

      // Create epic and prefix
      await createEpic('epic-auth', 'Authentication', { cwd: testDir });
      await createPrefix('AUTH', 'Authentication features', { cwd: testDir });

      // Link prefix to epic
      await updatePrefix('AUTH', { epic: 'epic-auth' }, { cwd: testDir });

      // Verify link
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      expect(prefixes.prefixes.AUTH.epic).toBe('epic-auth');
    });
  });

  describe('Scenario: Delete epic and unlink work units', () => {
    it('should delete epic and clear epic field from work units', async () => {
      // Create epic
      const epics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      epics.epics['epic-auth'] = {
        id: 'epic-auth',
        title: 'Authentication',
        workUnits: ['AUTH-001', 'AUTH-002']
      };
      await writeFile(epicsFile, JSON.stringify(epics, null, 2));

      // Create work units with epic field
      const workUnitsFile = join(specDir, 'work-units.json');
      await writeFile(workUnitsFile, JSON.stringify({
        workUnits: {
          'AUTH-001': { id: 'AUTH-001', status: 'done', epic: 'epic-auth' },
          'AUTH-002': { id: 'AUTH-002', status: 'backlog', epic: 'epic-auth' }
        }
      }, null, 2));

      // Delete epic with force flag
      await deleteEpic('epic-auth', { force: true }, { cwd: testDir });

      // Verify epic deleted
      const updatedEpics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      expect(updatedEpics.epics['epic-auth']).toBeUndefined();

      // Verify work units have epic field cleared
      const updatedWorkUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updatedWorkUnits.workUnits['AUTH-001'].epic).toBeUndefined();
      expect(updatedWorkUnits.workUnits['AUTH-002'].epic).toBeUndefined();
    });
  });
});

describe('Feature: Work Unit Query and Reporting', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    await mkdir(specDir, { recursive: true });
    await writeFile(workUnitsFile, JSON.stringify({ workUnits: {}, states: {} }, null, 2));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Query work units by status', () => {
    it('should filter work units by status', async () => {
      const { queryWorkUnitsByStatus } = await import('../query');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = { id: 'AUTH-001', status: 'implementing', title: 'OAuth' };
      workUnits.workUnits['AUTH-002'] = { id: 'AUTH-002', status: 'backlog', title: 'Login' };
      workUnits.workUnits['SEC-001'] = { id: 'SEC-001', status: 'implementing', title: 'Encryption' };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryWorkUnitsByStatus('implementing', { cwd: testDir, output: 'json' });
      const json = JSON.parse(result);

      expect(json).toHaveLength(2);
      expect(json.some((wu: { id: string }) => wu.id === 'AUTH-001')).toBe(true);
      expect(json.some((wu: { id: string }) => wu.id === 'SEC-001')).toBe(true);
    });
  });

  describe('Scenario: Query work units by epic', () => {
    it('should filter work units by epic', async () => {
      const { queryWorkUnitsByEpic } = await import('../query');
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = { id: 'AUTH-001', epic: 'epic-auth', title: 'OAuth' };
      workUnits.workUnits['AUTH-002'] = { id: 'AUTH-002', epic: 'epic-auth', title: 'Login' };
      workUnits.workUnits['SEC-001'] = { id: 'SEC-001', epic: 'epic-security', title: 'Encryption' };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryWorkUnitsByEpic('epic-auth', { cwd: testDir, output: 'json' });
      const json = JSON.parse(result);

      expect(json).toHaveLength(2);
      expect(json.some((wu: { id: string }) => wu.id === 'AUTH-001')).toBe(true);
      expect(json.some((wu: { id: string }) => wu.id === 'AUTH-002')).toBe(true);
    });
  });

  describe('Scenario: Query with compound filters', () => {
    it('should support AND/OR logic', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'implementing',
        estimate: 5,
        epic: 'epic-user-management'
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'backlog',
        estimate: 8,
        epic: 'epic-user-management'
      };
      workUnits.workUnits['SEC-001'] = {
        id: 'SEC-001',
        status: 'implementing',
        estimate: 3,
        epic: 'epic-security'
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const result = await queryWorkUnitsCompound({
        status: 'implementing',
        epic: 'epic-user-management',
        output: 'json'
      }, { cwd: testDir });

      const json = JSON.parse(result);
      expect(json).toHaveLength(1);
      expect(json[0].id).toBe('AUTH-001');
    });
  });

  describe('Scenario: Display Kanban board view', () => {
    it('should display work units organized by state columns', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = { id: 'AUTH-001', status: 'backlog', title: 'OAuth', estimate: 5 };
      workUnits.workUnits['AUTH-002'] = { id: 'AUTH-002', status: 'implementing', title: 'Login', estimate: 8 };
      workUnits.workUnits['SEC-001'] = { id: 'SEC-001', status: 'done', title: 'Encryption', estimate: 3 };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const board = await displayKanbanBoard({ cwd: testDir });

      expect(board).toContain('Backlog');
      expect(board).toContain('Implementing');
      expect(board).toContain('Done');
      expect(board).toContain('AUTH-001');
      expect(board).toContain('OAuth');
      expect(board).toContain('5');
      expect(board).toContain('AUTH-002');
      expect(board).toContain('Login');
      expect(board).toContain('8');
      expect(board).toContain('SEC-001');
      expect(board).toContain('Encryption');
      expect(board).toContain('3');
    });
  });

  describe('Scenario: Generate statistical report', () => {
    it('should calculate aggregate metrics', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = { id: 'AUTH-001', status: 'done', estimate: 5 };
      workUnits.workUnits['AUTH-002'] = { id: 'AUTH-002', status: 'implementing', estimate: 8 };
      workUnits.workUnits['AUTH-003'] = { id: 'AUTH-003', status: 'backlog', estimate: 3 };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const report = await generateStatisticalReport({ cwd: testDir });

      expect(report).toContain('Total work units: 3');
      expect(report).toContain('Total story points: 16');
      expect(report).toContain('Completed: 5 points');
      expect(report).toContain('In progress: 8 points');
      expect(report).toContain('Remaining: 3 points');
    });
  });

  describe('Scenario: Export to multiple formats', () => {
    it('should support JSON, CSV, and Markdown', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'done',
        estimate: 5
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const jsonOutput = join(testDir, 'export.json');
      await exportWorkUnits({ output: jsonOutput, format: 'json' }, { cwd: testDir });
      const jsonContent = await readFile(jsonOutput, 'utf-8');
      expect(() => JSON.parse(jsonContent)).not.toThrow();

      const csvOutput = join(testDir, 'export.csv');
      await exportWorkUnits({ output: csvOutput, format: 'csv' }, { cwd: testDir });
      const csvContent = await readFile(csvOutput, 'utf-8');
      expect(csvContent).toContain('id,title,status,estimate');
      expect(csvContent).toContain('AUTH-001,OAuth,done,5');

      const mdOutput = join(testDir, 'export.md');
      await exportWorkUnits({ output: mdOutput, format: 'markdown' }, { cwd: testDir });
      const mdContent = await readFile(mdOutput, 'utf-8');
      expect(mdContent).toContain('| ID | Title | Status | Estimate |');
      expect(mdContent).toContain('| AUTH-001 | OAuth | done | 5 |');
    });
  });
});

describe('Feature: Workflow Automation', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    await mkdir(specDir, { recursive: true });
    await writeFile(workUnitsFile, JSON.stringify({ workUnits: {}, states: {} }, null, 2));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Record iteration after tool use', () => {
    it('should increment iterations field', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'implementing',
        metrics: { iterations: 0 }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await recordWorkUnitIteration('AUTH-001', { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].metrics.iterations).toBe(1);
    });
  });

  describe('Scenario: Record tokens consumed', () => {
    it('should accumulate token usage', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'implementing',
        metrics: { actualTokens: 0 }
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await recordWorkUnitTokens('AUTH-001', 5000, { cwd: testDir });
      await recordWorkUnitTokens('AUTH-001', 3000, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].metrics.actualTokens).toBe(8000);
    });
  });

  describe('Scenario: Auto-advance state after tests pass', () => {
    it('should move testing → implementing when tests complete', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'testing',
        stateHistory: [{ state: 'testing', timestamp: new Date().toISOString() }]
      };
      workUnits.states = { testing: ['AUTH-001'], implementing: [] };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await autoAdvanceWorkUnitState('AUTH-001', { fromState: 'testing', event: 'tests-pass' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].status).toBe('implementing');
    });
  });

  describe('Scenario: Validate spec alignment before commit', () => {
    it('should check scenarios exist for work unit', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'implementing'
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      const featuresDir = join(specDir, 'features');
      await mkdir(featuresDir, { recursive: true });
      await writeFile(
        join(featuresDir, 'auth.feature'),
        `@auth\nFeature: Auth\n\n@AUTH-001\nScenario: Login\nGiven test\nWhen test\nThen test`
      );

      const result = await validateWorkUnitSpecAlignment('AUTH-001', { cwd: testDir });

      expect(result.aligned).toBe(true);
      expect(result.scenariosFound).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Auto-mark done after validation succeeds', () => {
    it('should move validating → done when validation passes', async () => {
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'validating',
        stateHistory: [
          { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
          { state: 'validating', timestamp: new Date().toISOString() }
        ]
      };
      workUnits.states = { validating: ['AUTH-001'], done: [] };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      await autoAdvanceWorkUnitState('AUTH-001', { fromState: 'validating', event: 'validation-pass' }, { cwd: testDir });

      const updated = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updated.workUnits['AUTH-001'].status).toBe('done');

      // Verify completion timestamp was recorded in state history
      const stateHistory = updated.workUnits['AUTH-001'].stateHistory;
      const doneEntry = stateHistory.find((entry: { state: string }) => entry.state === 'done');
      expect(doneEntry).toBeDefined();
      expect(doneEntry.timestamp).toBeDefined();
    });
  });
});
