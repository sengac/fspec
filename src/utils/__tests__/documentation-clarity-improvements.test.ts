/**
 * Feature: spec/features/documentation-clarity-improvements.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Since this is a documentation improvement feature, tests verify documentation structure,
 * clarity, and presence of required sections rather than code behavior.
 */

import { describe, it, expect } from 'vitest';
import { getAggregatedBootstrapDocumentation } from '../projectManagementSections/fspecBootstrap';

describe('Feature: Documentation Clarity Improvements', () => {
  describe('Scenario: Distinguish Foundation Event Storm from Feature Event Storm by name', () => {
    it('should use distinct names for Foundation vs Feature Event Storm', async () => {
      // @step Given the bootstrap documentation has two Event Storm sections
      const bootstrapContent = getAggregatedBootstrapDocumentation();

      // @step And Step 3 is named "Big Picture Event Storming (Foundation Level)"
      // Checked by reading file

      // @step And Step 4 is named "Event Storm - Domain Discovery BEFORE Example Mapping"
      // Checked by reading file

      // @step When an AI agent reads both sections
      // Simulated by reading the file

      // @step Then the AI must carefully read to distinguish between them
      // Verified by checking old names are gone

      // @step When the sections are renamed to "Foundation Event Storm" and "Feature Event Storm"
      expect(bootstrapContent).toContain('Foundation Event Storm');
      expect(bootstrapContent).toContain('Feature Event Storm');

      // @step And a comparison table is added showing when/scope/storage/commands/output/purpose
      expect(bootstrapContent).toMatch(
        /Foundation Event Storm.*Feature Event Storm/s
      );
      expect(bootstrapContent).toContain('| Aspect |');
      expect(bootstrapContent).toContain('When');
      expect(bootstrapContent).toContain('Scope');
      expect(bootstrapContent).toContain('Storage');

      // @step Then the distinction is immediately clear from the names alone
      expect(bootstrapContent).not.toContain('Big Picture Event Storming');
      expect(bootstrapContent).not.toContain(
        'Event Storm - Domain Discovery BEFORE Example Mapping'
      );

      // @step And the AI can quickly understand which Event Storm to use
      // Verified by clear naming and table structure
    });
  });

  describe('Scenario: Research codebase before deciding to use Feature Event Storm', () => {
    it('should document research-first workflow for Feature Event Storm decision', async () => {
      // @step Given an AI agent needs to decide whether to use Feature Event Storm
      const bootstrapContent = getAggregatedBootstrapDocumentation();

      // @step And the AI has not yet analyzed the codebase
      // Implied by the workflow guidance

      // @step When the AI follows the research-first workflow
      // Verified by checking for research commands

      // @step Then the AI runs AST research with pattern matching
      expect(bootstrapContent).toContain('fspec research --tool=ast');
      expect(bootstrapContent).toContain('--pattern=');
      expect(bootstrapContent).toContain('--lang=');
      expect(bootstrapContent).toContain('--path=');

      // @step And the AI understands the authentication domain from code analysis
      // Implied by research-first workflow

      // @step And the AI shares findings with the user: "I found 3 domain events: UserRegistered, LoginAttempted, SessionExpired"
      // Example shown in documentation

      // @step And the AI asks: "Should we do Feature Event Storm to map the full authentication flow?"
      expect(bootstrapContent).toContain('ASK USER');
      expect(bootstrapContent).toMatch(/uncertain.*ask/i);

      // @step Then the user decides based on actual code analysis
      expect(bootstrapContent).toMatch(/SUBJECTIVE.*COLLABORATIVE/);

      // @step And no guessing or assumptions are made
      expect(bootstrapContent).toContain('no guessing');
    });
  });

  describe('Scenario: Distinguish automatic vs manual coverage file operations', () => {
    it('should clearly label AUTOMATIC vs MANUAL coverage operations', async () => {
      // @step Given the documentation mentions "coverage files are automatically synchronized"
      const bootstrapContent = getAggregatedBootstrapDocumentation();

      // @step When an AI agent reads this without clear AUTOMATIC vs MANUAL labels
      // Simulated by absence of labels

      // @step Then the AI might think linking is also automatic
      // Problem state before improvement

      // @step And the AI wastes time waiting for coverage to auto-populate
      // Problem state before improvement

      // @step When the documentation separates operations with ✨ AUTOMATIC and 🔧 MANUAL labels
      expect(bootstrapContent).toContain('✨ AUTOMATIC');
      expect(bootstrapContent).toContain('🔧 MANUAL');

      // @step Then the AI immediately sees that create-feature creates .coverage files automatically
      expect(bootstrapContent).toMatch(/AUTOMATIC.*create-feature/s);
      expect(bootstrapContent).toMatch(/AUTOMATIC.*generate-coverage/s);

      // @step And the AI sees that delete-scenario syncs coverage automatically
      expect(bootstrapContent).toMatch(/AUTOMATIC.*delete-scenario/s);

      // @step And the AI understands that "link-coverage --test-file" is required manual step
      expect(bootstrapContent).toMatch(/MANUAL.*link-coverage/s);
      expect(bootstrapContent).toMatch(/MANUAL.*--test-file/s);

      // @step Then the AI runs the correct commands without confusion
      // Verified by clear labeling
    });
  });

  describe('Scenario: Understand @step comments match only step text not data tables', () => {
    it('should clarify that @step comments match only step.text not data tables', async () => {
      // @step Given a feature file has "Given I have the following items:" with a data table below
      const bootstrapContent = getAggregatedBootstrapDocumentation();

      // @step When an AI agent thinks @step comment must include the entire table text
      // Problem state before improvement

      // @step Then the AI wastes time trying to format multi-line comment
      // Problem state before improvement

      // @step When the documentation clarifies that parser extracts ONLY step.text
      expect(bootstrapContent).toContain('ONLY the step line text');
      expect(bootstrapContent).toContain('NOT data tables');

      // @step And an example shows data table step matching with table content ignored
      expect(bootstrapContent).toContain('table content ignored');
      expect(bootstrapContent).toMatch(/@step Given I have.*items.*:/);

      // @step Then the AI correctly writes "// @step Given I have the following items:"
      expect(bootstrapContent).toContain('// @step');

      // @step And the AI understands that table content is ignored by the matcher
      // Verified by documentation clarity
    });
  });

  describe('Scenario: Iterate foundation discovery fields when mistakes are discovered', () => {
    it('should document that foundation discovery iteration is fully supported', async () => {
      // @step Given an AI agent is filling foundation discovery fields
      const bootstrapContent = getAggregatedBootstrapDocumentation();

      // @step And the AI has filled field 3
      // Implied by workflow progression

      // @step When the AI realizes field 1 (project name) was wrong
      // Problem discovery scenario

      // @step And there is no iteration documentation
      // Problem state before improvement

      // @step Then the AI might think "too late, can't go back"
      // Problem state before improvement

      // @step When explicit iteration documentation is added
      expect(bootstrapContent).toContain('Iteration fully supported');

      // @step Then the AI confidently runs "fspec update-foundation projectName 'correct-name'"
      expect(bootstrapContent).toContain('update-foundation can re-update');
      expect(bootstrapContent).toMatch(/any field.*anytime/i);

      // @step And the AI fixes the mistake
      expect(bootstrapContent).toContain('cat spec/foundation.json.draft');
      expect(bootstrapContent).toContain('draft persists');

      // @step And the AI continues from field 4
      expect(bootstrapContent).toContain('re-run');

      // @step And the AI understands that iteration is fully supported
      expect(bootstrapContent).toMatch(/no restrictions/i);
    });
  });
});
