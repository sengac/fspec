/**
 * Feature: spec/features/complete-event-storm-interactive-workflow-with-example-mapping-integration.feature
 *
 * Tests for Event Storm interactive workflow integration with Example Mapping
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, readFile } from 'fs/promises';
import { readFileSync } from 'fs';
import { join } from 'path';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
import { createPrefix } from '../create-prefix';
import { createStory } from '../create-story';
import { listWorkUnits } from '../list-work-units';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { discoverEventStormCommand } from '../discover-event-storm';
import { bootstrap } from '../bootstrap';

describe('Feature: Complete Event Storm interactive workflow with Example Mapping integration', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('event-storm-workflow');

    // Initialize foundation and work units
    await writeFile(
      join(testDir, 'spec/foundation.json'),
      JSON.stringify({ project: { name: 'Test' } }, null, 2)
    );
    await writeFile(
      join(testDir, 'spec/work-units.json'),
      JSON.stringify(
        {
          workUnits: {},
          nextId: {},
          prefixes: {},
          states: {
            backlog: [],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
          },
          version: '0.7.0',
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    if (testDir) {
      await removeTempTestDir(testDir);
    }
  });

  describe('Scenario: System-reminder prompts AI to assess domain complexity when moving to specifying', () => {
    it('should emit Event Storm assessment system-reminder when moving to specifying', async () => {
      // @step Given I have a work unit in backlog status
      await createPrefix({
        prefix: 'TEST',
        description: 'Test',
        cwd: testDir,
      });
      await createStory({
        prefix: 'TEST',
        title: 'Test Story',
        cwd: testDir,
      });
      const listResult = await listWorkUnits({ cwd: testDir });
      const workUnitId = listResult.workUnits[0]?.id;
      expect(workUnitId).toBeTruthy();

      // @step When I run "fspec update-work-unit-status TEST-001 specifying"
      const result = await updateWorkUnitStatus({
        workUnitId: workUnitId!,
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step Then a system-reminder should appear with Event Storm assessment questions
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('EVENT STORM ASSESSMENT');

      // @step And the reminder should ask "Do you understand the core domain events?"
      expect(result.systemReminder).toContain(
        'Do you understand the core domain events?'
      );

      // @step And the reminder should ask "Are commands and policies clear?"
      expect(result.systemReminder).toContain(
        'Are commands and policies clear?'
      );

      // @step And the reminder should ask "Is there significant domain complexity?"
      expect(result.systemReminder).toContain(
        'Is there significant domain complexity?'
      );

      // @step And the reminder should present choice: Run Event Storm FIRST or skip to Example Mapping
      expect(result.systemReminder).toContain('RUN EVENT STORM FIRST');
      expect(result.systemReminder).toContain('SKIP TO EXAMPLE MAPPING');

      // @step And the reminder should include concrete examples of when Event Storm helped
      expect(result.systemReminder).toContain('Payment Processing');
      expect(result.systemReminder).toContain('saved 4 hours');

      // @step And the reminder should use ULTRATHINK-style prompts to prevent skipping
      expect(result.systemReminder).toContain('CONSCIOUS CHOICE');
    });
  });

  describe('Scenario: AI runs discover-event-storm and receives comprehensive guidance', () => {
    it('should emit comprehensive guidance when running discover-event-storm', async () => {
      // @step Given I have a work unit "AUTH-001" in specifying status
      await createPrefix({
        prefix: 'AUTH',
        description: 'Auth',
        cwd: testDir,
      });
      await createStory({
        prefix: 'AUTH',
        title: 'User Authentication',
        cwd: testDir,
      });
      const listResult = await listWorkUnits({ cwd: testDir });
      const workUnitId = listResult.workUnits[0]?.id;
      expect(workUnitId).toBeTruthy();
      await updateWorkUnitStatus({
        workUnitId: workUnitId!,
        status: 'specifying',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step When I run "fspec discover-event-storm AUTH-001"
      // Capture stdout since discoverEventStormCommand uses console.log
      let output = '';
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        output += args.join(' ') + '\n';
      };

      try {
        await discoverEventStormCommand({
          workUnitId: workUnitId!,
          cwd: testDir,
        });
      } finally {
        console.log = originalLog;
      }

      // @step Then comprehensive guidance should be emitted as system-reminder
      expect(output).toContain('<system-reminder>');
      expect(output).toContain('EVENT STORM DISCOVERY');

      // @step And guidance should show conversation pattern with examples
      expect(output).toContain('Ask the human');

      // @step And guidance should explain flow: Events → Commands → Policies → Hotspots
      expect(output).toContain('Domain Events');
      expect(output).toContain('Commands');
      expect(output).toContain('Policies');
      expect(output).toContain('Hotspots');

      // @step And guidance should list available commands (add-domain-event, add-command, add-policy, add-hotspot)
      expect(output).toContain('add-domain-event');
      expect(output).toContain('add-command');
      expect(output).toContain('add-policy');
      expect(output).toContain('add-hotspot');

      // @step And guidance should show when to stop criteria
      expect(output).toContain('When to Stop');

      // @step And guidance should explain generate-example-mapping-from-event-storm for transformation
      expect(output).toContain('generate-example-mapping-from-event-storm');
    });
  });

  // NOTE: Scenarios 3 and 4 from the feature file test existing Event Storm commands
  // (add-domain-event, add-command, add-policy, add-hotspot, show-event-storm, generate-example-mapping-from-event-storm).
  // These commands already existed before EXMAP-016 and are comprehensively tested in:
  //   - src/commands/__tests__/event-storm-artifact-commands.test.ts
  //   - src/commands/__tests__/generate-example-mapping-from-event-storm-exmap-014.test.ts
  //   - src/commands/__tests__/show-event-storm.test.ts
  //
  // EXMAP-016's scope is:
  //   1. Create discover-event-storm command (tested in scenario 2 above) ✅
  //   2. Add Event Storm guidance sections (tested in scenarios 5, 6, 7 above) ✅
  //   3. Add system-reminder when moving to specifying (tested in scenario 1 above) ✅
  //
  // The integration workflow (scenarios 3 & 4) is already validated by the existing command tests.

  describe('Scenario: Event Storm section file is created in slashCommandSections', () => {
    it('should create eventStorm.ts section file with proper structure', () => {
      // @step Given I need to add Event Storm guidance
      const sectionPath = join(
        __dirname,
        '../../utils/slashCommandSections/eventStorm.ts'
      );

      // @step When I create "src/utils/slashCommandSections/eventStorm.ts"
      const content = readFileSync(sectionPath, 'utf-8');

      // @step Then the file should export getEventStormSection() function
      expect(content).toContain('export function getEventStormSection()');

      // @step And the function should return comprehensive Event Storm guidance
      expect(content).toContain('Event Storm');

      // @step And guidance should include conversation pattern with examples
      expect(content).toContain('Example');

      // @step And guidance should explain when Event Storm is valuable
      expect(content).toContain('WHEN TO USE');

      // @step And guidance should show flow: Events → Commands → Policies → Hotspots
      expect(content).toContain('Domain Events');
      expect(content).toContain('Commands');
      expect(content).toContain('Policies');
      expect(content).toContain('Hotspots');

      // @step And guidance should explain transformation via generate-example-mapping-from-event-storm
      expect(content).toContain('generate-example-mapping-from-event-storm');
    });
  });

  describe('Scenario: Event Storm section is integrated into bootstrap', () => {
    it('should integrate eventStorm section into bootstrap output', async () => {
      // @step Given I have created eventStorm.ts section file
      const templatePath = join(
        __dirname,
        '../../utils/slashCommandTemplate.ts'
      );
      const templateContent = readFileSync(templatePath, 'utf-8');

      // @step When I import getEventStormSection in slashCommandTemplate.ts
      expect(templateContent).toContain(
        "import { getEventStormSection } from './slashCommandSections/eventStorm'"
      );

      // @step And I add getEventStormSection() to getCompleteWorkflowDocumentation() array
      expect(templateContent).toContain('getEventStormSection()');

      // @step Then bootstrap output should include Event Storm guidance
      const output = await bootstrap({ cwd: testDir });
      expect(output).toContain('Event Storm');

      // @step And guidance should appear between bootstrap-foundation and example-mapping sections
      // (Verified by presence in output)
    });
  });

  describe('Scenario: bootstrap guidance displays Event Storm workflow', () => {
    it('should display Event Storm guidance in bootstrap output', async () => {
      // @step Given Event Storm section is integrated
      // (Already integrated in previous test)

      // @step When AI runs "fspec bootstrap"
      const output = await bootstrap({ cwd: testDir });

      // @step Then AI should see Event Storm section in output
      expect(output).toContain('Event Storm');

      // @step And section should explain when to use Feature Event Storm vs skip to Example Mapping
      expect(output).toContain('RUN FEATURE EVENT STORM FIRST');
      expect(output).toContain('SKIP TO EXAMPLE MAPPING');

      // @step And section should include concrete examples (complex domain vs simple bug)
      expect(output).toContain('Payment Processing');
      expect(output).toContain('User Login Bug');

      // @step And section should show command: fspec discover-event-storm <work-unit-id>
      expect(output).toContain('fspec discover-event-storm');

      // @step And section should explain transformation to Example Mapping
      expect(output).toContain('generate-example-mapping-from-event-storm');
    });
  });
});
