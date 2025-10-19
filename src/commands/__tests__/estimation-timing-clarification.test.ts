/**
 * Feature: spec/features/clarify-estimation-timing-in-documentation-and-system-reminders.feature
 *
 * These tests verify that system-reminders, error messages, and documentation
 * clearly communicate when to estimate story points: AFTER generating scenarios
 * from Example Mapping, not before.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { showWorkUnit } from '../show-work-unit';
import { updateWorkUnitEstimate } from '../update-work-unit-estimate';

describe('Feature: Clarify estimation timing in documentation and system-reminders', () => {
  let testDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-estimation-timing-test-'));
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });
    await mkdir(join(specDir, 'features'), { recursive: true });
    workUnitsFile = join(specDir, 'work-units.json');
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: System-reminder in backlog state should not suggest adding estimates', () => {
    it('should NOT suggest adding estimate when work unit is in backlog', async () => {
      // Given I have a work unit in backlog state with no estimate
      const workUnits = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test work unit',
            type: 'story',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            relationships: {},
          },
        },
        kanban: {
          backlog: ['TEST-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit <work-unit-id>"
      const result = await showWorkUnit({ workUnitId: 'TEST-001', cwd: testDir });

      // Then the system-reminder should NOT suggest adding an estimate
      const reminders = result.systemReminders || [];
      const reminderText = reminders.join('\n');
      expect(reminderText).not.toContain('Use Example Mapping results to estimate');
      expect(reminderText).not.toContain('estimate story points');

      // And the status should be backlog
      expect(result.status).toBe('backlog');
    });
  });

  describe('Scenario: System-reminder after Example Mapping should prompt for feature file generation first', () => {
    it('should remind to generate scenarios before estimating', async () => {
      // Given I have a work unit in specifying state
      // And I have completed Example Mapping (rules, examples, questions answered)
      // And I have NOT generated scenarios yet
      const workUnits = {
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            title: 'Test work unit with Example Mapping',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            relationships: {},
            userStory: {
              role: 'user',
              action: 'do something',
              benefit: 'get value',
            },
            rules: ['Rule 1', 'Rule 2'],
            examples: ['Example 1', 'Example 2'],
            questions: [
              {
                text: 'Question 1?',
                selected: true,
                answer: 'Answer 1',
              },
            ],
          },
        },
        kanban: {
          backlog: [],
          specifying: ['TEST-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit <work-unit-id>"
      const result = await showWorkUnit({ workUnitId: 'TEST-002', cwd: testDir });

      // Then the system-reminder should say "After generating scenarios from Example Mapping, estimate based on feature file complexity"
      const reminders = result.systemReminders || [];
      const reminderText = reminders.join('\n');
      expect(reminderText).toContain('After generating scenarios from Example Mapping');
      expect(reminderText).toContain('estimate based on feature file complexity');

      // And the system-reminder should NOT say "Use Example Mapping results to estimate story points"
      expect(reminderText).not.toContain(
        'Use Example Mapping results to estimate story points'
      );
    });
  });

  describe('Scenario: ACDD violation error and system-reminder should align', () => {
    it('should show consistent messaging between error and reminder', async () => {
      // Given I have a work unit in specifying state
      // And I have completed Example Mapping
      // And I have NOT generated scenarios yet
      const workUnits = {
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            title: 'Test work unit',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            relationships: {},
            userStory: {
              role: 'user',
              action: 'do something',
              benefit: 'get value',
            },
            rules: ['Rule 1'],
            examples: ['Example 1'],
            questions: [],
          },
        },
        kanban: {
          backlog: [],
          specifying: ['TEST-003'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
        },
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I try to estimate with "fspec update-work-unit-estimate <work-unit-id> 3"
      let errorMessage = '';
      try {
        await updateWorkUnitEstimate({ workUnitId: 'TEST-003', estimate: 3, cwd: testDir });
      } catch (error: unknown) {
        const err = error as Error;
        errorMessage = err.message;
      }

      // Then the command should fail with an ACDD violation error
      expect(errorMessage).toBeTruthy();
      expect(errorMessage).toContain('ACDD VIOLATION');

      // And the error should say "Feature file required before estimation"
      expect(errorMessage).toContain('feature file');
      expect(errorMessage).toContain('estimation');

      // And the error should guide me to generate scenarios first
      expect(errorMessage).toContain('generate scenarios');
      expect(errorMessage).toContain('fspec generate-scenarios');
    });
  });

  describe('Scenario: Documentation should clarify estimation timing', () => {
    it('should explicitly state estimation happens after generating scenarios', async () => {
      // Given I read the spec/CLAUDE.md workflow documentation
      const projectRoot = process.cwd();
      const specClaudeMd = await readFile(
        join(projectRoot, 'spec/CLAUDE.md'),
        'utf-8'
      );

      // When I look for guidance on when to estimate story points
      const specEstimationSection = specClaudeMd.toLowerCase();

      // Then the documentation should explicitly state "After generating scenarios from Example Mapping"
      expect(
        specEstimationSection.includes('after generating scenarios') ||
          (specEstimationSection.includes('generate scenarios') &&
            specEstimationSection.includes('estimate'))
      ).toBe(true);

      // And the documentation should NOT say "After Example Mapping" without mentioning scenario generation
      // (This is trickier - we allow "After Example Mapping" as long as it's qualified with scenario generation context)
      const problematicPattern =
        /after example mapping.*estimate(?!.*generat)/i;
      expect(specClaudeMd.match(problematicPattern)).toBeFalsy();
    });
  });

  describe('Scenario: Slash command documentation should clarify estimation timing', () => {
    it('should show correct workflow order in Step 2.5', async () => {
      // Given I read the .claude/commands/fspec.md slash command documentation
      const projectRoot = process.cwd();
      const fspecMd = await readFile(
        join(projectRoot, '.claude/commands/fspec.md'),
        'utf-8'
      );

      // When I look at "Step 2.5: Story Point Estimation"
      const step25Match = fspecMd.match(
        /Step 2\.5[:\s]+Story Point Estimation[\s\S]{0,500}/i
      );
      expect(step25Match).toBeTruthy();

      const step25Section = step25Match![0];

      // Then it should explicitly state estimation happens AFTER generating scenarios
      expect(
        step25Section.toLowerCase().includes('after generating scenarios') ||
          (step25Section.toLowerCase().includes('generate scenarios') &&
            step25Section.toLowerCase().includes('estimate'))
      ).toBe(true);

      // And it should show the correct workflow order: Example Mapping → Generate Scenarios → Estimate
      expect(
        step25Section.toLowerCase().includes('example mapping') &&
          step25Section.toLowerCase().includes('generate') &&
          step25Section.toLowerCase().includes('estimate')
      ).toBe(true);
    });
  });
});
