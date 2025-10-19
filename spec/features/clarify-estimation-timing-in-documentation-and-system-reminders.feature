@done
@workflow
@documentation
@cli
@phase1
@DOCS-001
Feature: Clarify estimation timing in documentation and system-reminders

  """
  Files to check: src/commands/show-work-unit.ts (system-reminder logic), src/commands/update-work-unit-status.ts (state transition reminders), CLAUDE.md (workflow documentation), .claude/commands/fspec.md (slash command docs)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Story point estimation happens AFTER Example Mapping, during specifying state
  #   2. Cannot estimate accurately without understanding scope from Example Mapping
  #   3. System-reminders in backlog state must NOT suggest adding estimates
  #   4. System-reminders in specifying state SHOULD remind to estimate after Example Mapping
  #
  # EXAMPLES:
  #   1. AI shows MCP epic with 5 work units in backlog, suggests adding estimates before moving to specifying (WRONG)
  #   2. AI moves work unit to specifying, does Example Mapping, THEN suggests estimate based on discovered complexity (CORRECT)
  #   3. AI reads show-work-unit output showing 'no estimate' reminder in backlog state, incorrectly offers to add estimate immediately (WRONG)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Are there other places in the codebase besides system-reminders where estimation timing should be clarified?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec
    I want to understand when to estimate story points
    So that I follow ACDD workflow correctly without suggesting premature estimation

  Scenario: System-reminder in backlog state should not suggest adding estimates
    Given I have a work unit in backlog state with no estimate
    When I run "fspec show-work-unit <work-unit-id>"
    Then the system-reminder should NOT suggest adding an estimate
    And the system-reminder should NOT display "Use Example Mapping results to estimate story points"
    And the output should indicate estimates happen during specifying phase

  Scenario: System-reminder after Example Mapping should prompt for feature file generation first
    Given I have a work unit in specifying state
    And I have completed Example Mapping (rules, examples, questions answered)
    And I have NOT generated scenarios yet
    When I run "fspec show-work-unit <work-unit-id>"
    Then the system-reminder should say "After generating scenarios from Example Mapping, estimate based on feature file complexity"
    And the system-reminder should NOT say "Use Example Mapping results to estimate story points"

  Scenario: ACDD violation error and system-reminder should align
    Given I have a work unit in specifying state
    And I have completed Example Mapping
    And I have NOT generated scenarios yet
    When I try to estimate with "fspec update-work-unit-estimate <work-unit-id> 3"
    Then the command should fail with an ACDD violation error
    And the error should say "Feature file required before estimation"
    And the error should guide me to generate scenarios first
    And the system-reminder text should match the enforcement message

  Scenario: Documentation should clarify estimation timing
    Given I read the CLAUDE.md workflow documentation
    When I look for guidance on when to estimate story points
    Then the documentation should explicitly state "After generating scenarios from Example Mapping"
    And the documentation should NOT say "After Example Mapping" without mentioning scenario generation

  Scenario: Slash command documentation should clarify estimation timing
    Given I read the .claude/commands/fspec.md slash command documentation
    When I look at "Step 2.5: Story Point Estimation"
    Then it should explicitly state estimation happens AFTER generating scenarios
    And it should show the correct workflow order: Example Mapping → Generate Scenarios → Estimate
