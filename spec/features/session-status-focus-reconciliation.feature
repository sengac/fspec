@done
@BUG-231
@session
@tool-integration
@session-management
@rust
Feature: Successful work-unit transitions reconcile session activity
  """
  Reconcile at the shared session-aware Fspec execution boundary after success,
  while keeping fspec-core command business logic sessionless. Explicit tool
  attribution wins over inferred command-target focus. Watcher snapshots may
  refresh or clear existing focus but never infer the issuing session.
  Completed and deleted cards clear current activity, not authorization binding.
  Explicit later follow-up activity on an already completed card is permitted.
  """

  Background: User Story
    As a developer supervising concurrent agents
    I want session activity to follow successful work-unit transitions
    So that completed work and failed commands do not leave misleading card tracking

  Scenario: A successful legacy agent status command switches the issuing session
    Given two sessions with different card activity
    When the background session successfully updates another card without explicit attribution
    Then only that issuing session switches to the updated card
    And its environment reminder and current card status agree

  Scenario: A failed status command does not imply a successful focus switch
    Given a session working on card "AUTH-001"
    When its attempted transition for card "AUTH-002" fails
    Then its activity remains on "AUTH-001"
    And no focus change event is emitted for that failed command

  Scenario: Same-card status refresh retains correct authorization stage
    Given a session with activity and authorization bound to card "AUTH-001"
    When a successful status update changes that card's stage
    Then the authoritative activity contains the current stage
    And existing stage checks use the current stage even after activity is cleared

  Scenario: External completion or deletion clears only sessions focused on that card
    Given two sessions focused on "AUTH-001" and another on "AUTH-002"
    When a real work-unit snapshot marks "AUTH-001" completed or removes it
    Then both sessions on "AUTH-001" remain open with cleared activity
    And the session on "AUTH-002" remains unchanged
    And permission bindings are not relaxed by clearing activity

  Scenario: Explicit attribution takes precedence over administrative command targets
    Given a session with explicit tool attribution for "AUTH-001" or "NONE"
    When it successfully updates another card's status
    Then the command target does not override its explicit activity attribution
    And completing an unrelated card does not clear its current activity

  Scenario: Late completion cannot erase newer activity or forbid later follow-up work
    Given a session that has moved to newer activity after starting a card transition
    When the older card's completion is reconciled
    Then the newer activity is preserved
    And an explicitly attributed follow-up call may select an already completed card

  Scenario: Sessionless commands refresh metadata without claiming another card
    Given sessions focused on different cards
    When a shell user updates an unrelated card without a session identity
    Then no session switches to that card
    And updates to an already focused card still refresh its metadata

  Scenario: Watcher reconciliation updates a busy session outside the runtime
    Given a focused session whose agent lock is busy
    When a watcher thread outside Tokio observes completion
    Then focus persistence and events complete without waiting for the agent lock
    And the deferred environment reminder reflects cleared activity

  Scenario: Watcher replacement and lag recovery retain reconciliation
    Given a session service with a subscribed work-unit watcher
    When its watcher is replaced and notifications are missed
    Then authoritative resynchronization clears completed activity
    And stopped subscriptions cannot mutate newer activity
