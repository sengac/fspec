import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { recordIteration } from '../record-iteration';
import { validateSpecAlignment } from '../validate-spec-alignment';
import { autoAdvance } from '../auto-advance';

describe('Feature: Workflow Automation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-workflow-automation');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Record iteration after tool use', () => {
    it('should increment iterations count on work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "implementing"
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const initialData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'implementing',
            iterations: 0,
            updatedAt: new Date('2025-01-01').toISOString(),
          },
        },
        states: {
          implementing: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // When I run "fspec record-iteration AUTH-001"
      const result = await recordIteration({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the iterations count should increment on "AUTH-001"
      expect(result.success).toBe(true);

      const updatedData = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updatedData.workUnits['AUTH-001'].iterations).toBe(1);

      // And the updatedAt timestamp should be updated
      expect(
        new Date(updatedData.workUnits['AUTH-001'].updatedAt).getTime()
      ).toBeGreaterThan(new Date('2025-01-01').getTime());
    });
  });

  describe('Scenario: Validate spec alignment before commit', () => {
    it('should warn when no scenarios exist for work unit', async () => {
      // Given I have a project with spec directory
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const initialData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'implementing',
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          implementing: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // And a work unit "AUTH-001" exists with status "implementing"
      // And no scenarios are tagged with "@AUTH-001"
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      await writeFile(
        join(testDir, 'spec', 'features', 'test.feature'),
        '@critical\nFeature: Test\n  Scenario: No tag\n    Given test'
      );

      // When I run "fspec validate-spec-alignment AUTH-001"
      const result = await validateSpecAlignment({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should warn "No scenarios for AUTH-001"
      expect(result.valid).toBe(false);
      expect(result.warnings).toBeDefined();
      expect(result.warnings![0]).toContain('No scenarios for AUTH-001');
    });
  });

  describe('Scenario: Auto-advance state after tests pass', () => {
    it('should transition from testing to implementing', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" has status "testing"
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const initialData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'testing',
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          testing: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // When I run "fspec auto-advance AUTH-001 --from=testing --event=tests-pass"
      const result = await autoAdvance({
        workUnitId: 'AUTH-001',
        from: 'testing',
        event: 'tests-pass',
        cwd: testDir,
      });

      // Then the work unit should transition to "implementing"
      expect(result.success).toBe(true);

      const updatedData = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updatedData.workUnits['AUTH-001'].status).toBe('implementing');
      expect(updatedData.states.implementing).toContain('AUTH-001');
      expect(updatedData.states.testing).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Auto-mark done after validation succeeds', () => {
    it('should transition to done and record completion timestamp', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" has status "validating"
      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      const initialData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'validating',
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          validating: ['AUTH-001'],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // When I run "fspec auto-advance AUTH-001 --from=validating --event=validation-pass"
      const result = await autoAdvance({
        workUnitId: 'AUTH-001',
        from: 'validating',
        event: 'validation-pass',
        cwd: testDir,
      });

      // Then the work unit should transition to "done"
      expect(result.success).toBe(true);

      const updatedData = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(updatedData.workUnits['AUTH-001'].status).toBe('done');

      // And completion timestamp should be recorded
      expect(updatedData.workUnits['AUTH-001'].completedAt).toBeDefined();
      expect(
        new Date(updatedData.workUnits['AUTH-001'].completedAt).getTime()
      ).toBeGreaterThan(0);
    });
  });
});
