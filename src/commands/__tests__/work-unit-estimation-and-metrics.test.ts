import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitEstimate } from '../update-work-unit-estimate';
import { recordTokens } from '../record-tokens';
import { recordIteration } from '../record-iteration';
import { queryMetrics } from '../query-metrics';
import { queryEstimateAccuracy } from '../query-estimate-accuracy';
import { queryEstimationGuide } from '../query-estimation-guide';

describe('Feature: Work Unit Estimation and Metrics', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-work-unit-estimation-and-metrics');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Assign story points to work unit', () => {
    it('should set story point estimate', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'task',
            status: 'backlog',
          },
        },
        states: {
          backlog: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run update-work-unit with estimate
      const result = await updateWorkUnitEstimate({
        workUnitId: 'AUTH-001',
        estimate: 5,
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have estimate of 5 story points
      const content = await readFile(workUnitsFile, 'utf-8');
      const updatedData = JSON.parse(content);
      expect(updatedData.workUnits['AUTH-001'].estimate).toBe(5);
    });
  });

  describe('Scenario: Use Fibonacci sequence for estimates', () => {
    it('should accept Fibonacci numbers for estimates', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'task',
            status: 'backlog',
          },
        },
        states: {
          backlog: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run update-work-unit with Fibonacci estimate
      const result = await updateWorkUnitEstimate({
        workUnitId: 'AUTH-001',
        estimate: 8,
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the estimate should be valid Fibonacci number
      const content = await readFile(workUnitsFile, 'utf-8');
      const updatedData = JSON.parse(content);
      expect(updatedData.workUnits['AUTH-001'].estimate).toBe(8);
    });
  });

  describe('Scenario: Reject non-Fibonacci estimate', () => {
    it('should reject non-Fibonacci estimates', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'backlog',
          },
        },
        states: {
          backlog: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run update-work-unit with non-Fibonacci estimate
      // Then the command should fail
      await expect(
        updateWorkUnitEstimate({
          workUnitId: 'AUTH-001',
          estimate: 7,
          cwd: testDir,
        })
      ).rejects.toThrow('Invalid estimate');

      // And the error should suggest valid values
      await expect(
        updateWorkUnitEstimate({
          workUnitId: 'AUTH-001',
          estimate: 7,
          cwd: testDir,
        })
      ).rejects.toThrow('1,2,3,5,8,13,21');
    });
  });

  describe('Scenario: Record tokens consumed during implementation', () => {
    it('should track token consumption', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "implementing"
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'implementing',
            actualTokens: 0,
          },
        },
        states: {
          implementing: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run record-tokens with tokens
      const result = await recordTokens({
        workUnitId: 'AUTH-001',
        tokens: 45000,
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have actualTokens of 45000
      const content = await readFile(workUnitsFile, 'utf-8');
      const updatedData = JSON.parse(content);
      expect(updatedData.workUnits['AUTH-001'].actualTokens).toBe(45000);
    });
  });

  describe('Scenario: Increment iteration count', () => {
    it('should increment iteration counter', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with iterations 2
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'implementing',
            iterations: 2,
          },
        },
        states: {
          implementing: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run record-iteration
      const result = await recordIteration({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have iterations of 3
      const content = await readFile(workUnitsFile, 'utf-8');
      const updatedData = JSON.parse(content);
      expect(updatedData.workUnits['AUTH-001'].iterations).toBe(3);
    });
  });

  describe('Scenario: Calculate cycle time from state history', () => {
    it('should calculate time from todo to done', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" has stateHistory
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'done',
            stateHistory: [
              { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
              { state: 'specifying', timestamp: '2025-01-15T11:00:00Z' },
              { state: 'testing', timestamp: '2025-01-15T13:00:00Z' },
              { state: 'implementing', timestamp: '2025-01-15T14:00:00Z' },
              { state: 'validating', timestamp: '2025-01-15T17:00:00Z' },
              { state: 'done', timestamp: '2025-01-15T18:00:00Z' },
            ],
          },
        },
        states: {
          done: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run query metrics
      const result = await queryMetrics({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the output should show cycle time: "8 hours"
      expect(result.cycleTime).toBe('8 hours');

      // And the output should show time per state
      expect(result.timePerState).toBeDefined();
      expect(result.timePerState.backlog).toBe('1 hour');
      expect(result.timePerState.specifying).toBe('2 hours');
      expect(result.timePerState.testing).toBe('1 hour');
      expect(result.timePerState.implementing).toBe('3 hours');
      expect(result.timePerState.validating).toBe('1 hour');
    });
  });

  describe('Scenario: Compare estimate vs actual for completed work unit', () => {
    it('should compare estimated vs actual effort', async () => {
      // Given I have a project with spec directory
      // And a completed work unit "AUTH-001" has estimate and actuals
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'done',
            estimate: 5,
            actualTokens: 95000,
            iterations: 2,
          },
        },
        states: {
          done: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run query estimate-accuracy
      const result = await queryEstimateAccuracy({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the output should show estimated points
      expect(result.estimated).toBe('5 points');

      // And the output should show actual metrics
      expect(result.actual).toBe('95000 tokens, 2 iterations');

      // And the output should show comparison
      expect(result.comparison).toContain('Within expected range');
    });
  });

  describe('Scenario: Analyze estimate accuracy across all completed work', () => {
    it('should calculate accuracy metrics across all work', async () => {
      // Given I have a project with spec directory
      // And completed work units
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            status: 'done',
            estimate: 1,
            actualTokens: 22000,
            iterations: 1,
          },
          'AUTH-002': {
            id: 'AUTH-002',
            status: 'done',
            estimate: 1,
            actualTokens: 28000,
            iterations: 2,
          },
          'AUTH-003': {
            id: 'AUTH-003',
            status: 'done',
            estimate: 3,
            actualTokens: 70000,
            iterations: 2,
          },
          'AUTH-004': {
            id: 'AUTH-004',
            status: 'done',
            estimate: 3,
            actualTokens: 80000,
            iterations: 3,
          },
          'AUTH-005': {
            id: 'AUTH-005',
            status: 'done',
            estimate: 5,
            actualTokens: 95000,
            iterations: 2,
          },
        },
        states: {
          done: ['AUTH-001', 'AUTH-002', 'AUTH-003', 'AUTH-004', 'AUTH-005'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run query estimate-accuracy with output=json
      const result = await queryEstimateAccuracy({
        cwd: testDir,
        output: 'json',
      });

      // Then the output should show average tokens per story point
      expect(result.byStoryPoints).toBeDefined();
      expect(result.byStoryPoints['1']).toEqual({
        avgTokens: 25000,
        avgIterations: 1.5,
        samples: 2,
      });
      expect(result.byStoryPoints['3']).toEqual({
        avgTokens: 75000,
        avgIterations: 2.5,
        samples: 2,
      });
      expect(result.byStoryPoints['5']).toEqual({
        avgTokens: 95000,
        avgIterations: 2.0,
        samples: 1,
      });
    });
  });

  describe('Scenario: Analyze estimate accuracy by prefix', () => {
    it('should calculate accuracy per prefix', async () => {
      // Given I have a project with spec directory
      // And completed work units with different prefixes
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            status: 'done',
            estimate: 5,
            actualTokens: 95000,
          },
          'AUTH-002': {
            id: 'AUTH-002',
            status: 'done',
            estimate: 3,
            actualTokens: 70000,
          },
          'SEC-001': {
            id: 'SEC-001',
            status: 'done',
            estimate: 5,
            actualTokens: 140000,
          },
          'SEC-002': {
            id: 'SEC-002',
            status: 'done',
            estimate: 3,
            actualTokens: 95000,
          },
        },
        states: {
          done: ['AUTH-001', 'AUTH-002', 'SEC-001', 'SEC-002'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run query estimate-accuracy --by-prefix
      const result = await queryEstimateAccuracy({
        cwd: testDir,
        byPrefix: true,
        output: 'json',
      });

      // Then the output should show AUTH prefix stats
      expect(result.byPrefix['AUTH']).toBeDefined();
      expect(result.byPrefix['AUTH'].avgAccuracy).toContain('estimates');
      expect(result.byPrefix['AUTH'].recommendation).toContain(
        'well-calibrated'
      );

      // And the output should show SEC prefix stats
      expect(result.byPrefix['SEC']).toBeDefined();
      expect(result.byPrefix['SEC'].avgAccuracy).toContain('estimates');
      expect(result.byPrefix['SEC'].recommendation).toContain(
        'increase estimates'
      );
    });
  });

  describe('Scenario: Get estimation recommendations for new work', () => {
    it('should suggest estimates based on historical data', async () => {
      // Given I have a project with spec directory
      // And completed work units with established patterns
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const data = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            status: 'done',
            estimate: 1,
            actualTokens: 25000,
            iterations: 1,
          },
          'AUTH-002': {
            id: 'AUTH-002',
            status: 'done',
            estimate: 1,
            actualTokens: 22000,
            iterations: 2,
          },
          'AUTH-003': {
            id: 'AUTH-003',
            status: 'done',
            estimate: 3,
            actualTokens: 70000,
            iterations: 2,
          },
          'AUTH-004': {
            id: 'AUTH-004',
            status: 'done',
            estimate: 3,
            actualTokens: 85000,
            iterations: 3,
          },
          'AUTH-005': {
            id: 'AUTH-005',
            status: 'done',
            estimate: 5,
            actualTokens: 95000,
            iterations: 2,
          },
          'AUTH-006': {
            id: 'AUTH-006',
            status: 'done',
            estimate: 5,
            actualTokens: 120000,
            iterations: 4,
          },
        },
        states: {
          done: [
            'AUTH-001',
            'AUTH-002',
            'AUTH-003',
            'AUTH-004',
            'AUTH-005',
            'AUTH-006',
          ],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

      // When I run query estimation-guide
      const result = await queryEstimationGuide({
        cwd: testDir,
      });

      // Then the output should show recommended patterns
      expect(result.patterns).toBeDefined();
      expect(result.patterns).toEqual([
        {
          points: 1,
          expectedTokens: '22k-25k',
          expectedIterations: '1-2',
          confidence: 'medium',
        },
        {
          points: 3,
          expectedTokens: '70k-85k',
          expectedIterations: '2-3',
          confidence: 'medium',
        },
        {
          points: 5,
          expectedTokens: '95k-120k',
          expectedIterations: '2-4',
          confidence: 'medium',
        },
      ]);
    });
  });
});
