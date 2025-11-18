@cli
@documentation-management
@documentation
@bootstrap
@high
@ai-guidance
@DOC-016
Feature: Documentation Clarity Improvements

  """
  This feature improves fspec bootstrap documentation clarity based on AI feedback from real-world usage. Implementation involves updating markdown files in the bootstrap command output, not code changes. Focus is on documentation structure, naming conventions, and workflow guidance to prevent AI confusion during fspec usage.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rename to 'Foundation Event Storm' and 'Feature Event Storm' for self-documenting clarity. Add comparison table showing when/scope/storage/commands/output/purpose differences. Foundation aligns with foundation.json, Feature clearly indicates per-story recurring use.
  #   2. Make Feature Event Storm decision interactive and research-first: 1) Research codebase using 'fspec research --tool=ast --files "path/to/files"' FIRST to understand domain, 2) Self-assess based on research findings, 3) If uncertain after research, ASK USER (share findings, let them decide), 4) Proceed with chosen approach. Decision is SUBJECTIVE and COLLABORATIVE - emphasize no guessing, always ask if unsure. Research builds familiarity before judgment.
  #   3. Add 'Coverage File Lifecycle' section that explicitly separates AUTOMATIC vs MANUAL operations with clear visual labels (✨ AUTOMATIC / 🔧 MANUAL). AUTOMATIC: creation (create-feature, generate-coverage), synchronization (delete-scenario, update-scenario auto-updates coverage), validation (update-work-unit-status blocks if stale). MANUAL: linking tests (link-coverage --test-file), linking implementation (link-coverage --impl-file), manual re-sync if needed. This prevents confusion between auto-sync and manual linking.
  #   4. After researching the code, these 'edge cases' don't actually exist\! Feature parser extracts ONLY step.text (not data tables/docstrings). Matching uses hybrid fuzzy similarity with adaptive thresholds (0.70-0.85 based on length), whitespace normalized. Documentation should clarify: @step comments match ONLY the step line text (e.g., 'Given I have items:' NOT the table below). Add simple example showing data table step matching (table content ignored). No need for complex edge case documentation - the implementation is simpler than it appears.
  #   5. Document foundation discovery iteration/recovery capabilities explicitly: 1) Iteration fully supported - update-foundation can re-update any field anytime with no restrictions, 2) Review draft with 'cat spec/foundation.json.draft' (no dedicated command yet), 3) Validation failure recovery - draft persists on --finalize failure, fix errors and re-run, draft only deleted on success, 4) Manual edit protection - direct file edits detected and reverted (CLI-only enforcement). Add clear 'Foundation Discovery Iteration' section explaining these capabilities.
  #
  # EXAMPLES:
  #   1. In bootstrap docs, Step 3 is 'Big Picture Event Storming (Foundation Level)' and Step 4 is 'Event Storm - Domain Discovery BEFORE Example Mapping'. AI reads both and has to carefully distinguish them. After renaming to 'Foundation Event Storm' and 'Feature Event Storm' with comparison table, the distinction is immediately clear from names alone.
  #   2. AI is about to do Feature Event Storm but hasn't seen the codebase yet. Instead of guessing, AI first runs 'fspec research --tool=ast --files src/auth/*.ts' to understand authentication domain. Then shares findings: 'I found 3 domain events: UserRegistered, LoginAttempted, SessionExpired. Should we do Feature Event Storm to map the full authentication flow?' Human decides based on actual code analysis.
  #   3. AI reads 'coverage files are automatically synchronized' and thinks linking is also automatic. Wastes time waiting for coverage to auto-populate. With ✨ AUTOMATIC vs 🔧 MANUAL labels, AI immediately sees: ✨ AUTOMATIC: create-feature creates .coverage files, delete-scenario syncs coverage. 🔧 MANUAL: link-coverage --test-file required to link tests. AI runs the right commands without confusion.
  #   4. Feature file has 'Given I have the following items:' with a data table below. AI thinks @step comment must include the entire table text. Wastes time trying to format multi-line comment. With clarification that parser extracts ONLY step.text, AI correctly writes: // @step Given I have the following items: (table content ignored by matcher).
  #   5. AI fills field 3 of foundation discovery, then realizes field 1 (project name) was wrong. Without iteration docs, AI might think 'too late, can't go back'. With explicit iteration documentation, AI confidently runs 'fspec update-foundation projectName "correct-name"' to fix the mistake, then continues from field 4.
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should we address the Event Storm naming confusion? Should we rename 'Big Picture Event Storming' and 'Event Storm - Domain Discovery' to make them more distinct (e.g., 'Strategic Event Storm' vs 'Tactical Event Storm'), or would adding a comparison table early in the docs be better?
  #   A: true
  #
  #   Q: For deciding when to use Feature Event Storm vs skip to Example Mapping - the current criteria are somewhat subjective ('complex domain', 'many entities'). Should we provide a clearer default heuristic, like 'When in doubt for stories >5 points, do Feature Event Storm' or 'Always do Feature Event Storm for new domains, skip for familiar ones'?
  #   A: true
  #
  #   Q: The documentation mentions coverage files track scenario-to-test-to-implementation mappings, and there are references to 'automatic coverage synchronization' in git commits. But the lifecycle of coverage files isn't fully clear - when are they auto-generated vs manual? What triggers synchronization? Should we add a 'Coverage File Lifecycle' section explaining generation, linking, synchronization, and auditing?
  #   A: true
  #
  #   Q: For @step comment matching (used by link-coverage validation), should we document the exact matching rules for edge cases like: multi-line Gherkin steps, steps with data tables, steps with docstrings, case sensitivity, and whitespace handling? Or are these edge cases rare enough that we can wait for them to come up organically?
  #   A: true
  #
  #   Q: For foundation discovery (discover-foundation workflow), can AI re-run 'fspec update-foundation' on previously filled fields to fix mistakes? Is there a way to review the draft state before finalization? Should we document the error recovery process if validation fails at finalization?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec
    I want to understand all bootstrap documentation without confusion
    So that I can confidently use fspec commands and workflows in the correct context

  Scenario: Distinguish Foundation Event Storm from Feature Event Storm by name
    Given the bootstrap documentation has two Event Storm sections
    And Step 3 is named "Big Picture Event Storming (Foundation Level)"
    And Step 4 is named "Event Storm - Domain Discovery BEFORE Example Mapping"
    When an AI agent reads both sections
    Then the AI must carefully read to distinguish between them
    When the sections are renamed to "Foundation Event Storm" and "Feature Event Storm"
    And a comparison table is added showing when/scope/storage/commands/output/purpose
    Then the distinction is immediately clear from the names alone
    And the AI can quickly understand which Event Storm to use

  Scenario: Research codebase before deciding to use Feature Event Storm
    Given an AI agent needs to decide whether to use Feature Event Storm
    And the AI has not yet analyzed the codebase
    When the AI follows the research-first workflow
    Then the AI runs "fspec research --tool=ast --files src/auth/*.ts"
    And the AI understands the authentication domain from code analysis
    And the AI shares findings with the user: "I found 3 domain events: UserRegistered, LoginAttempted, SessionExpired"
    And the AI asks: "Should we do Feature Event Storm to map the full authentication flow?"
    Then the user decides based on actual code analysis
    And no guessing or assumptions are made

  Scenario: Distinguish automatic vs manual coverage file operations
    Given the documentation mentions "coverage files are automatically synchronized"
    When an AI agent reads this without clear AUTOMATIC vs MANUAL labels
    Then the AI might think linking is also automatic
    And the AI wastes time waiting for coverage to auto-populate
    When the documentation separates operations with ✨ AUTOMATIC and 🔧 MANUAL labels
    Then the AI immediately sees that create-feature creates .coverage files automatically
    And the AI sees that delete-scenario syncs coverage automatically
    And the AI understands that "link-coverage --test-file" is required manual step
    Then the AI runs the correct commands without confusion

  Scenario: Understand @step comments match only step text not data tables
    Given a feature file has "Given I have the following items:" with a data table below
    When an AI agent thinks @step comment must include the entire table text
    Then the AI wastes time trying to format multi-line comment
    When the documentation clarifies that parser extracts ONLY step.text
    And an example shows data table step matching with table content ignored
    Then the AI correctly writes "// @step Given I have the following items:"
    And the AI understands that table content is ignored by the matcher

  Scenario: Iterate foundation discovery fields when mistakes are discovered
    Given an AI agent is filling foundation discovery fields
    And the AI has filled field 3
    When the AI realizes field 1 (project name) was wrong
    And there is no iteration documentation
    Then the AI might think "too late, can't go back"
    When explicit iteration documentation is added
    Then the AI confidently runs "fspec update-foundation projectName 'correct-name'"
    And the AI fixes the mistake
    And the AI continues from field 4
    And the AI understands that iteration is fully supported
