/**
 * Feature: spec/features/context-aware-system-reminders-for-workflow-state-transitions.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: Context-aware system-reminders for workflow state transitions', () => {
  let testDir: string;
  let specDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('remind-009');
    specDir = join(testDir, 'spec');

    await mkdir(join(specDir, 'features'), { recursive: true });

    // Create minimal work-units.json
    const workUnits = {
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          type: 'story',
          title: 'Test Story',
          description: 'Test description',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          rules: ['Test story must validate system reminders correctly'],
          examples: ['Reminders shown during state transitions'],
          architectureNotes: [
            'Implementation: Display context-aware reminders for each workflow state',
          ],
          attachments: ['spec/attachments/TEST-001/ast-research.json'],
        },
      },
      states: {
        backlog: ['TEST-001'],
        specifying: [],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
      nextIds: { TEST: 2 },
    };

    await writeFile(
      join(specDir, 'work-units.json'),
      JSON.stringify(workUnits, null, 2)
    );

    // Create feature file with scenario for TEST-001
    const featureContent = `@TEST-001
Feature: Test Feature

  Background: User Story
    As a tester
    I want to test reminders
    So that I can verify system behavior

  Scenario: Test scenario
    Given I have a test
    When I run a test
    Then I see results
`;
    await writeFile(
      join(specDir, 'features', 'test-feature.feature'),
      featureContent
    );

    // Create coverage file for test-feature.feature
    const coverageContent = {
      scenarios: [
        {
          name: 'Test scenario',
          testMappings: [
            {
              file: 'src/__tests__/test-feature.test.ts',
              lines: '1-10',
              implMappings: [
                {
                  file: 'src/test-impl.ts',
                  lines: '1-10',
                },
              ],
            },
          ],
        },
      ],
    };
    await writeFile(
      join(specDir, 'features', 'test-feature.feature.coverage'),
      JSON.stringify(coverageContent, null, 2)
    );

    // Create the test file referenced in coverage
    await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
    const testFileContent = `// @step Given  I have a test
// @step When  I run a test
// @step Then  I see results
describe('Test scenario', () => {
  it('should work', () => {
    expect(true).toBe(true);
  });
});
`;
    await writeFile(
      join(testDir, 'src', '__tests__', 'test-feature.test.ts'),
      testFileContent
    );

    // Create the implementation file referenced in coverage
    const implFileContent = `export function testImpl() {
  return true;
}
`;
    await writeFile(join(testDir, 'src', 'test-impl.ts'), implFileContent);
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Show command reminder when transitioning to SPECIFYING state', () => {
    it('should emit system-reminder with discovery commands when moving to specifying', async () => {
      // Given I have a work unit in backlog status
      // (setup in beforeEach)

      // When I run "fspec update-work-unit-status TEST-001 specifying"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then a system-reminder should be emitted
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');

      // And the reminder should contain specific commands
      expect(result.systemReminder).toContain('fspec add-rule <id> "rule"');
      expect(result.systemReminder).toContain('fspec remove-rule <id> <index>');
      expect(result.systemReminder).toContain(
        'fspec add-example <id> "example"'
      );
      expect(result.systemReminder).toContain(
        'fspec remove-example <id> <index>'
      );
      expect(result.systemReminder).toContain(
        'fspec add-question <id> "@human: question?"'
      );
      expect(result.systemReminder).toContain(
        'fspec answer-question <id> <index> --answer "..."'
      );
      expect(result.systemReminder).toContain('fspec generate-scenarios <id>');
      expect(result.systemReminder).toContain('For more: fspec help discovery');

      // And commands should be displayed one per line
      const reminderContent = extractReminderContent(result.systemReminder!);
      const commandLines = reminderContent
        .split('\n')
        .filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThan(5); // Multiple commands on separate lines
    });
  });

  describe('Scenario: Show command reminder when transitioning to TESTING state', () => {
    it('should emit system-reminder with testing commands when moving to testing', async () => {
      // Given I have a work unit in specifying status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I run "fspec update-work-unit-status TEST-001 testing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then a system-reminder should be emitted
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');

      // And the reminder should contain testing commands
      expect(result.systemReminder).toContain(
        'fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>'
      );
      expect(result.systemReminder).toContain('fspec show-coverage <feature>');
      expect(result.systemReminder).toContain('fspec show-feature <name>');
      expect(result.systemReminder).toContain(
        'For more: fspec link-coverage --help'
      );

      // And commands should be displayed one per line
      const reminderContent = extractReminderContent(result.systemReminder!);
      const commandLines = reminderContent
        .split('\n')
        .filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThanOrEqual(3);
    });
  });

  describe('Scenario: Show command reminder when transitioning to IMPLEMENTING state', () => {
    it('should emit system-reminder with implementation commands when moving to implementing', async () => {
      // Given I have a work unit in testing status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I run "fspec update-work-unit-status TEST-001 implementing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'implementing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then a system-reminder should be emitted
      expect(result.systemReminder).toContain('<system-reminder>');

      // And the reminder should contain implementation commands
      expect(result.systemReminder).toContain(
        'fspec link-coverage <feature> --scenario "..." --test-file <path> --impl-file <path> --impl-lines <lines>'
      );
      expect(result.systemReminder).toContain('fspec checkpoint <id> <name>');
      expect(result.systemReminder).toContain(
        'fspec restore-checkpoint <id> <name>'
      );
      expect(result.systemReminder).toContain('fspec list-checkpoints <id>');
      expect(result.systemReminder).toContain(
        'For more: fspec checkpoint --help'
      );

      // And commands should be displayed one per line
      const reminderContent = extractReminderContent(result.systemReminder);
      const commandLines = reminderContent
        .split('\n')
        .filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe('Scenario: Show command reminder when transitioning to VALIDATING state', () => {
    it('should emit system-reminder with validation commands (fspec only) when moving to validating', async () => {
      // Given I have a work unit in implementing status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'implementing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I run "fspec update-work-unit-status TEST-001 validating"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'validating',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then a system-reminder should be emitted
      expect(result.systemReminder).toContain('<system-reminder>');

      // And the reminder should contain validation commands
      expect(result.systemReminder).toContain('fspec validate');
      expect(result.systemReminder).toContain('fspec validate-tags');
      expect(result.systemReminder).toContain('fspec check');
      expect(result.systemReminder).toContain('fspec audit-coverage <feature>');
      expect(result.systemReminder).toContain('For more: fspec check --help');

      // And the reminder should NOT contain external commands
      const reminderContent = extractReminderContent(result.systemReminder);
      expect(reminderContent).not.toContain('npm test');
      expect(reminderContent).not.toContain('npm run check');

      // And commands should be displayed one per line
      const commandLines = reminderContent
        .split('\n')
        .filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe('Scenario: No command reminder when transitioning to BACKLOG state', () => {
    it('should prevent moving back to backlog from other states', async () => {
      // Given I have a work unit in specifying status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I try to run "fspec update-work-unit-status TEST-001 backlog"
      // Then it should throw an error
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'backlog',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow('Cannot move work back to backlog');
    });
  });

  describe('Scenario: No command reminder when transitioning to DONE state', () => {
    it('should not emit command reminder when moving to done', async () => {
      // Given I have a work unit in validating status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'implementing',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'validating',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I run "fspec update-work-unit-status TEST-001 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'done',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then no command reminder system-reminder should be emitted
      if (result.systemReminder.includes('<system-reminder>')) {
        const reminderContent = extractReminderContent(result.systemReminder);
        expect(reminderContent).not.toContain('Common commands');
        expect(reminderContent).not.toContain('fspec validate');
      }
    });
  });

  describe('Scenario: No command reminder when transitioning to BLOCKED state', () => {
    it('should not emit command reminder when moving to blocked', async () => {
      // Given I have a work unit in implementing status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'implementing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I run "fspec update-work-unit-status TEST-001 blocked"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'blocked',
        blockedReason: 'Test block reason',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then a system-reminder should be emitted with blocker guidance
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('BLOCKED status');

      // But it should not have the "Common commands" section like other states
      const reminderContent = extractReminderContent(result.systemReminder);
      expect(reminderContent).not.toContain(
        'Common commands for BLOCKED state'
      );
    });
  });

  describe('Scenario: Show reminder every time status changes', () => {
    it('should show reminder again when returning to same state', async () => {
      // Given I have a work unit that has previously been in specifying status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // And I moved it to testing and saw a testing reminder
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // When I move it back to specifying status
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then a system-reminder for specifying state should be emitted again
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('fspec add-rule <id> "rule"');
      expect(result.systemReminder).toContain('fspec generate-scenarios <id>');

      // And it should show the same commands as the first time
      expect(result.systemReminder).toContain('For more: fspec help discovery');
    });
  });
});

// Helper function
function extractReminderContent(stderr: string): string {
  const start = stderr.indexOf('<system-reminder>');
  const end = stderr.indexOf('</system-reminder>');

  if (start === -1 || end === -1) {
    return '';
  }

  return stderr.substring(start + '<system-reminder>'.length, end);
}
