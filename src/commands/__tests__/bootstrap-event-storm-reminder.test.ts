/**
 * Feature: spec/features/update-bootstrap-output-with-big-picture-event-storming-guidance.feature
 *
 * Tests for Big Picture Event Storming reminder in bootstrap output
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { bootstrap } from '../bootstrap';

const TEST_DIR = join(__dirname, '../../../test-temp-bootstrap-event-storm');

describe('Feature: Update bootstrap output with Big Picture Event Storming guidance', () => {
  beforeEach(async () => {
    // Create test directory structure
    await mkdir(TEST_DIR, { recursive: true });
    await mkdir(join(TEST_DIR, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Cleanup test directory
    await rm(TEST_DIR, { recursive: true, force: true });
  });

  describe('Scenario: Reminder emitted when eventStorm empty and work unit exists', () => {
    it('should emit system-reminder with work unit ID and next steps', async () => {
      // @step Given foundation.json exists with empty eventStorm field
      const foundationPath = join(TEST_DIR, 'spec', 'foundation.json');
      await writeFile(
        foundationPath,
        JSON.stringify(
          {
            version: '2.0.0',
            project: {
              name: 'Test Project',
              vision: 'Test vision',
              projectType: 'cli-tool',
            },
            eventStorm: {
              items: [],
            },
          },
          null,
          2
        )
      );

      // @step And a FOUND-XXX Event Storm work unit exists in backlog status
      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      await writeFile(
        workUnitsPath,
        JSON.stringify(
          {
            version: '2.0.0',
            workUnits: {
              'FOUND-001': {
                id: 'FOUND-001',
                title: 'Conduct Big Picture Event Storming for Foundation',
                description: 'Event Storm description',
                type: 'story',
                status: 'backlog',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            },
          },
          null,
          2
        )
      );

      // @step When I run "fspec bootstrap"
      const output = await bootstrap({ cwd: TEST_DIR });

      // @step Then a system-reminder should be emitted
      expect(output).toContain('<system-reminder>');
      expect(output).toContain('</system-reminder>');

      // @step And the reminder should reference the work unit ID
      expect(output).toContain('FOUND-001');

      // @step And the reminder should provide next steps to work on the work unit
      expect(output).toContain('fspec show-work-unit FOUND-001');
      expect(output).toContain('fspec update-work-unit-status FOUND-001');

      // @step And the reminder should list foundation Event Storm commands
      expect(output).toContain('fspec add-foundation-bounded-context');
      expect(output).toContain('fspec add-aggregate-to-foundation');
      expect(output).toContain('fspec add-domain-event-to-foundation');
      expect(output).toContain('fspec show-foundation-event-storm');

      // @step And the reminder should reference CLAUDE.md documentation
      expect(output).toContain('CLAUDE.md');

      // @step And the reminder should explain why Event Storm matters
      expect(output).toContain('bounded context');

      // @step And the reminder should appear after CLAUDE.md content
      // Verify reminder appears after help content but before final "fspec mode" message
      const reminderIndex = output.indexOf('<system-reminder>');
      const claudeMdIndex = output.indexOf(
        '## Step 12: Complete Command Reference'
      );
      expect(reminderIndex).toBeGreaterThan(claudeMdIndex);
    });
  });

  describe('Scenario: Reminder emitted when eventStorm empty and no work unit', () => {
    it('should emit system-reminder suggesting to create work unit or run commands', async () => {
      // @step Given foundation.json exists with empty eventStorm field
      const foundationPath = join(TEST_DIR, 'spec', 'foundation.json');
      await writeFile(
        foundationPath,
        JSON.stringify(
          {
            version: '2.0.0',
            project: {
              name: 'Test Project',
              vision: 'Test vision',
              projectType: 'cli-tool',
            },
            eventStorm: {
              items: [],
            },
          },
          null,
          2
        )
      );

      // @step And NO Event Storm work unit exists
      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      await writeFile(
        workUnitsPath,
        JSON.stringify(
          {
            version: '2.0.0',
            workUnits: {},
          },
          null,
          2
        )
      );

      // @step When I run "fspec bootstrap"
      const output = await bootstrap({ cwd: TEST_DIR });

      // @step Then a system-reminder should be emitted
      expect(output).toContain('<system-reminder>');
      expect(output).toContain('</system-reminder>');

      // @step And the reminder should suggest creating a work unit OR running commands directly
      expect(output).toContain('Option 1');
      expect(output).toContain('Option 2');
      expect(output).toContain('fspec create-story');

      // @step And the reminder should list foundation Event Storm commands
      expect(output).toContain('fspec add-foundation-bounded-context');
      expect(output).toContain('fspec add-aggregate-to-foundation');
      expect(output).toContain('fspec add-domain-event-to-foundation');
      expect(output).toContain('fspec show-foundation-event-storm');

      // @step And the reminder should reference CLAUDE.md documentation
      expect(output).toContain('CLAUDE.md');

      // @step And the reminder should explain why Event Storm matters
      expect(output).toContain('bounded context');
    });
  });

  describe('Scenario: No reminder when eventStorm already populated', () => {
    it('should not emit system-reminder about Event Storm', async () => {
      // @step Given foundation.json exists with populated eventStorm field
      const foundationPath = join(TEST_DIR, 'spec', 'foundation.json');
      await writeFile(
        foundationPath,
        JSON.stringify(
          {
            version: '2.0.0',
            project: {
              name: 'Test Project',
              vision: 'Test vision',
              projectType: 'cli-tool',
            },
            eventStorm: {
              items: [
                {
                  type: 'bounded-context',
                  name: 'Authentication',
                  description: 'User authentication context',
                },
              ],
            },
          },
          null,
          2
        )
      );

      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      await writeFile(
        workUnitsPath,
        JSON.stringify(
          {
            version: '2.0.0',
            workUnits: {},
          },
          null,
          2
        )
      );

      // @step When I run "fspec bootstrap"
      const output = await bootstrap({ cwd: TEST_DIR });

      // @step Then NO system-reminder should be emitted about Event Storm
      expect(output).not.toContain('BIG PICTURE EVENT STORMING NEEDED');
      expect(output).not.toContain('eventStorm field is empty');

      // @step And bootstrap output should show normal CLAUDE.md content
      expect(output).toContain('## Step 12: Complete Command Reference');
    });
  });

  describe('Scenario: No reminder when foundation.json does not exist', () => {
    it('should not emit system-reminder about Event Storm', async () => {
      // @step Given foundation.json does NOT exist
      // (No foundation.json created in this test)

      const workUnitsPath = join(TEST_DIR, 'spec', 'work-units.json');
      await writeFile(
        workUnitsPath,
        JSON.stringify(
          {
            version: '2.0.0',
            workUnits: {},
          },
          null,
          2
        )
      );

      // @step When I run "fspec bootstrap"
      const output = await bootstrap({ cwd: TEST_DIR });

      // @step Then NO system-reminder should be emitted about Event Storm
      expect(output).not.toContain('BIG PICTURE EVENT STORMING NEEDED');
      expect(output).not.toContain('eventStorm field is empty');

      // @step And bootstrap output should show normal CLAUDE.md content
      expect(output).toContain('## Step 12: Complete Command Reference');
    });
  });
});
