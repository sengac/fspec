@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Require archival admission before inference
  """
  Uses the shared bounded ordered recovery owner defined in
  recoverable-conversation-persistence.feature. Diagnostic failures never become
  model context, provider prompts or fabricated user messages.
  """

  Scenario: User persistence failure prevents inference and retains the input
    Given a real session with an unavailable message archive
    When user input is submitted to the agent loop
    Then a scoped persistence error is visible and no provider request starts
    And repairing storage and explicit recovery archives the original input once
