/**
 * Feature: spec/features/update-claude-md-with-big-picture-event-storming-workflow-documentation.feature
 *
 * Tests for Big Picture Event Storm documentation section
 */

import { describe, it, expect } from 'vitest';
import { getBigPictureEventStormSection } from '../bigPictureEventStorm';

describe('Feature: Update CLAUDE.md with Big Picture Event Storming workflow documentation', () => {
  describe('Scenario: AI agent reads Step 1.5a documentation after foundation discovery', () => {
    it('should contain Step 1.5a section with proper structure and foundation commands', () => {
      // @step Given foundation discovery has been completed with discover-foundation --finalize
      // @step And a work unit exists in backlog prompting Big Picture Event Storming
      // (No setup needed - testing static documentation)

      // @step When AI agent runs fspec bootstrap to load context
      const section = getBigPictureEventStormSection();

      // @step Then CLAUDE.md should contain Step 1.5a section titled "Big Picture Event Storming (Foundation Level)"
      expect(section).toContain('Step 1.5a');
      expect(section).toContain('Big Picture Event Storming');
      expect(section).toContain('Foundation Level');

      // @step And the section should appear after Step 1.5 "Bootstrap Foundation"
      // (Section ordering tested in integration test - this is unit level)

      // @step And the section should appear before Step 1.6 "Event Storm - Work Unit Level"
      // (Section ordering tested in integration test - this is unit level)

      // @step And the section should explain when to conduct Big Picture Event Storming
      expect(section).toContain('when to conduct');
      expect(section).toContain('after foundation discovery');

      // @step And the section should list all foundation Event Storm commands with examples
      expect(section).toContain('add-foundation-bounded-context');
      expect(section).toContain('add-aggregate-to-foundation');
      expect(section).toContain('add-domain-event-to-foundation');
      expect(section).toContain('add-command-to-foundation');
      expect(section).toContain('show-foundation-event-storm');
    });
  });

  describe('Scenario: Developer distinguishes foundation vs work unit Event Storming commands', () => {
    it('should contain comparison table showing foundation vs work unit commands', () => {
      // @step Given CLAUDE.md has been updated with Step 1.5a
      const section = getBigPictureEventStormSection();

      // @step When developer reads the comparison table
      // Table should exist in the section
      expect(section).toContain('|'); // Markdown table delimiter

      // @step Then the table should show foundation Event Storm uses "add-foundation-bounded-context" command
      expect(section).toContain('add-foundation-bounded-context');

      // @step And the table should show work unit Event Storm uses "add-bounded-context <work-unit-id>" command
      expect(section).toContain('add-bounded-context');
      expect(section).toContain('<work-unit-id>');

      // @step And the table should compare scope (entire domain vs single feature)
      expect(section).toContain('scope');
      expect(section).toContain('entire domain');
      expect(section).toContain('single feature');

      // @step And the table should compare storage location (foundation.json vs work-units.json)
      expect(section).toContain('foundation.json');
      expect(section).toContain('work-units.json');

      // @step And the table should compare timing (once after foundation vs many times per story)
      expect(section).toContain('once');
      expect(section).toContain('many times');

      // @step And the table should compare output (tag ontology vs scenarios)
      expect(section).toContain('tag ontology');
      expect(section).toContain('scenarios');
    });
  });

  describe('Scenario: AI agent follows Big Picture Event Storm session flow guidance', () => {
    it('should explain workflow steps with example commands and conversations', () => {
      // @step Given CLAUDE.md Step 1.5a explains session flow
      const section = getBigPictureEventStormSection();

      // @step When AI agent reads the workflow steps
      // (Reading is implicit in test execution)

      // @step Then step 1 should explain identifying bounded contexts
      expect(section).toContain('Identify Bounded Contexts');
      expect(section).toContain('bounded context');

      // @step And step 2 should explain identifying aggregates per context
      expect(section).toContain('Identify Aggregates');
      expect(section).toContain('aggregate');

      // @step And step 3 should explain identifying domain events per context
      expect(section).toContain('Identify Domain Events');
      expect(section).toContain('domain event');

      // @step And step 4 should explain viewing and validating with show-foundation-event-storm
      expect(section).toContain('View and Validate');
      expect(section).toContain('show-foundation-event-storm');

      // @step And each step should include example commands
      expect(section).toContain('```bash');
      expect(section).toContain('fspec add-foundation');

      // @step And each step should include example human-AI conversation
      expect(section).toContain('AI:');
      expect(section).toContain('Human:');
    });
  });

  describe('Scenario: AI agent integrates Big Picture Event Storm with tag ontology', () => {
    it('should explain derive-tags-from-foundation command and EXMAP-004 integration', () => {
      // @step Given Big Picture Event Storm has been completed
      // @step And foundation.json eventStorm field is populated
      // (Preconditions implicit in documentation test)

      // @step When AI agent reads CLAUDE.md integration section
      const section = getBigPictureEventStormSection();

      // @step Then the section should explain running derive-tags-from-foundation command
      expect(section).toContain('derive-tags-from-foundation');

      // @step And the section should explain how component tags are generated from bounded contexts
      expect(section).toContain('component tags');
      expect(section).toContain('bounded context');
      expect(section).toContain('generated');

      // @step And the section should reference EXMAP-004 work unit
      expect(section).toContain('EXMAP-004');

      // @step And the section should show expected output (number of tags and relationships created)
      expect(section).toContain('Created');
      expect(section).toContain('tags');
      expect(section).toContain('relationships');
    });
  });
});
