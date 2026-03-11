@CMPCT-014
Feature: Esc to stop does not interrupt compaction — isCompacting state bypasses interrupt handler

  """
  Fix is in AgentView.tsx Priority 5 Esc handler: change condition from `displayIsLoading && currentSessionId` to `(displayIsLoading || compaction.state.isActive) && currentSessionId`
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Esc handler in AgentView.tsx Priority 5 must check compaction.state.isActive in addition to displayIsLoading
  #   2. When compaction is active, Esc must call sessionInterrupt() to stop the agent — same behavior as normal loading
  #   3. Esc priority ordering must remain: modals > select mode > interrupt > clear input > exit confirmation
  #
  # EXAMPLES:
  #   1. Compaction is active (isCompacting=true, isLoading=false), user presses Esc → agent is interrupted, compaction stops
  #   2. Both compaction and loading are active (isCompacting=true, isLoading=true), user presses Esc → agent is interrupted
  #   3. Compaction is active but turn modal is open, user presses Esc → modal closes first (priority ordering preserved)
  #   4. Neither compaction nor loading active, user presses Esc with text in input → input clears (existing behavior unchanged)
  #
  # ========================================

  Background: User Story
    As a user
    I want to press Esc during compaction to stop the agent
    So that I can interrupt compaction just like I can interrupt normal loading

  Scenario: Esc interrupts compaction when only compaction is active
    Given the agent is compacting context
    And the agent is not in a loading state
    When I press Esc
    Then the agent should be interrupted
    And the compaction should stop

  Scenario: Esc interrupts when both compaction and loading are active
    Given the agent is compacting context
    And the agent is also in a loading state
    When I press Esc
    Then the agent should be interrupted

  Scenario: Esc closes modal before interrupting compaction
    Given the agent is compacting context
    And a turn modal is open
    When I press Esc
    Then the turn modal should close
    And the compaction should continue running

  Scenario: Esc clears input when neither compaction nor loading is active
    Given the agent is idle
    And I have text in the input field
    When I press Esc
    Then the input field should be cleared
    And no session interrupt should occur
