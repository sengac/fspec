@done
@templates
@high
@foundation-management
@documentation
@FOUND-014
Feature: Update CLAUDE.md with Big Picture Event Storming workflow documentation
  """
  This is a documentation-only feature modifying spec/CLAUDE.md. No code changes, only markdown content updates. Section will be inserted between existing Step 1.5 (Bootstrap Foundation) and Step 1.6 (renumbered from 1.5). Must include comparison table, workflow steps, command examples, and EXMAP-004 integration explanation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New section must be added to CLAUDE.md after Step 1.5 (Bootstrap Foundation) and before Step 1.6 (Event Storm - Work Unit Level)
  #   2. Section must explain difference between Big Picture (foundation-level) and Process Modeling (work unit-level) Event Storming
  #   3. All foundation Event Storm commands must be documented with examples (add-foundation-bounded-context, add-aggregate-to-foundation, add-domain-event-to-foundation, add-command-to-foundation, show-foundation-event-storm)
  #   4. Section must include table comparing foundation vs work unit Event Storming (scope, storage, commands, timing, output)
  #   5. Section must explain integration with EXMAP-004 tag ontology generation (derive-tags-from-foundation command)
  #   6. Existing Step 1.5 (Event Storm) must be renumbered to Step 1.6 to make room for new Step 1.5a (Big Picture Event Storm)
  #
  # EXAMPLES:
  #   1. AI agent completes discover-foundation --finalize, sees work unit FOUND-XXX in backlog, reads CLAUDE.md Step 1.5a, understands to conduct Big Picture Event Storm using foundation commands
  #   2. Developer reads CLAUDE.md and sees clear table showing foundation Event Storm uses add-foundation-bounded-context while work unit Event Storm uses add-bounded-context with work unit ID
  #   3. AI agent reads section explaining Big Picture Event Storm session flow, asks human for bounded contexts, uses add-foundation-bounded-context command, then populates aggregates and events per context
  #   4. AI agent completes Big Picture Event Storm, reads CLAUDE.md integration section, runs derive-tags-from-foundation command to generate component tags from bounded contexts (EXMAP-004)
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to understand when and how to conduct Big Picture Event Storming after foundation discovery
    So that I can properly capture bounded contexts and domain architecture in foundation.json eventStorm field

  Scenario: AI agent reads Step 1.5a documentation after foundation discovery
    Given foundation discovery has been completed with discover-foundation --finalize
    And a work unit exists in backlog prompting Big Picture Event Storming
    When AI agent runs fspec bootstrap to load context
    Then CLAUDE.md should contain Step 1.5a section titled "Big Picture Event Storming (Foundation Level)"
    And the section should appear after Step 1.5 "Bootstrap Foundation"
    And the section should appear before Step 1.6 "Event Storm - Work Unit Level"
    And the section should explain when to conduct Big Picture Event Storming
    And the section should list all foundation Event Storm commands with examples

  Scenario: Developer distinguishes foundation vs work unit Event Storming commands
    Given CLAUDE.md has been updated with Step 1.5a
    When developer reads the comparison table
    Then the table should show foundation Event Storm uses "add-foundation-bounded-context" command
    And the table should show work unit Event Storm uses "add-bounded-context <work-unit-id>" command
    And the table should compare scope (entire domain vs single feature)
    And the table should compare storage location (foundation.json vs work-units.json)
    And the table should compare timing (once after foundation vs many times per story)
    And the table should compare output (tag ontology vs scenarios)

  Scenario: AI agent follows Big Picture Event Storm session flow guidance
    Given CLAUDE.md Step 1.5a explains session flow
    When AI agent reads the workflow steps
    Then step 1 should explain identifying bounded contexts
    And step 2 should explain identifying aggregates per context
    And step 3 should explain identifying domain events per context
    And step 4 should explain viewing and validating with show-foundation-event-storm
    And each step should include example commands
    And each step should include example human-AI conversation

  Scenario: AI agent integrates Big Picture Event Storm with tag ontology
    Given Big Picture Event Storm has been completed
    And foundation.json eventStorm field is populated
    When AI agent reads CLAUDE.md integration section
    Then the section should explain running derive-tags-from-foundation command
    And the section should explain how component tags are generated from bounded contexts
    And the section should reference EXMAP-004 work unit
    And the section should show expected output (number of tags and relationships created)
