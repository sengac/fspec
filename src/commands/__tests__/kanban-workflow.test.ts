import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnit } from '../work-unit';
import { prioritizeWorkUnit } from '../work-unit';
import { queryWorkUnit } from '../work-unit';
import { displayBoard } from '../work-unit';
import { validateWorkUnits, repairWorkUnits } from '../work-unit';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Kanban Workflow State Management', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('kanban-workflow');

    // Initialize work units file with states index
    await writeJsonTestFile(setup.workUnitsFile, {
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
    });
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Move work unit from backlog to specifying', () => {
    it('should update state and states index with history', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "backlog"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      const oldTimestamp = new Date('2024-01-01T00:00:00Z').toISOString();
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        stateHistory: [{ state: 'backlog', timestamp: oldTimestamp }],
        createdAt: oldTimestamp,
        updatedAt: oldTimestamp,
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=specifying"
      await updateWorkUnit(
        'AUTH-001',
        { status: 'specifying' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit "AUTH-001" status should be "specifying"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('specifying');

      // And the states.backlog array should not contain "AUTH-001"
      expect(updatedWorkUnits.states.backlog).not.toContain('AUTH-001');

      // And the states.specifying array should contain "AUTH-001"
      expect(updatedWorkUnits.states.specifying).toContain('AUTH-001');

      // And the stateHistory should include transition from "backlog" to "specifying"
      expect(updatedWorkUnits.workUnits['AUTH-001'].stateHistory).toHaveLength(
        2
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].stateHistory[1].state).toBe(
        'specifying'
      );

      // And the updatedAt timestamp should be updated
      expect(updatedWorkUnits.workUnits['AUTH-001'].updatedAt).not.toBe(
        oldTimestamp
      );
    });
  });

  describe('Scenario: Complete ACDD workflow from backlog to done', () => {
    it('should progress through all states with full history', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "backlog"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // Create a dummy feature file with scenario tagged @AUTH-001 (needed for testing state)
      await writeFile(
        join(setup.featuresDir, 'auth.feature'),
        `@auth\nFeature: Auth\n\n@AUTH-001\nScenario: Login\nGiven test\nWhen test\nThen test`
      );

      // When I run state transitions through ACDD workflow
      await updateWorkUnit(
        'AUTH-001',
        { status: 'specifying' },
        { cwd: setup.testDir }
      );
      await updateWorkUnit(
        'AUTH-001',
        { status: 'testing' },
        { cwd: setup.testDir }
      );
      await updateWorkUnit(
        'AUTH-001',
        { status: 'implementing' },
        { cwd: setup.testDir }
      );
      await updateWorkUnit(
        'AUTH-001',
        { status: 'validating' },
        { cwd: setup.testDir }
      );
      await updateWorkUnit(
        'AUTH-001',
        { status: 'done' },
        { cwd: setup.testDir }
      );

      // Then the work unit status should be "done"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('done');

      // And the states.done array should contain "AUTH-001"
      expect(updatedWorkUnits.states.done).toContain('AUTH-001');

      // And the stateHistory should have 6 entries (initial + 5 transitions)
      expect(updatedWorkUnits.workUnits['AUTH-001'].stateHistory).toHaveLength(
        6
      );

      // And the stateHistory should show ACDD progression
      const states = updatedWorkUnits.workUnits['AUTH-001'].stateHistory.map(
        (h: { state: string }) => h.state
      );
      expect(states).toEqual([
        'backlog',
        'specifying',
        'testing',
        'implementing',
        'validating',
        'done',
      ]);
    });
  });

  describe('Scenario: Attempt to skip specifying state (violates ACDD)', () => {
    it('should prevent direct backlog to testing transition', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "backlog"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=testing"
      // Then the command should fail
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'testing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('Invalid state transition');

      // And the error should explain ACDD requirement
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'testing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow("Must move to 'specifying' state first");

      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'testing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('ACDD requires specification before testing');
    });
  });

  describe('Scenario: Attempt to skip testing state (violates ACDD)', () => {
    it('should prevent direct specifying to implementing transition', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'specifying',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=implementing"
      // Then the command should fail
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'implementing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('Invalid state transition');

      // And the error should explain ACDD requirement
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'implementing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow("Must move to 'testing' state first");

      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'implementing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('ACDD requires tests before implementation');
    });
  });

  describe('Scenario: Attempt to move work back to backlog', () => {
    it('should prevent backward transition to backlog', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "implementing"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'implementing',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
          { state: 'testing', timestamp: new Date().toISOString() },
          { state: 'implementing', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.implementing.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=backlog"
      // Then the command should fail
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'backlog' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('Cannot move work back to backlog');

      // And the error should suggest using blocked state
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'backlog' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow("Use 'blocked' state if work cannot progress");
    });
  });

  describe('Scenario: Move work unit to blocked state from any state', () => {
    it('should allow blocking from any state with reason', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "implementing"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'implementing',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'implementing', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.implementing.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=blocked --blocked-reason='Waiting for API endpoint'"
      await updateWorkUnit(
        'AUTH-001',
        {
          status: 'blocked',
          blockedReason: 'Waiting for API endpoint',
        },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit status should be "blocked"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('blocked');

      // And the states.implementing array should not contain "AUTH-001"
      expect(updatedWorkUnits.states.implementing).not.toContain('AUTH-001');

      // And the states.blocked array should contain "AUTH-001"
      expect(updatedWorkUnits.states.blocked).toContain('AUTH-001');

      // And the work unit should have blockedReason
      expect(updatedWorkUnits.workUnits['AUTH-001'].blockedReason).toBe(
        'Waiting for API endpoint'
      );

      // And the stateHistory should record the blocked transition
      const lastState =
        updatedWorkUnits.workUnits['AUTH-001'].stateHistory[
          updatedWorkUnits.workUnits['AUTH-001'].stateHistory.length - 1
        ];
      expect(lastState.state).toBe('blocked');
    });
  });

  describe('Scenario: Unblock work unit and return to previous state', () => {
    it('should clear blocked reason and restore state', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" was blocked from specifying state
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'blocked',
        blockedReason: 'Waiting for clarification',
        stateHistory: [
          { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
          { state: 'specifying', timestamp: '2025-01-15T11:00:00Z' },
          { state: 'blocked', timestamp: '2025-01-15T12:00:00Z' },
        ],
        createdAt: '2025-01-15T10:00:00Z',
        updatedAt: '2025-01-15T12:00:00Z',
      };
      workUnits.states.blocked.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=specifying"
      await updateWorkUnit(
        'AUTH-001',
        { status: 'specifying' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit status should be "specifying"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('specifying');

      // And the work unit blockedReason should be cleared
      expect(
        updatedWorkUnits.workUnits['AUTH-001'].blockedReason
      ).toBeUndefined();

      // And the states.blocked array should not contain "AUTH-001"
      expect(updatedWorkUnits.states.blocked).not.toContain('AUTH-001');

      // And the states.specifying array should contain "AUTH-001"
      expect(updatedWorkUnits.states.specifying).toContain('AUTH-001');
    });
  });

  describe('Scenario: Require blocked reason when moving to blocked state', () => {
    it('should fail without blocked reason', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "implementing"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'implementing',
        stateHistory: [
          { state: 'implementing', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.implementing.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=blocked"
      // Then the command should fail
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'blocked' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('Blocked reason is required');

      // And the error should suggest using --blocked-reason
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'blocked' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow("Use --blocked-reason='description of blocker'");
    });
  });

  describe('Scenario: Validate Gherkin scenarios exist before moving to testing', () => {
    it('should require scenario tagged with work unit ID', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'specifying',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // And no scenarios are tagged with "@AUTH-001"
      // (empty features directory)

      // When I run "fspec update-work-unit AUTH-001 --status=testing"
      // Then the command should fail
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'testing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('No Gherkin scenarios found');

      // And the error should explain the requirement
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'testing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('At least one scenario must be tagged with @AUTH-001');

      // And the error should suggest solution
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { status: 'testing' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow(
        "Use 'fspec generate-scenarios AUTH-001' or manually tag scenarios"
      );
    });
  });

  describe('Scenario: Successfully move to testing when scenarios exist', () => {
    it('should allow transition when scenario is tagged', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'specifying',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // And a scenario is tagged with "@AUTH-001" in spec/features/authentication.feature
      await writeFile(
        join(setup.featuresDir, 'authentication.feature'),
        `@auth\nFeature: Authentication\n\n@AUTH-001\nScenario: OAuth login\nGiven test\nWhen test\nThen test`
      );

      // When I run "fspec update-work-unit AUTH-001 --status=testing"
      await updateWorkUnit(
        'AUTH-001',
        { status: 'testing' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit status should be "testing"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('testing');
    });
  });

  describe('Scenario: Validate estimate assigned before moving from specifying', () => {
    it('should warn about missing estimate but allow transition', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying" and no estimate
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'specifying',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'specifying', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // And a scenario is tagged with "@AUTH-001"
      await writeFile(
        join(setup.featuresDir, 'auth.feature'),
        `@auth\nFeature: Auth\n\n@AUTH-001\nScenario: Login\nGiven test\nWhen test\nThen test`
      );

      // When I run "fspec update-work-unit AUTH-001 --status=testing"
      const result = await updateWorkUnit(
        'AUTH-001',
        { status: 'testing' },
        { cwd: setup.testDir }
      );

      // Then the command should display warning (in result.warnings if returned)
      // But the transition should succeed
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('testing');
    });
  });

  describe('Scenario: Prevent parent from being marked done with incomplete children', () => {
    it('should validate all children are complete before marking parent done', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "validating"
      // And a work unit "AUTH-002" exists with parent "AUTH-001" and status "implementing"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'validating',
        children: ['AUTH-002'],
        stateHistory: [
          { state: 'validating', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Google provider',
        status: 'implementing',
        parent: 'AUTH-001',
        stateHistory: [
          { state: 'implementing', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.validating.push('AUTH-001');
      workUnits.states.implementing.push('AUTH-002');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=done"
      // Then the command should fail
      await expect(
        updateWorkUnit('AUTH-001', { status: 'done' }, { cwd: setup.testDir })
      ).rejects.toThrow('Cannot mark parent as done');

      // And the error should list incomplete child
      await expect(
        updateWorkUnit('AUTH-001', { status: 'done' }, { cwd: setup.testDir })
      ).rejects.toThrow('AUTH-002');

      await expect(
        updateWorkUnit('AUTH-001', { status: 'done' }, { cwd: setup.testDir })
      ).rejects.toThrow('implementing');

      // And the error should suggest completing children
      await expect(
        updateWorkUnit('AUTH-001', { status: 'done' }, { cwd: setup.testDir })
      ).rejects.toThrow('Complete all children first');
    });
  });

  describe('Scenario: Allow parent to be marked done when all children complete', () => {
    it('should allow parent done when all children done', async () => {
      // Given I have a project with spec directory
      // And parent and all children are ready
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'validating',
        children: ['AUTH-002', 'AUTH-003'],
        stateHistory: [
          { state: 'validating', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Google',
        status: 'done',
        parent: 'AUTH-001',
        stateHistory: [{ state: 'done', timestamp: new Date().toISOString() }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-003'] = {
        id: 'AUTH-003',
        title: 'GitHub',
        status: 'done',
        parent: 'AUTH-001',
        stateHistory: [{ state: 'done', timestamp: new Date().toISOString() }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.validating.push('AUTH-001');
      workUnits.states.done.push('AUTH-002', 'AUTH-003');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=done"
      await updateWorkUnit(
        'AUTH-001',
        { status: 'done' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit "AUTH-001" status should be "done"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('done');
    });
  });

  describe('Scenario: Reorder work unit to top of backlog (highest priority)', () => {
    it('should move work unit to first position', async () => {
      // Given I have a project with spec directory
      // And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.states.backlog = ['AUTH-001', 'AUTH-002', 'AUTH-003'];
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec prioritize AUTH-003 --position=top"
      await prioritizeWorkUnit(
        'AUTH-003',
        { position: 'top' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the states.backlog array should be reordered
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.states.backlog).toEqual([
        'AUTH-003',
        'AUTH-001',
        'AUTH-002',
      ]);
    });
  });

  describe('Scenario: Reorder work unit to bottom of backlog (lowest priority)', () => {
    it('should move work unit to last position', async () => {
      // Given I have a project with spec directory
      // And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.states.backlog = ['AUTH-001', 'AUTH-002', 'AUTH-003'];
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec prioritize AUTH-001 --position=bottom"
      await prioritizeWorkUnit(
        'AUTH-001',
        { position: 'bottom' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the states.backlog array should be reordered
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.states.backlog).toEqual([
        'AUTH-002',
        'AUTH-003',
        'AUTH-001',
      ]);
    });
  });

  describe('Scenario: Move work unit before another work unit', () => {
    it('should reposition work unit before target', async () => {
      // Given I have a project with spec directory
      // And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.states.backlog = ['AUTH-001', 'AUTH-002', 'AUTH-003'];
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec prioritize AUTH-003 --before=AUTH-002"
      await prioritizeWorkUnit(
        'AUTH-003',
        { before: 'AUTH-002' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the states.backlog array should be reordered
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.states.backlog).toEqual([
        'AUTH-001',
        'AUTH-003',
        'AUTH-002',
      ]);
    });
  });

  describe('Scenario: Move work unit after another work unit', () => {
    it('should reposition work unit after target', async () => {
      // Given I have a project with spec directory
      // And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.states.backlog = ['AUTH-001', 'AUTH-002', 'AUTH-003'];
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec prioritize AUTH-001 --after=AUTH-003"
      await prioritizeWorkUnit(
        'AUTH-001',
        { after: 'AUTH-003' },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the states.backlog array should be reordered
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.states.backlog).toEqual([
        'AUTH-002',
        'AUTH-003',
        'AUTH-001',
      ]);
    });
  });

  describe('Scenario: Set work unit to specific position in backlog', () => {
    it('should move work unit to exact position', async () => {
      // Given I have a project with spec directory
      // And the states.backlog array contains in order: "AUTH-001", "AUTH-002", "AUTH-003", "AUTH-004"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.states.backlog = [
        'AUTH-001',
        'AUTH-002',
        'AUTH-003',
        'AUTH-004',
      ];
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec prioritize AUTH-004 --position=1"
      await prioritizeWorkUnit(
        'AUTH-004',
        { position: 1 },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the states.backlog array should be reordered
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.states.backlog).toEqual([
        'AUTH-001',
        'AUTH-004',
        'AUTH-002',
        'AUTH-003',
      ]);
    });
  });

  describe('Scenario: Attempt to prioritize work not in backlog', () => {
    it('should fail for work units not in backlog state', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "implementing"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'implementing',
        stateHistory: [
          { state: 'implementing', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.implementing.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec prioritize AUTH-001 --position=top"
      // Then the command should fail
      await expect(
        prioritizeWorkUnit(
          'AUTH-001',
          { position: 'top' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow('Can only prioritize work units in backlog state');

      await expect(
        prioritizeWorkUnit(
          'AUTH-001',
          { position: 'top' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow("AUTH-001 is in 'implementing' state");
    });
  });

  describe('Scenario: Attempt to position work unit before non-existent work unit', () => {
    it('should fail with not found error', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "backlog"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'backlog',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // And no work unit "AUTH-999" exists

      // When I run "fspec prioritize AUTH-001 --before=AUTH-999"
      // Then the command should fail
      await expect(
        prioritizeWorkUnit(
          'AUTH-001',
          { before: 'AUTH-999' },
          { cwd: setup.testDir }
        )
      ).rejects.toThrow("Work unit 'AUTH-999' does not exist");
    });
  });

  describe('Scenario: Track complete state history with timestamps', () => {
    it('should record every state transition with timestamp and optional reason', async () => {
      // Given I have a project with spec directory
      // And state transitions occur at specific times
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'backlog',
        stateHistory: [{ state: 'backlog', timestamp: '2025-01-15T10:00:00Z' }],
        createdAt: '2025-01-15T10:00:00Z',
        updatedAt: '2025-01-15T10:00:00Z',
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // Simulate multiple transitions (in real implementation, these would respect timestamps)
      await updateWorkUnit(
        'AUTH-001',
        { status: 'specifying' },
        { cwd: setup.testDir }
      );

      let updated = JSON.parse(await readFile(setup.workUnitsFile, 'utf-8'));
      updated.workUnits['AUTH-001'].stateHistory[1].timestamp =
        '2025-01-15T11:00:00Z';
      await writeFile(setup.workUnitsFile, JSON.stringify(updated, null, 2));

      await updateWorkUnit(
        'AUTH-001',
        {
          status: 'blocked',
          blockedReason: 'Question',
        },
        { cwd: setup.testDir }
      );

      updated = JSON.parse(await readFile(setup.workUnitsFile, 'utf-8'));
      updated.workUnits['AUTH-001'].stateHistory[2].timestamp =
        '2025-01-15T12:00:00Z';
      await writeFile(setup.workUnitsFile, JSON.stringify(updated, null, 2));

      await updateWorkUnit(
        'AUTH-001',
        { status: 'specifying' },
        { cwd: setup.testDir }
      );

      updated = JSON.parse(await readFile(setup.workUnitsFile, 'utf-8'));
      updated.workUnits['AUTH-001'].stateHistory[3].timestamp =
        '2025-01-15T14:00:00Z';
      await writeFile(setup.workUnitsFile, JSON.stringify(updated, null, 2));

      // Create scenario for testing transition
      await writeFile(
        join(setup.featuresDir, 'auth.feature'),
        `@auth\nFeature: Auth\n\n@AUTH-001\nScenario: Login\nGiven test\nWhen test\nThen test`
      );

      await updateWorkUnit(
        'AUTH-001',
        { status: 'testing' },
        { cwd: setup.testDir }
      );

      updated = JSON.parse(await readFile(setup.workUnitsFile, 'utf-8'));
      updated.workUnits['AUTH-001'].stateHistory[4].timestamp =
        '2025-01-15T15:00:00Z';
      await writeFile(setup.workUnitsFile, JSON.stringify(updated, null, 2));

      // Then the stateHistory should have 5 entries
      const finalWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(finalWorkUnits.workUnits['AUTH-001'].stateHistory).toHaveLength(5);

      // And verify the progression
      const history = finalWorkUnits.workUnits['AUTH-001'].stateHistory;
      expect(history[0].state).toBe('backlog');
      expect(history[1].state).toBe('specifying');
      expect(history[2].state).toBe('blocked');
      expect(history[3].state).toBe('specifying');
      expect(history[4].state).toBe('testing');
    });
  });

  describe('Scenario: Calculate time spent in each state from history', () => {
    it('should compute cycle time from state history', async () => {
      // Given I have a project with spec directory
      // And a work unit with complete state history
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'done',
        stateHistory: [
          { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
          { state: 'specifying', timestamp: '2025-01-15T11:00:00Z' },
          { state: 'testing', timestamp: '2025-01-15T13:00:00Z' },
          { state: 'implementing', timestamp: '2025-01-15T14:00:00Z' },
          { state: 'validating', timestamp: '2025-01-15T17:00:00Z' },
          { state: 'done', timestamp: '2025-01-15T18:00:00Z' },
        ],
        createdAt: '2025-01-15T10:00:00Z',
        updatedAt: '2025-01-15T18:00:00Z',
      };
      workUnits.states.done.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec query work-unit AUTH-001 --show-cycle-time"
      const result = await queryWorkUnit('AUTH-001', {
        showCycleTime: true,
        cwd: setup.testDir,
      });

      // Then the output should show duration for each state
      expect(result).toContain('backlog');
      expect(result).toContain('1 hour');
      expect(result).toContain('specifying');
      expect(result).toContain('2 hours');
      expect(result).toContain('testing');
      expect(result).toContain('implementing');
      expect(result).toContain('3 hours');
      expect(result).toContain('validating');

      // And the total cycle time should be calculated
      expect(result).toContain('8 hours');
    });
  });

  describe('Scenario: Allow validation to move back to implementing on test failure', () => {
    it('should allow backward transition from validating to implementing with reason', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "validating"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'validating',
        stateHistory: [
          { state: 'validating', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.validating.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=implementing --reason='Test failures'"
      await updateWorkUnit(
        'AUTH-001',
        {
          status: 'implementing',
          reason: 'Test failures',
        },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit status should be "implementing"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe(
        'implementing'
      );

      // And the stateHistory should record the reason
      const lastEntry =
        updatedWorkUnits.workUnits['AUTH-001'].stateHistory[
          updatedWorkUnits.workUnits['AUTH-001'].stateHistory.length - 1
        ];
      expect(lastEntry.reason).toBe('Test failures');
    });
  });

  describe('Scenario: Allow validation to move back to specifying on spec error', () => {
    it('should allow backward transition from validating to specifying', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "validating"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'validating',
        stateHistory: [
          { state: 'validating', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.validating.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=specifying --reason='Acceptance criteria incomplete'"
      await updateWorkUnit(
        'AUTH-001',
        {
          status: 'specifying',
          reason: 'Acceptance criteria incomplete',
        },
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      // And the work unit status should be "specifying"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('specifying');
    });
  });

  describe('Scenario: Allow moving from done to fix mistakes (ACDD backward movement)', () => {
    it('should allow status changes on completed work when mistakes discovered', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "done"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'done',
        stateHistory: [
          { state: 'backlog', timestamp: new Date().toISOString() },
          { state: 'done', timestamp: new Date().toISOString() },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.done.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec update-work-unit AUTH-001 --status=implementing"
      // Then the command should succeed (backward movement allowed)
      await updateWorkUnit(
        'AUTH-001',
        { status: 'implementing' },
        { cwd: setup.testDir }
      );

      // And the work unit status should be "implementing"
      const updatedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe(
        'implementing'
      );

      // And the state history should include the backward transition
      expect(
        updatedWorkUnits.workUnits['AUTH-001'].stateHistory.length
      ).toBeGreaterThan(2);
    });
  });

  describe('Scenario: Query work units by current state', () => {
    it('should filter and return work units in specified state', async () => {
      // Given I have a project with spec directory
      // And work units exist in various states
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        title: 'Auth 1',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'specifying',
        title: 'Auth 2',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-003'] = {
        id: 'AUTH-003',
        status: 'implementing',
        title: 'Auth 3',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        status: 'implementing',
        title: 'Dash 1',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'done',
        title: 'API 1',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      workUnits.states.specifying.push('AUTH-002');
      workUnits.states.implementing.push('AUTH-003', 'DASH-001');
      workUnits.states.done.push('API-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec query work-units --status=implementing --output=json"
      const result = await queryWorkUnit(null, {
        status: 'implementing',
        output: 'json',
        cwd: setup.testDir,
      });

      // Then the output should be valid JSON
      const json = JSON.parse(result);

      // And the JSON should contain 2 work units
      expect(json).toHaveLength(2);

      // And the JSON should include work units in implementing state
      const ids = json.map((wu: { id: string }) => wu.id);
      expect(ids).toContain('AUTH-003');
      expect(ids).toContain('DASH-001');
    });
  });

  describe('Scenario: Display Kanban board showing all states', () => {
    it('should render board with columns for each state', async () => {
      // Given I have a project with spec directory
      // And work units exist across all states
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        title: 'Auth 1',
        estimate: 5,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'specifying',
        title: 'Auth 2',
        estimate: 8,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-003'] = {
        id: 'AUTH-003',
        status: 'testing',
        title: 'Auth 3',
        estimate: 3,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        status: 'implementing',
        title: 'Dash',
        estimate: 5,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        status: 'validating',
        title: 'API',
        estimate: 5,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['SEC-001'] = {
        id: 'SEC-001',
        status: 'done',
        title: 'Security',
        estimate: 3,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      workUnits.states.specifying.push('AUTH-002');
      workUnits.states.testing.push('AUTH-003');
      workUnits.states.implementing.push('DASH-001');
      workUnits.states.validating.push('API-001');
      workUnits.states.done.push('SEC-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec board"
      const output = await displayBoard({ cwd: setup.testDir });

      // Then the output should display columns for all states
      expect(output).toContain('backlog');
      expect(output).toContain('specifying');
      expect(output).toContain('testing');
      expect(output).toContain('implementing');
      expect(output).toContain('validating');
      expect(output).toContain('done');

      // And each column should show work units with estimates
      expect(output).toContain('AUTH-001');
      expect(output).toContain('5 pts');
      expect(output).toContain('AUTH-002');
      expect(output).toContain('8 pts');
      expect(output).toContain('AUTH-003');
      expect(output).toContain('3 pts');
      expect(output).toContain('DASH-001');
      expect(output).toContain('API-001');
      expect(output).toContain('SEC-001');

      // And the summary should show points breakdown
      expect(output).toContain('26 points in progress');
      expect(output).toContain('3 points completed');
    });
  });

  describe('Scenario: Validate work unit state is one of allowed values', () => {
    it('should reject invalid status values', async () => {
      // Given I have a project with spec directory
      // And I manually set an invalid status
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'invalid-state', // Invalid!
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec validate-work-units"
      const result = await validateWorkUnits({ cwd: setup.testDir });

      // Then the command should fail
      expect(result.valid).toBe(false);

      // And the error should contain validation details
      expect(result.errors).toContain('Invalid status value');
      expect(result.errors).toContain('backlog');
      expect(result.errors).toContain('specifying');
      expect(result.errors).toContain('testing');
      expect(result.errors).toContain('implementing');
      expect(result.errors).toContain('validating');
      expect(result.errors).toContain('done');
      expect(result.errors).toContain('blocked');
    });
  });

  describe('Scenario: Detect work unit in wrong state array', () => {
    it('should identify state index inconsistencies', async () => {
      // Given I have a project with spec directory
      // And work unit has status "implementing" but is in wrong array
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'implementing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001'); // Wrong array!
      // Note: states.implementing should contain AUTH-001 but doesn't
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec validate-work-units"
      const result = await validateWorkUnits({ cwd: setup.testDir });

      // Then the command should fail
      expect(result.valid).toBe(false);

      // And the error should explain the inconsistency
      expect(result.errors).toContain('State consistency error');
      expect(result.errors).toContain(
        "AUTH-001 has status 'implementing' but is in 'backlog' array"
      );

      // And the error should suggest repair
      expect(result.errors).toContain(
        "Run 'fspec repair-work-units' to fix inconsistencies"
      );
    });
  });

  describe('Scenario: Repair work units state index inconsistencies', () => {
    it('should move work units to correct state arrays', async () => {
      // Given I have a project with spec directory
      // And work unit is in wrong state array
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'implementing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001'); // Wrong!
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec repair-work-units"
      const result = await repairWorkUnits({ cwd: setup.testDir });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the command should report the fix
      expect(result.message).toContain(
        'Moved AUTH-001 from backlog to implementing'
      );

      // And the states should be corrected
      const repairedWorkUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      expect(repairedWorkUnits.states.implementing).toContain('AUTH-001');
      expect(repairedWorkUnits.states.backlog).not.toContain('AUTH-001');
    });
  });
});
