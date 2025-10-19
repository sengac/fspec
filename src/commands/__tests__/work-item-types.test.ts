/**
 * Feature: spec/features/work-item-types-for-stories-tasks-and-bugs.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData } from '../../types';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';

// Import commands
import { createWorkUnit } from '../create-work-unit';
import { listWorkUnits } from '../list-work-units';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { showWorkUnit } from '../show-work-unit';
import { updateWorkUnit } from '../update-work-unit';
import { queryWorkUnits } from '../query-work-units';
import { queryMetrics } from '../query-metrics';

describe('Feature: Work item types for stories, tasks, and bugs', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-work-item-types');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec/features'), { recursive: true });

    // Create foundation.json for all tests (required by commands)
    await createMinimalFoundation(testDir);
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create story work unit with explicit type', () => {
    it('should create work unit with type story', async () => {
      // Given I am in a project with fspec initialized
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            id: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      const workUnits: WorkUnitsData = {
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
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-work-unit AUTH 'User Login' --type=story"
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'User Login',
        type: 'story',
        cwd: testDir,
      });

      // Then a work unit should be created with id matching "AUTH-\d+"
      expect(result.success).toBe(true);
      expect(result.workUnitId).toMatch(/^AUTH-\d+$/);

      // And the work unit should have type "story"
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      expect(data.workUnits[result.workUnitId].type).toBe('story');

      // And the work unit should have title "User Login"
      expect(data.workUnits[result.workUnitId].title).toBe('User Login');
    });
  });

  describe('Scenario: Create task work unit with explicit type', () => {
    it('should create work unit with type task', async () => {
      // Given I am in a project with fspec initialized
      const prefixes: PrefixesData = {
        prefixes: {
          CLEAN: {
            id: 'CLEAN',
            description: 'Cleanup tasks',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      const workUnits: WorkUnitsData = {
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
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-work-unit CLEAN 'Audit coverage files' --type=task"
      const result = await createWorkUnit({
        prefix: 'CLEAN',
        title: 'Audit coverage files',
        type: 'task',
        cwd: testDir,
      });

      // Then a work unit should be created with id matching "CLEAN-\d+"
      expect(result.success).toBe(true);
      expect(result.workUnitId).toMatch(/^CLEAN-\d+$/);

      // And the work unit should have type "task"
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      expect(data.workUnits[result.workUnitId].type).toBe('task');

      // And the work unit should have title "Audit coverage files"
      expect(data.workUnits[result.workUnitId].title).toBe(
        'Audit coverage files'
      );
    });
  });

  describe('Scenario: Create bug work unit with explicit type', () => {
    it('should create work unit with type bug', async () => {
      // Given I am in a project with fspec initialized
      const prefixes: PrefixesData = {
        prefixes: {
          BUG: {
            id: 'BUG',
            description: 'Bug fixes',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      const workUnits: WorkUnitsData = {
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
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-work-unit BUG 'Login fails with @ symbol' --type=bug"
      const result = await createWorkUnit({
        prefix: 'BUG',
        title: 'Login fails with @ symbol',
        type: 'bug',
        cwd: testDir,
      });

      // Then a work unit should be created with id matching "BUG-\d+"
      expect(result.success).toBe(true);
      expect(result.workUnitId).toMatch(/^BUG-\d+$/);

      // And the work unit should have type "bug"
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      expect(data.workUnits[result.workUnitId].type).toBe('bug');

      // And the work unit should have title "Login fails with @ symbol"
      expect(data.workUnits[result.workUnitId].title).toBe(
        'Login fails with @ symbol'
      );
    });
  });

  describe('Scenario: Default type is story for backward compatibility', () => {
    it('should default to story type when not specified', async () => {
      // Given I am in a project with fspec initialized
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            id: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      const workUnits: WorkUnitsData = {
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
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-work-unit AUTH 'Feature'" without specifying type
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'Feature',
        // type not specified
        cwd: testDir,
      });

      // Then a work unit should be created with type "story"
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      expect(data.workUnits[result.workUnitId].type).toBe('story');

      // And the work unit should have title "Feature"
      expect(data.workUnits[result.workUnitId].title).toBe('Feature');
    });
  });

  describe('Scenario: Filter work units by type', () => {
    it('should filter work units by type parameter', async () => {
      // Given I have work units with different types
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            id: 'AUTH',
            description: 'Auth',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          CLEAN: {
            id: 'CLEAN',
            description: 'Cleanup',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          BUG: {
            id: 'BUG',
            description: 'Bugs',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      // And I have a story work unit "AUTH-001"
      // And I have a task work unit "CLEAN-001"
      // And I have a bug work unit "BUG-001"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Story work',
            type: 'story',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Task work',
            type: 'task',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'BUG-001': {
            id: 'BUG-001',
            title: 'Bug work',
            type: 'bug',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'CLEAN-001', 'BUG-001'],
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

      // When I run "fspec list-work-units --type=task"
      const result = await listWorkUnits({
        type: 'task',
        cwd: testDir,
      });

      // Then the output should include "CLEAN-001"
      expect(result.workUnits.some(wu => wu.id === 'CLEAN-001')).toBe(true);

      // And the output should not include "AUTH-001"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-001')).toBe(false);

      // And the output should not include "BUG-001"
      expect(result.workUnits.some(wu => wu.id === 'BUG-001')).toBe(false);
    });
  });

  describe('Scenario: Task workflow skips testing state', () => {
    it('should allow task to skip testing state', async () => {
      // Given I have a task work unit "CLEAN-001" in backlog
      const workUnits: WorkUnitsData = {
        workUnits: {
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Cleanup task',
            type: 'task',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['CLEAN-001'],
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

      // When I move the work unit through states: specifying → implementing → validating → done
      // Move to specifying
      await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'specifying',
        cwd: testDir,
      });

      // Move to implementing (should skip testing)
      const implementingResult = await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'implementing',
        cwd: testDir,
      });

      // Then each state transition should succeed
      expect(implementingResult.success).toBe(true);

      // Move to validating
      const validatingResult = await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'validating',
        cwd: testDir,
      });
      expect(validatingResult.success).toBe(true);

      // Move to done
      const doneResult = await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'done',
        cwd: testDir,
      });
      expect(doneResult.success).toBe(true);

      // And attempting to move to "testing" state should fail with error
      const workUnitsReset: WorkUnitsData = {
        workUnits: {
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Cleanup task',
            type: 'task',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['CLEAN-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsReset, null, 2)
      );

      // Try to move to testing (should fail)
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'CLEAN-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow();

      // And the error should explain "Tasks do not have a testing phase"
      try {
        await updateWorkUnitStatus({
          workUnitId: 'CLEAN-001',
          status: 'testing',
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message).toContain('testing');
          expect(error.message).toContain('task');
        }
      }
    });
  });

  describe('Scenario: Story requires feature file before moving to testing', () => {
    it('should fail when story moves to testing without feature file', async () => {
      // Given I have a story work unit "AUTH-001" in specifying state
      // And the story has no linked feature file
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User authentication',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
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

      // When I try to move the work unit to testing state
      // Then the command should fail with exit code 1
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'AUTH-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow();

      // And the output should contain "Cannot move to testing: no feature file linked"
      try {
        await updateWorkUnitStatus({
          workUnitId: 'AUTH-001',
          status: 'testing',
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          // Stories check for scenarios tagged with work unit ID
          expect(error.message).toContain('scenario');
        }
      }
    });
  });

  describe('Scenario: Task can move through workflow without feature file', () => {
    it('should allow task to move without feature file validation', async () => {
      // Given I have a task work unit "CLEAN-001" in backlog
      const workUnits: WorkUnitsData = {
        workUnits: {
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Cleanup task',
            type: 'task',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['CLEAN-001'],
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

      // When I move the work unit to implementing state
      // First move to specifying (required by ACDD)
      await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'specifying',
        cwd: testDir,
      });

      // Then move to implementing (tasks can skip testing)
      const result = await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'implementing',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And no feature file validation should occur
      // (verified by successful state transition without feature file)
    });
  });

  describe('Scenario: Bug must link to existing feature file', () => {
    it('should fail when bug moves to testing without linked feature file', async () => {
      // Given I have a bug work unit "BUG-001" in specifying state
      // And no feature file is linked to the bug
      const workUnits: WorkUnitsData = {
        workUnits: {
          'BUG-001': {
            id: 'BUG-001',
            title: 'Login bug',
            type: 'bug',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['BUG-001'],
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

      // When I try to move the work unit to testing state
      // Then the command should fail with exit code 1
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'BUG-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow();

      // And the output should contain "Bugs must link to existing feature file"
      try {
        await updateWorkUnitStatus({
          workUnitId: 'BUG-001',
          status: 'testing',
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message).toContain('bug');
          expect(error.message).toContain('feature file');
        }
      }

      // And the output should suggest "If feature has no spec, create a story instead"
      try {
        await updateWorkUnitStatus({
          workUnitId: 'BUG-001',
          status: 'testing',
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message).toContain('story');
        }
      }
    });
  });

  describe('Scenario: Bug links to existing feature file successfully', () => {
    it('should allow bug to move to testing with linked feature file', async () => {
      // Given I have a feature file "spec/features/user-authentication.feature"
      const featureContent = `@BUG-001
Feature: User Authentication
  Scenario: Login with credentials
    Given I have valid credentials
    When I attempt to login
    Then I should be authenticated`;

      await writeFile(
        join(testDir, 'spec/features/user-authentication.feature'),
        featureContent
      );

      // And I have a bug work unit "BUG-001" for that feature
      const workUnits: WorkUnitsData = {
        workUnits: {
          'BUG-001': {
            id: 'BUG-001',
            title: 'Login bug fix',
            type: 'bug',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['BUG-001'],
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

      // When I link the bug to the feature file
      // (Feature file already tagged with @BUG-001)

      // And I move the bug to testing state
      const result = await updateWorkUnitStatus({
        workUnitId: 'BUG-001',
        status: 'testing',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Work unit type is immutable after creation', () => {
    it('should prevent changing work unit type after creation', async () => {
      // Given I have a work unit "AUTH-001" with type "story"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User auth',
            type: 'story',
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

      // When I try to change the type to "task"
      // Then the command should fail
      await expect(
        updateWorkUnit({
          workUnitId: 'AUTH-001',
          type: 'task',
          cwd: testDir,
        })
      ).rejects.toThrow();

      // And the output should explain "Type is immutable. Delete and recreate if incorrect."
      try {
        await updateWorkUnit({
          workUnitId: 'AUTH-001',
          type: 'task',
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message).toContain('immutable');
          expect(error.message).toContain('Delete');
        }
      }
    });
  });

  describe('Scenario: Task supports optional Example Mapping in specifying phase', () => {
    it('should allow tasks to use Example Mapping fields optionally', async () => {
      // Given I have a task work unit "CLEAN-001" in specifying state
      const workUnits: WorkUnitsData = {
        workUnits: {
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Cleanup task',
            type: 'task',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['CLEAN-001'],
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

      // When I add rules and examples to the task using Example Mapping commands
      // (Simulated by directly updating work unit)
      const updatedWorkUnits: WorkUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      updatedWorkUnits.workUnits['CLEAN-001'].rules = [
        'Check all coverage files',
      ];
      updatedWorkUnits.workUnits['CLEAN-001'].examples = [
        'Run fspec show-coverage for each feature',
      ];
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(updatedWorkUnits, null, 2)
      );

      // Then the commands should succeed
      // And the task should store the rules and examples
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      expect(data.workUnits['CLEAN-001'].rules).toBeDefined();
      expect(data.workUnits['CLEAN-001'].examples).toBeDefined();

      // And moving to implementing should not require Example Mapping fields to be filled
      const result = await updateWorkUnitStatus({
        workUnitId: 'CLEAN-001',
        status: 'implementing',
        cwd: testDir,
      });
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Existing work units default to story type', () => {
    it('should treat existing work units without type as stories', async () => {
      // Given I have an existing work unit "AUTH-001" without a type field
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Legacy work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            // type field not present (legacy)
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

      // When I read the work unit
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the work unit should have type "story"
      expect(result.type).toBe('story');

      // And no migration command should be required
      // (Automatic defaulting verified by successful read)
    });
  });

  describe('Scenario: Query work units with type filtering', () => {
    it('should query work units filtered by type', async () => {
      // Given I have multiple work units of different types
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Story work',
            type: 'story',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Task work',
            type: 'task',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'BUG-001': {
            id: 'BUG-001',
            title: 'Bug work',
            type: 'bug',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'CLEAN-001', 'BUG-001'],
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

      // When I run "fspec query-work-units --type=task --format=json"
      const result = await queryWorkUnits({
        type: 'task',
        format: 'json',
        cwd: testDir,
      });

      // Then the JSON output should only include work units with type "task"
      expect(result.workUnits.every(wu => wu.type === 'task')).toBe(true);
      expect(result.workUnits.some(wu => wu.id === 'CLEAN-001')).toBe(true);
      expect(result.workUnits.some(wu => wu.id === 'AUTH-001')).toBe(false);
      expect(result.workUnits.some(wu => wu.id === 'BUG-001')).toBe(false);
    });
  });

  describe('Scenario: Metrics reporting can break down by type', () => {
    it('should report metrics filtered by type or combined', async () => {
      // Given I have completed work units of different types
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Story work',
            type: 'story',
            status: 'done',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'CLEAN-001': {
            id: 'CLEAN-001',
            title: 'Task work',
            type: 'task',
            status: 'done',
            estimate: 2,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'BUG-001': {
            id: 'BUG-001',
            title: 'Bug work',
            type: 'bug',
            status: 'done',
            estimate: 3,
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
          done: ['AUTH-001', 'CLEAN-001', 'BUG-001'],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec query-metrics --type=story"
      const storyResult = await queryMetrics({
        type: 'story',
        cwd: testDir,
      });

      // Then the output should show metrics for story work units only
      expect(storyResult.aggregateMetrics).toBeDefined();
      expect(storyResult.aggregateMetrics!.totalWorkUnits).toBe(1);
      expect(storyResult.aggregateMetrics!.completedWorkUnits).toBe(1);

      // When I run "fspec query-metrics" without type filter
      const combinedResult = await queryMetrics({
        cwd: testDir,
      });

      // Then the output should show combined metrics for all types
      expect(combinedResult.aggregateMetrics).toBeDefined();
      expect(combinedResult.aggregateMetrics!.totalWorkUnits).toBe(3);
      expect(combinedResult.aggregateMetrics!.completedWorkUnits).toBe(3);
    });
  });
});
