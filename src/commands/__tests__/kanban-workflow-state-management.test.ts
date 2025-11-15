import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../../types';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';

// Import commands (to be created/extended)
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { prioritizeWorkUnit } from '../prioritize-work-unit';
import { displayBoard } from '../display-board';
import { repairWorkUnits } from '../repair-work-units';
import { queryWorkUnits } from '../query-work-units';

describe('Feature: Kanban Workflow State Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-kanban-workflow-state-management');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec/features'), { recursive: true });

    // Create foundation.json for all tests (required by commands)
    await createMinimalFoundation(testDir);
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Move work unit from backlog to specifying', () => {
    it('should transition from backlog to specifying', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "backlog"
      const initialTime = '2025-01-15T10:00:00.000Z';
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'backlog',
            createdAt: initialTime,
            updatedAt: initialTime,
            stateHistory: [{ state: 'backlog', timestamp: initialTime }],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec update-work-unit AUTH-001 --status=specifying"
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      // And the work unit "AUTH-001" status should be "specifying"
      expect(updated.workUnits['AUTH-001'].status).toBe('specifying');

      // And the states.backlog array should not contain "AUTH-001"
      expect(updated.states.backlog).not.toContain('AUTH-001');

      // And the states.specifying array should contain "AUTH-001"
      expect(updated.states.specifying).toContain('AUTH-001');

      // And the stateHistory should include transition from "backlog" to "specifying"
      expect(updated.workUnits['AUTH-001'].stateHistory).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].stateHistory[1].state).toBe(
        'specifying'
      );

      // And the updatedAt timestamp should be updated
      expect(updated.workUnits['AUTH-001'].updatedAt).not.toBe(initialTime);
    });
  });

  describe('Scenario: Complete ACDD workflow from backlog to done', () => {
    it('should complete full ACDD workflow', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "backlog"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            type: 'story',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'backlog', timestamp: new Date().toISOString() },
            ],
            rules: [
              'Complete ACDD workflow must transition through all states',
            ],
            examples: [
              'OAuth feature progresses from backlog to done through all ACDD states',
            ],
            architectureNotes: [
              'Implementation: State machine enforces sequential workflow progression',
            ],
            attachments: [
              {
                path: 'spec/attachments/AUTH-001/ast-research.json',
                description:
                  'ACDD workflow state transitions and validation logic',
                addedAt: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // Create a feature file with tagged scenario
      const featureContent = `Feature: Auth
@AUTH-001
Scenario: Login
  Given user exists
  When user logs in
  Then session is created`;
      await writeFile(
        join(testDir, 'spec/features/auth.feature'),
        featureContent
      );

      // When I run the full workflow
      // Note: skipTemporalValidation is used here because the feature file
      // was created before entering specifying state (test setup constraint)
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        skipTemporalValidation: true,
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        skipTemporalValidation: true,
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'implementing',
        skipTemporalValidation: true,
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'validating',
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      });

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      // Then the work unit status should be "done"
      expect(updated.workUnits['AUTH-001'].status).toBe('done');

      // And the states.done array should contain "AUTH-001"
      expect(updated.states.done).toContain('AUTH-001');

      // And the stateHistory should have 6 entries
      expect(updated.workUnits['AUTH-001'].stateHistory).toHaveLength(6);

      // And the stateHistory should show progression
      const states = updated.workUnits['AUTH-001'].stateHistory.map(
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

  describe('Scenario: Try to move from backlog to testing - blocked (must go through specifying)', () => {
    it('should prevent skipping specifying state', async () => {
      // Given a work unit in backlog
      const workUnits: WorkUnitsData = {
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'backlog', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: ['WORK-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I try to jump to testing
      const error = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'testing',
        cwd: testDir,
      }).catch((e: Error) => e);

      // Then the command should fail with an error message explaining that specifying is required first
      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Invalid state transition');
        expect(error.message).toContain(
          "Must move to 'specifying' state first"
        );
        expect(error.message).toContain(
          'ACDD requires specification before testing'
        );
      }
    });
  });

  describe('Scenario: Attempt to skip specifying state (violates ACDD)', () => {
    it('should prevent skipping specifying state', async () => {
      // Given a work unit in backlog
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'backlog', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I try to jump to testing
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'AUTH-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow('Invalid state transition');

      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain(
          "Must move to 'specifying' state first"
        );
        expect(error.message).toContain(
          'ACDD requires specification before testing'
        );
      }
    });
  });

  describe('Scenario: Attempt to skip testing state (violates ACDD)', () => {
    it('should prevent skipping testing state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            type: 'story',
            status: 'specifying',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'backlog', timestamp: new Date().toISOString() },
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
            rules: ['OAuth flow must support industry-standard providers'],
            examples: ['User authenticates via Google OAuth provider'],
            architectureNotes: [
              'Implementation: Use OAuth 2.0 libraries for secure authentication',
            ],
            attachments: [
              {
                path: 'spec/attachments/AUTH-001/ast-research.json',
                description:
                  'OAuth library API research and integration patterns',
                addedAt: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'implementing',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Invalid state transition');
        expect(error.message).toContain("Must move to 'testing' state first");
        expect(error.message).toContain(
          'ACDD requires tests before implementation'
        );
      }
    });
  });

  describe('Scenario: Attempt to move work back to backlog', () => {
    it('should prevent moving back to backlog', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'implementing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'backlog',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Cannot move work back to backlog');
        expect(error.message).toContain(
          "Use 'blocked' state if work cannot progress"
        );
      }
    });
  });

  describe('Scenario: Move work unit to blocked state from any state', () => {
    it('should allow blocked state from any state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'implementing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'blocked',
        blockedReason: 'Waiting for API endpoint',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      expect(updated.workUnits['AUTH-001'].status).toBe('blocked');
      expect(updated.states.implementing).not.toContain('AUTH-001');
      expect(updated.states.blocked).toContain('AUTH-001');
      expect(updated.workUnits['AUTH-001'].blockedReason).toBe(
        'Waiting for API endpoint'
      );
      expect(updated.workUnits['AUTH-001'].stateHistory[1].state).toBe(
        'blocked'
      );
    });
  });

  describe('Scenario: Unblock work unit and return to previous state', () => {
    it('should return to previous state when unblocked', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'blocked',
            blockedReason: 'Waiting',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
              { state: 'specifying', timestamp: '2025-01-15T11:00:00Z' },
              { state: 'blocked', timestamp: '2025-01-15T12:00:00Z' },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: ['AUTH-001'],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      expect(updated.workUnits['AUTH-001'].status).toBe('specifying');
      expect(updated.workUnits['AUTH-001'].blockedReason).toBeUndefined();
      expect(updated.states.blocked).not.toContain('AUTH-001');
      expect(updated.states.specifying).toContain('AUTH-001');
    });
  });

  describe('Scenario: Require blocked reason when moving to blocked state', () => {
    it('should require blockedReason field', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'implementing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'blocked',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Blocked reason is required');
        expect(error.message).toContain(
          "Use --blocked-reason='description of blocker'"
        );
      }
    });
  });

  describe('Scenario: Validate Gherkin scenarios exist before moving to testing', () => {
    it('should require scenarios before testing state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            type: 'story',
            status: 'specifying',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
            rules: ['Gherkin scenarios must be tagged with work unit ID'],
            examples: [
              'Feature file has @AUTH-001 tag on at least one scenario',
            ],
            architectureNotes: [
              'Implementation: Validate scenario tags before allowing testing transition',
            ],
            attachments: [
              {
                path: 'spec/attachments/AUTH-001/ast-research.json',
                description:
                  'Scenario tag validation and Gherkin parsing logic',
                addedAt: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // No feature files with @AUTH-001 tag
      await writeFile(
        join(testDir, 'spec/features/auth.feature'),
        'Feature: Auth\nScenario: Test\n  Given x'
      );

      await expect(
        updateWorkUnitStatus({
          workUnitId: 'AUTH-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow('No Gherkin scenarios found');

      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      }).catch((e: Error) => e);

      if (error instanceof Error) {
        expect(error.message).toContain(
          'At least one scenario must be tagged with @AUTH-001'
        );
        expect(error.message).toContain(
          "Use 'fspec generate-scenarios AUTH-001'"
        );
      }
    });
  });

  describe('Scenario: Successfully move to testing when scenarios exist', () => {
    it('should allow testing when scenarios exist', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            type: 'story',
            status: 'specifying',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
            rules: ['User authentication must create a valid session'],
            examples: [
              'User logs in with correct credentials and session is established',
            ],
            architectureNotes: [
              'Implementation: Session management with secure token generation',
            ],
            attachments: [
              {
                path: 'spec/attachments/AUTH-001/ast-research.json',
                description:
                  'Session token generation and security best practices',
                addedAt: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // Create feature file with @AUTH-001 tag
      const featureContent = `Feature: Authentication
@AUTH-001
Scenario: User logs in
  Given user exists
  When user logs in
  Then session is created`;
      await writeFile(
        join(testDir, 'spec/features/authentication.feature'),
        featureContent
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        skipTemporalValidation: true,
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['AUTH-001'].status).toBe('testing');
    });
  });

  describe('Scenario: Validate estimate assigned before moving from specifying', () => {
    it('should warn if no estimate when leaving specifying', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'specifying', timestamp: new Date().toISOString() },
            ],
            rules: ['OAuth flow must support industry-standard providers'],
            examples: ['User authenticates via Google OAuth provider'],
            architectureNotes: [
              'Implementation: Use OAuth 2.0 libraries for secure authentication',
            ],
            attachments: [
              {
                path: 'spec/attachments/AUTH-001/ast-research.json',
                description:
                  'OAuth library API research and integration patterns',
                addedAt: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // Create feature with scenario
      const featureContent =
        '@AUTH-001\nFeature: Auth\nScenario: Login\n  Given x';
      await writeFile(
        join(testDir, 'spec/features/auth.feature'),
        featureContent
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        skipTemporalValidation: true,
        cwd: testDir,
      });

      // Should succeed but with warning
      expect(result.success).toBe(true);
      expect(result.warnings).toBeDefined();
      expect(
        result.warnings?.some(w => w.includes('No estimate assigned'))
      ).toBe(true);
    });
  });

  describe('Scenario: Prevent parent from being marked done with incomplete children', () => {
    it('should prevent parent done when children incomplete', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Parent',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'validating', timestamp: new Date().toISOString() },
            ],
            children: ['AUTH-002'],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Child',
            status: 'implementing',
            parent: 'AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'implementing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-002'],
          validating: ['AUTH-001'],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Cannot mark parent as done');
        expect(error.message).toContain('AUTH-002');
        expect(error.message).toContain('implementing');
        expect(error.message).toContain('Complete all children first');
      }
    });
  });

  describe('Scenario: Allow parent to be marked done when all children complete', () => {
    it('should allow parent done when all children done', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Parent',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'validating', timestamp: new Date().toISOString() },
            ],
            children: ['AUTH-002', 'AUTH-003'],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Child 1',
            status: 'done',
            parent: 'AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'done', timestamp: new Date().toISOString() },
            ],
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Child 2',
            status: 'done',
            parent: 'AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'done', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['AUTH-001'],
          done: ['AUTH-002', 'AUTH-003'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['AUTH-001'].status).toBe('done');
    });
  });

  describe('Scenario: Reorder work unit to top of backlog (highest priority)', () => {
    it('should move work unit to top of backlog', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-003',
        position: 'top',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.states.backlog).toEqual([
        'AUTH-003',
        'AUTH-001',
        'AUTH-002',
      ]);
    });
  });

  describe('Scenario: Reorder work unit to bottom of backlog (lowest priority)', () => {
    it('should move work unit to bottom of backlog', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        position: 'bottom',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.states.backlog).toEqual([
        'AUTH-002',
        'AUTH-003',
        'AUTH-001',
      ]);
    });
  });

  describe('Scenario: Move work unit before another work unit', () => {
    it('should insert work unit before target', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-003',
        before: 'AUTH-002',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.states.backlog).toEqual([
        'AUTH-001',
        'AUTH-003',
        'AUTH-002',
      ]);
    });
  });

  describe('Scenario: Move work unit after another work unit', () => {
    it('should insert work unit after target', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        after: 'AUTH-003',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.states.backlog).toEqual([
        'AUTH-002',
        'AUTH-003',
        'AUTH-001',
      ]);
    });
  });

  describe('Scenario: Set work unit to specific position in backlog', () => {
    it('should set work unit to specific position', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-004': {
            id: 'AUTH-004',
            title: 'Work 4',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003', 'AUTH-004'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await prioritizeWorkUnit({
        workUnitId: 'AUTH-004',
        position: 2, // 1-based: position 2 = second item (index 1)
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.states.backlog).toEqual([
        'AUTH-001',
        'AUTH-004',
        'AUTH-002',
        'AUTH-003',
      ]);
    });
  });

  describe('Scenario: Attempt to position work unit before non-existent work unit', () => {
    it('should fail when target does not exist', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const error = await prioritizeWorkUnit({
        workUnitId: 'AUTH-001',
        before: 'AUTH-999',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain("Work unit 'AUTH-999' does not exist");
      }
    });
  });

  describe('Scenario: Track complete state history with timestamps', () => {
    it('should record all state transitions with timestamps', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'backlog',
            createdAt: '2025-01-15T10:00:00Z',
            updatedAt: '2025-01-15T10:00:00Z',
            stateHistory: [
              { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
            ],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'blocked',
        blockedReason: 'Question',
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        cwd: testDir,
      });

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      expect(updated.workUnits['AUTH-001'].stateHistory).toHaveLength(4);
      expect(updated.workUnits['AUTH-001'].stateHistory[0].state).toBe(
        'backlog'
      );
      expect(updated.workUnits['AUTH-001'].stateHistory[1].state).toBe(
        'specifying'
      );
      expect(updated.workUnits['AUTH-001'].stateHistory[2].state).toBe(
        'blocked'
      );
      expect(updated.workUnits['AUTH-001'].stateHistory[2].reason).toBe(
        'Question'
      );
      expect(updated.workUnits['AUTH-001'].stateHistory[3].state).toBe(
        'specifying'
      );
    });
  });

  describe('Scenario: Calculate time spent in each state from history', () => {
    it('should calculate duration in each state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
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
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await queryWorkUnits({
        workUnitId: 'AUTH-001',
        showCycleTime: true,
        cwd: testDir,
      });

      expect(result.stateTimings).toBeDefined();
      expect(result.stateTimings.backlog).toBe('1 hour');
      expect(result.stateTimings.specifying).toBe('2 hours');
      expect(result.stateTimings.testing).toBe('1 hour');
      expect(result.stateTimings.implementing).toBe('3 hours');
      expect(result.stateTimings.validating).toBe('1 hour');
      expect(result.totalCycleTime).toBe('8 hours');
    });
  });

  describe('Scenario: Allow validation to move back to implementing on test failure', () => {
    it('should allow backward transition to implementing', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'validating', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['AUTH-001'],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'implementing',
        reason: 'Test failures',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['AUTH-001'].status).toBe('implementing');
      expect(updated.workUnits['AUTH-001'].stateHistory[1].reason).toBe(
        'Test failures'
      );
    });
  });

  describe('Scenario: Allow validation to move back to specifying on spec error', () => {
    it('should allow backward transition to specifying', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'validating', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['AUTH-001'],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'specifying',
        reason: 'Acceptance criteria incomplete',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['AUTH-001'].status).toBe('specifying');
    });
  });

  describe('Scenario: Move from testing to specifying when tests revealed incomplete acceptance criteria', () => {
    it('should allow backward transition from testing to specifying', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Work',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'testing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['WORK-001'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'specifying',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['WORK-001'].status).toBe('specifying');
      expect(updated.states.testing).not.toContain('WORK-001');
      expect(updated.states.specifying).toContain('WORK-001');
    });
  });

  describe('Scenario: Move from implementing to testing when test cases need refactoring', () => {
    it('should allow backward transition from implementing to testing', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'implementing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['WORK-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'testing',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['WORK-001'].status).toBe('testing');
      expect(updated.states.implementing).not.toContain('WORK-001');
      expect(updated.states.testing).toContain('WORK-001');
    });
  });

  describe('Scenario: Move from implementing to specifying when requirements misunderstood', () => {
    it('should allow backward transition from implementing to specifying', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            title: 'Work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'implementing', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['WORK-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await updateWorkUnitStatus({
        workUnitId: 'WORK-001',
        status: 'specifying',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.workUnits['WORK-001'].status).toBe('specifying');
      expect(updated.states.implementing).not.toContain('WORK-001');
      expect(updated.states.specifying).toContain('WORK-001');
    });
  });

  describe('Scenario: Allow moving from done to fix mistakes (ACDD backward movement)', () => {
    it('should allow status changes on completed work when mistakes discovered', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "done"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              { state: 'backlog', timestamp: '2025-01-15T10:00:00Z' },
              { state: 'specifying', timestamp: '2025-01-15T11:00:00Z' },
              { state: 'done', timestamp: new Date().toISOString() },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['AUTH-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec update-work-unit AUTH-001 --status=implementing"
      // Then the command should succeed (backward movement allowed)
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'implementing',
        cwd: testDir,
      });

      // And the work unit status should be "implementing"
      const updatedWorkUnits = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe(
        'implementing'
      );

      // And the state history should include the backward transition
      expect(
        updatedWorkUnits.workUnits['AUTH-001'].stateHistory.length
      ).toBeGreaterThan(3);
    });
  });

  describe('Scenario: Query work units by current state', () => {
    it('should filter work units by state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Work 4',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'Work 5',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: ['AUTH-002'],
          testing: [],
          implementing: ['AUTH-003', 'DASH-001'],
          validating: [],
          done: ['API-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await queryWorkUnits({
        status: 'implementing',
        output: 'json',
        cwd: testDir,
      });

      expect(result.workUnits).toHaveLength(2);
      expect(
        result.workUnits.some((wu: { id: string }) => wu.id === 'AUTH-003')
      ).toBe(true);
      expect(
        result.workUnits.some((wu: { id: string }) => wu.id === 'DASH-001')
      ).toBe(true);
    });
  });

  describe('Scenario: Display Kanban board showing all states', () => {
    it('should display work units grouped by state', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'specifying',
            estimate: 8,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Work 3',
            status: 'testing',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Work 4',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'Work 5',
            status: 'validating',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'SEC-001': {
            id: 'SEC-001',
            title: 'Work 6',
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
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await displayBoard({ cwd: testDir });

      expect(result.board).toBeDefined();
      expect(result.board.backlog).toContain('AUTH-001');
      expect(result.board.specifying).toContain('AUTH-002');
      expect(result.board.testing).toContain('AUTH-003');
      expect(result.board.implementing).toContain('DASH-001');
      expect(result.board.validating).toContain('API-001');
      expect(result.board.done).toContain('SEC-001');
      expect(result.summary).toContain('26 points in progress');
      expect(result.summary).toContain('3 points completed');
    });
  });

  describe('Scenario: Validate work unit state is one of allowed values', () => {
    it('should validate state against allowed values', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'invalid-state' as 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const { validateWorkUnits } = await import('../validate-work-units');
      const result = await validateWorkUnits({ cwd: testDir });

      expect(result.valid).toBe(false);
      expect(result.errors).toBeDefined();
      if (result.errors) {
        const errorStr = result.errors.join(' ');
        expect(errorStr).toContain('Invalid status value');
      }
    });
  });

  describe('Scenario: Detect work unit in wrong state array', () => {
    it('should detect state/index mismatches', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const { validateWorkUnits } = await import('../validate-work-units');
      const result = await validateWorkUnits({ cwd: testDir });

      expect(result.valid).toBe(false);
      expect(result.errors).toBeDefined();
      if (result.errors) {
        const errorStr = result.errors.join(' ');
        expect(errorStr).toContain('State consistency error');
        expect(errorStr).toContain('AUTH-001');
        expect(errorStr).toContain('implementing');
        expect(errorStr).toContain('backlog');
      }
    });
  });

  describe('Scenario: Repair work units state index inconsistencies', () => {
    it('should repair broken state indexes', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      const result = await repairWorkUnits({ cwd: testDir });

      expect(result.success).toBe(true);
      expect(result.repairs).toBeDefined();
      expect(result.repairs).toContain(
        'Moved AUTH-001 from backlog to implementing'
      );

      const updated = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      expect(updated.states.implementing).toContain('AUTH-001');
      expect(updated.states.backlog).not.toContain('AUTH-001');
    });
  });
});
