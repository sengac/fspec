/**
 * Feature: spec/features/context-aware-system-reminders-for-workflow-state-transitions.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

const TEST_DIR = join(process.cwd(), '.test-remind-009');
const SPEC_DIR = join(TEST_DIR, 'spec');

describe('Feature: Context-aware system-reminders for workflow state transitions', () => {
  beforeEach(async () => {
    await mkdir(SPEC_DIR, { recursive: true });

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
      join(SPEC_DIR, 'work-units.json'),
      JSON.stringify(workUnits, null, 2)
    );
  });

  afterEach(async () => {
    await rm(TEST_DIR, { recursive: true, force: true });
  });

  describe('Scenario: Show command reminder when transitioning to SPECIFYING state', () => {
    it('should emit system-reminder with discovery commands when moving to specifying', async () => {
      // Given I have a work unit in backlog status
      // (setup in beforeEach)

      // When I run "fspec update-work-unit-status TEST-001 specifying"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: TEST_DIR,
      });

      // Then a system-reminder should be emitted
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');

      // And the reminder should contain specific commands
      expect(result.systemReminder).toContain('fspec add-rule <id> "rule"');
      expect(result.systemReminder).toContain('fspec remove-rule <id> <index>');
      expect(result.systemReminder).toContain('fspec add-example <id> "example"');
      expect(result.systemReminder).toContain('fspec remove-example <id> <index>');
      expect(result.systemReminder).toContain('fspec add-question <id> "@human: question?"');
      expect(result.systemReminder).toContain('fspec answer-question <id> <index> --answer "..."');
      expect(result.systemReminder).toContain('fspec generate-scenarios <id>');
      expect(result.systemReminder).toContain('For more: fspec help discovery');

      // And commands should be displayed one per line
      const reminderContent = extractReminderContent(result.systemReminder!);
      const commandLines = reminderContent.split('\n').filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThan(5); // Multiple commands on separate lines
    });
  });

  describe('Scenario: Show command reminder when transitioning to TESTING state', () => {
    it('should emit system-reminder with testing commands when moving to testing', async () => {
      // Given I have a work unit in specifying status
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: TEST_DIR,
      });

      // When I run "fspec update-work-unit-status TEST-001 testing"
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'testing',
          cwd: TEST_DIR,
        })
      );

      // Then a system-reminder should be emitted
      expect(output.stderr).toContain('<system-reminder>');

      // And the reminder should contain testing commands
      expect(output.stderr).toContain('fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>');
      expect(output.stderr).toContain('fspec show-coverage <feature>');
      expect(output.stderr).toContain('fspec show-feature <name>');
      expect(output.stderr).toContain('For more: fspec link-coverage --help');

      // And commands should be displayed one per line
      const reminderContent = extractReminderContent(output.stderr);
      const commandLines = reminderContent.split('\n').filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThanOrEqual(3);
    });
  });

  describe('Scenario: Show command reminder when transitioning to IMPLEMENTING state', () => {
    it('should emit system-reminder with implementation commands when moving to implementing', async () => {
      // Given I have a work unit in testing status
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'specifying', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'testing', cwd: TEST_DIR });

      // When I run "fspec update-work-unit-status TEST-001 implementing"
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'implementing',
          cwd: TEST_DIR,
        })
      );

      // Then a system-reminder should be emitted
      expect(output.stderr).toContain('<system-reminder>');

      // And the reminder should contain implementation commands
      expect(output.stderr).toContain('fspec link-coverage <feature> --scenario "..." --impl-file <path> --impl-lines <lines>');
      expect(output.stderr).toContain('fspec checkpoint <id> <name>');
      expect(output.stderr).toContain('fspec restore-checkpoint <id> <name>');
      expect(output.stderr).toContain('fspec list-checkpoints <id>');
      expect(output.stderr).toContain('For more: fspec checkpoint --help');

      // And commands should be displayed one per line
      const reminderContent = extractReminderContent(output.stderr);
      const commandLines = reminderContent.split('\n').filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe('Scenario: Show command reminder when transitioning to VALIDATING state', () => {
    it('should emit system-reminder with validation commands (fspec only) when moving to validating', async () => {
      // Given I have a work unit in implementing status
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'specifying', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'testing', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'implementing', cwd: TEST_DIR });

      // When I run "fspec update-work-unit-status TEST-001 validating"
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'validating',
          cwd: TEST_DIR,
        })
      );

      // Then a system-reminder should be emitted
      expect(output.stderr).toContain('<system-reminder>');

      // And the reminder should contain validation commands
      expect(output.stderr).toContain('fspec validate');
      expect(output.stderr).toContain('fspec validate-tags');
      expect(output.stderr).toContain('fspec check');
      expect(output.stderr).toContain('fspec audit-coverage <feature>');
      expect(output.stderr).toContain('For more: fspec check --help');

      // And the reminder should NOT contain external commands
      const reminderContent = extractReminderContent(output.stderr);
      expect(reminderContent).not.toContain('npm test');
      expect(reminderContent).not.toContain('npm run check');

      // And commands should be displayed one per line
      const commandLines = reminderContent.split('\n').filter(line => line.includes('fspec'));
      expect(commandLines.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe('Scenario: No command reminder when transitioning to BACKLOG state', () => {
    it('should not emit command reminder when moving to backlog', async () => {
      // Given I have a work unit in specifying status
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'specifying', cwd: TEST_DIR });

      // When I run "fspec update-work-unit-status TEST-001 backlog"
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'backlog',
          cwd: TEST_DIR,
        })
      );

      // Then no command reminder system-reminder should be emitted
      // (There might be other system-reminders, but not about commands)
      if (output.stderr.includes('<system-reminder>')) {
        const reminderContent = extractReminderContent(output.stderr);
        expect(reminderContent).not.toContain('Common commands');
        expect(reminderContent).not.toContain('fspec add-rule');
      }
    });
  });

  describe('Scenario: No command reminder when transitioning to DONE state', () => {
    it('should not emit command reminder when moving to done', async () => {
      // Given I have a work unit in validating status
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'specifying', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'testing', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'implementing', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'validating', cwd: TEST_DIR });

      // When I run "fspec update-work-unit-status TEST-001 done"
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'done',
          cwd: TEST_DIR,
        })
      );

      // Then no command reminder system-reminder should be emitted
      if (output.stderr.includes('<system-reminder>')) {
        const reminderContent = extractReminderContent(output.stderr);
        expect(reminderContent).not.toContain('Common commands');
        expect(reminderContent).not.toContain('fspec validate');
      }
    });
  });

  describe('Scenario: No command reminder when transitioning to BLOCKED state', () => {
    it('should not emit command reminder when moving to blocked', async () => {
      // Given I have a work unit in implementing status
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'specifying', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'testing', cwd: TEST_DIR });
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'implementing', cwd: TEST_DIR });

      // When I run "fspec update-work-unit-status TEST-001 blocked"
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'blocked',
          cwd: TEST_DIR,
        })
      );

      // Then no command reminder system-reminder should be emitted
      if (output.stderr.includes('<system-reminder>')) {
        const reminderContent = extractReminderContent(output.stderr);
        expect(reminderContent).not.toContain('Common commands');
        expect(reminderContent).not.toContain('fspec');
      }
    });
  });

  describe('Scenario: Show reminder every time status changes', () => {
    it('should show reminder again when returning to same state', async () => {
      // Given I have a work unit that has previously been in specifying status
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'specifying', cwd: TEST_DIR });

      // And I moved it to testing and saw a testing reminder
      await updateWorkUnitStatus({ id: 'TEST-001', status: 'testing', cwd: TEST_DIR });

      // When I move it back to specifying status
      const output = await captureOutput(() =>
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'specifying',
          cwd: TEST_DIR,
        })
      );

      // Then a system-reminder for specifying state should be emitted again
      expect(output.stderr).toContain('<system-reminder>');
      expect(output.stderr).toContain('fspec add-rule <id> "rule"');
      expect(output.stderr).toContain('fspec generate-scenarios <id>');

      // And it should show the same commands as the first time
      expect(output.stderr).toContain('For more: fspec help discovery');
    });
  });
});

// Helper functions
interface CapturedOutput {
  stdout: string;
  stderr: string;
}

async function captureOutput(fn: () => Promise<void>): Promise<CapturedOutput> {
  const originalStdout = process.stdout.write;
  const originalStderr = process.stderr.write;

  let stdout = '';
  let stderr = '';

  process.stdout.write = ((chunk: string) => {
    stdout += chunk;
    return true;
  }) as any;

  process.stderr.write = ((chunk: string) => {
    stderr += chunk;
    return true;
  }) as any;

  try {
    await fn();
  } finally {
    process.stdout.write = originalStdout;
    process.stderr.write = originalStderr;
  }

  return { stdout, stderr };
}

function extractReminderContent(stderr: string): string {
  const start = stderr.indexOf('<system-reminder>');
  const end = stderr.indexOf('</system-reminder>');

  if (start === -1 || end === -1) {
    return '';
  }

  return stderr.substring(start + '<system-reminder>'.length, end);
}
