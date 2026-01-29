/**
 * Feature: spec/features/conversational-review-prompt-before-done.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Conversational Review Prompt Before Done', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('conversational-review-prompt-before-done');

    // Configure agent as 'claude' to get system-reminder tags
    const configData = { agent: 'claude' };
    await writeJsonTestFile(
      join(setup.specDir, 'fspec-config.json'),
      configData
    );
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Story work unit transitioning to done shows review prompt', () => {
    it('should emit system-reminder suggesting quality review before marking done', async () => {
      // @step Given I have a story work unit "AUTH-001" in "validating" status
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
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
        nextIds: { AUTH: 2 },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I run "fspec update-work-unit-status AUTH-001 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: setup.testDir,
      });

      // @step Then the command should emit a system-reminder
      expect(result.output).toContain('<system-reminder>');
      expect(result.output).toContain('</system-reminder>');

      // @step And the system-reminder should suggest asking user "Would you like me to run fspec review AUTH-001 to check for quality issues?"
      expect(result.output).toContain('QUALITY CHECK OPPORTUNITY');
      expect(result.output).toContain('fspec review AUTH-001');
      expect(result.output).toContain('Would you like me to run');

      // @step And the system-reminder should include the exact command "fspec review AUTH-001"
      expect(result.output).toContain('fspec review AUTH-001');

      // @step And the system-reminder should be agent-aware formatted
      // (Verified by checking for <system-reminder> tags with claude agent)
      expect(result.output).toContain('<system-reminder>');

      // @step And the work unit status should be updated to "done"
      expect(result.success).toBe(true);
      expect(result.output).toContain('status updated to done');
    });
  });

  describe('Scenario: Bug work unit transitioning to done shows review prompt', () => {
    it('should emit system-reminder with workflow steps for bug work units', async () => {
      // @step Given I have a bug work unit "BUG-001" in "validating" status
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BUG-001': {
            id: 'BUG-001',
            title: 'Fix login validation',
            type: 'bug',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BUG-001'],
          done: [],
          blocked: [],
        },
        nextIds: { BUG: 2 },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I run "fspec update-work-unit-status BUG-001 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'BUG-001',
        status: 'done',
        cwd: setup.testDir,
      });

      // @step Then the command should emit a system-reminder
      expect(result.output).toContain('<system-reminder>');

      // @step And the system-reminder should suggest asking user about running fspec review
      expect(result.output).toContain('fspec review BUG-001');
      expect(result.output).toContain('quality review');

      // @step And the system-reminder should include suggested workflow steps
      expect(result.output).toContain('1.');
      expect(result.output).toContain('2.');
      expect(result.output).toContain('3.');

      // @step And the work unit status should be updated to "done"
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Task work unit transitioning to done does NOT show review prompt', () => {
    it('should NOT emit review prompt for task work units', async () => {
      // @step Given I have a task work unit "TASK-001" in "validating" status
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TASK-001': {
            id: 'TASK-001',
            title: 'Update dependencies',
            type: 'task',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['TASK-001'],
          done: [],
          blocked: [],
        },
        nextIds: { TASK: 2 },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I run "fspec update-work-unit-status TASK-001 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TASK-001',
        status: 'done',
        cwd: setup.testDir,
      });

      // @step Then the command should NOT emit a review prompt system-reminder
      // Check that output does NOT contain quality review prompt
      expect(result.output).not.toContain('QUALITY CHECK OPPORTUNITY');
      expect(result.output).not.toContain('fspec review TASK-001');

      // @step And the work unit status should be updated to "done"
      expect(result.success).toBe(true);

      // @step And the output should be the standard success message only
      expect(result.output).toContain('status updated to done');
    });
  });

  describe('Scenario: AI workflow - user accepts review suggestion', () => {
    it('should guide AI through review workflow when user accepts', async () => {
      // @step Given I have a story work unit "API-001" in "validating" status
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'API-001': {
            id: 'API-001',
            title: 'REST API endpoints',
            type: 'story',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['API-001'],
          done: [],
          blocked: [],
        },
        nextIds: { API: 2 },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step And the AI is moving the work unit to done
      // @step When the system-reminder prompts the AI to ask about review
      const result = await updateWorkUnitStatus({
        workUnitId: 'API-001',
        status: 'done',
        cwd: setup.testDir,
      });

      // @step And the user responds "yes, please run the review"
      // (This step is narrative - system-reminder guides AI to ask user)

      // @step Then the AI should run "fspec review API-001"
      // Verify system-reminder suggests this command
      expect(result.output).toContain('fspec review API-001');

      // @step And the AI should address any findings from the review
      // Verify system-reminder instructs AI to fix issues
      expect(result.output).toContain('address findings');

      // @step And the AI should mark the work unit as done after fixes
      // Verify workflow step mentions marking done after review
      expect(result.output).toContain('then mark done');
    });
  });

  describe('Scenario: AI workflow - user declines review suggestion', () => {
    it('should allow AI to proceed when user declines review', async () => {
      // @step Given I have a story work unit "UI-001" in "validating" status
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'UI-001': {
            id: 'UI-001',
            title: 'Dashboard UI',
            type: 'story',
            status: 'validating',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['UI-001'],
          done: [],
          blocked: [],
        },
        nextIds: { UI: 2 },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step And the AI is moving the work unit to done
      // @step When the system-reminder prompts the AI to ask about review
      const result = await updateWorkUnitStatus({
        workUnitId: 'UI-001',
        status: 'done',
        cwd: setup.testDir,
      });

      // @step And the user responds "no, just mark it done"
      // (This step is narrative - system-reminder is non-blocking)

      // @step Then the AI should proceed to mark the work unit as done
      // Verify work unit is updated to done
      expect(result.success).toBe(true);
      expect(result.output).toContain('status updated to done');

      // @step And the AI should skip running fspec review
      // Verify system-reminder presents this as an option (If no: Proceed)
      expect(result.output).toContain('If no');
      expect(result.output).toContain('Proceed');
    });
  });
});
