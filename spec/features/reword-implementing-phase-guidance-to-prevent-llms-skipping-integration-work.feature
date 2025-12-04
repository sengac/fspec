@done
@cli
@workflow
@system-reminder
@high
@REMIND-015
Feature: Reword IMPLEMENTING phase guidance to prevent LLMs skipping integration work

  """
  Modifies src/utils/system-reminder.ts getStatusChangeReminder function. Changes IMPLEMENTING phase reminder (lines 164-187) to emphasize CREATION + CONNECTION pattern. Also updates specifyingStateReminder to add WHO CALLS THIS prompt. Uses system-reminder anti-drift pattern for LLM guidance.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. IMPLEMENTING phase guidance must emphasize CREATION + CONNECTION, not just making tests pass
  #   2. Guidance must include WHO CALLS THIS heuristic to prompt integration thinking
  #   3. Guidance must explicitly state INTEGRATION IS NOT SCOPE CREEP
  #   4. Remove minimization-encouraging phrases: ONLY enough, minimum, minimal, avoid over-implementation
  #   5. Suggested next steps must include integration verification steps
  #   6. SPECIFYING phase guidance must include WHO CALLS THIS prompt to identify integration points during Example Mapping
  #
  # EXAMPLES:
  #   1. When LLM enters IMPLEMENTING phase, guidance shows IMPLEMENTATION = CREATION + CONNECTION prominently
  #   2. Guidance asks For every piece of code you write, ask: WHO CALLS THIS?
  #   3. Guidance lists COMPLETE MEANS checklist: unit tests pass, integration tests written, imports added, call sites connected, feature works end-to-end
  #   4. Guidance explicitly separates STAY IN SCOPE (dont add unrelated features) from INTEGRATION IS NOT SCOPE CREEP (wiring is required)
  #   5. Suggested next steps include: write integration tests, wire up integration points, verify feature works end-to-end
  #   6. SPECIFYING phase asks: WHO CALLS THIS? Does this feature need to be wired into other parts of the system?
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the SPECIFYING phase also be updated to prompt LLMs to think about integration scenarios during Example Mapping?
  #   A: Yes, include SPECIFYING phase updates in this work unit to prompt integration thinking during Example Mapping
  #
  # ========================================

  Background: User Story
    As a AI coding agent using fspec
    I want to receive clear guidance during IMPLEMENTING phase that emphasizes integration work
    So that I complete features end-to-end instead of creating isolated modules that exist but aren't connected

  Scenario: IMPLEMENTING phase shows CREATION + CONNECTION prominently
    Given a work unit transitions to IMPLEMENTING status
    When the status change reminder is generated
    Then the reminder should contain "IMPLEMENTATION = CREATION + CONNECTION"


  Scenario: IMPLEMENTING phase includes WHO CALLS THIS heuristic
    Given a work unit transitions to IMPLEMENTING status
    When the status change reminder is generated
    Then the reminder should contain "WHO CALLS THIS?"


  Scenario: IMPLEMENTING phase lists COMPLETE MEANS checklist
    Given a work unit transitions to IMPLEMENTING status
    When the status change reminder is generated
    Then the reminder should contain "COMPLETE MEANS:"
    And the reminder should contain "Unit tests pass"
    And the reminder should contain "Call sites connected"
    And the reminder should contain "Feature works end-to-end"


  Scenario: IMPLEMENTING phase separates STAY IN SCOPE from INTEGRATION IS NOT SCOPE CREEP
    Given a work unit transitions to IMPLEMENTING status
    When the status change reminder is generated
    Then the reminder should contain "STAY IN SCOPE"
    And the reminder should contain "INTEGRATION IS NOT SCOPE CREEP"


  Scenario: IMPLEMENTING phase removes minimization-encouraging phrases
    Given a work unit transitions to IMPLEMENTING status
    When the status change reminder is generated
    Then the reminder should not contain "ONLY enough"
    And the reminder should not contain "minimum code"
    And the reminder should not contain "minimal code"
    And the reminder should not contain "Avoid over-implementation"


  Scenario: IMPLEMENTING phase next steps include integration verification
    Given a work unit transitions to IMPLEMENTING status
    When the status change reminder is generated
    Then the suggested next steps should include wiring up integration points
    And the suggested next steps should include verifying feature works end-to-end


  Scenario: SPECIFYING phase includes WHO CALLS THIS prompt for integration points
    Given a work unit transitions to SPECIFYING status
    When the status change reminder is generated
    Then the reminder should contain "WHO CALLS THIS?"
    And the reminder should prompt for integration scenarios

