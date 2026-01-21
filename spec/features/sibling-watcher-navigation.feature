@navigation
@tui
@WATCH-013
Feature: Sibling Watcher Navigation
  """
  Modify switchToSession() in AgentView.tsx to filter sessions by parent when current session is a watcher. Uses sessionGetParent() NAPI binding (WATCH-007) to determine if current session is a watcher and get its parent ID. When current session is a watcher, sessionManagerList() results are filtered to only include sessions where sessionGetParent() returns the same parent ID.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When in a watcher session (sessionGetParent returns non-null), Shift+Left/Right navigates only between sibling watchers that share the same parent
  #   2. When in a regular session (non-watcher, sessionGetParent returns null), Shift+Left/Right navigates through all top-level sessions (existing behavior unchanged)
  #   3. Sibling watchers are defined as sessions where sessionGetParent(sessionId) returns the same parent ID
  #   4. If only one sibling watcher exists, Shift+Left/Right does nothing (need at least 2 siblings to navigate)
  #   5. Navigation wraps around: from last sibling, Shift+Right goes to first sibling; from first sibling, Shift+Left goes to last sibling
  #   6. The switchToSession function filters sessionManagerList() to only include sessions where sessionGetParent returns the current session's parent ID
  #
  # EXAMPLES:
  #   1. Parent session A has three watchers: W1, W2, W3. User is in W1 and presses Shift+Right → switches to W2. Presses Shift+Right again → switches to W3. Presses Shift+Right again → wraps to W1.
  #   2. User is in regular session B (not a watcher). Presses Shift+Right → navigates through all sessions (A, B, C, etc.) as before, including both parent sessions and watcher sessions.
  #   3. Parent session A has only one watcher W1. User is in W1 and presses Shift+Right → nothing happens, stays in W1 (no siblings to navigate to).
  #   4. Parent session A has two watchers: W1 and W2. User is in W1 and presses Shift+Left → wraps to W2. Presses Shift+Left again → wraps to W1.
  #   5. Two parent sessions exist: Parent A (watchers W1, W2) and Parent B (watchers W3, W4). User in W1 presses Shift+Right → switches to W2 only (not W3 or W4 since they belong to Parent B).
  #
  # ========================================
  Background: User Story
    As a user in a watcher session
    I want to navigate between sibling watchers using Shift+Left/Right
    So that I can quickly switch between watchers of the same parent without cycling through unrelated sessions

  Scenario: Navigate forward through sibling watchers
    Given a parent session "Main Dev" exists
    And three watcher sessions exist for "Main Dev": "W1", "W2", "W3"
    And I am viewing watcher session "W1"
    When I press Shift+Right
    Then I should be viewing watcher session "W2"
    When I press Shift+Right
    Then I should be viewing watcher session "W3"
    When I press Shift+Right
    Then I should be viewing watcher session "W1"

  Scenario: Regular session navigates through all sessions
    Given three sessions exist: "Session A", "Session B", "Session C"
    And none of the sessions are watchers
    And I am viewing session "Session B"
    When I press Shift+Right
    Then I should be viewing session "Session C"
    When I press Shift+Right
    Then I should be viewing session "Session A"

  Scenario: Single watcher has no siblings to navigate
    Given a parent session "Main Dev" exists
    And one watcher session "W1" exists for "Main Dev"
    And I am viewing watcher session "W1"
    When I press Shift+Right
    Then I should remain viewing watcher session "W1"

  Scenario: Navigate backward through sibling watchers
    Given a parent session "Main Dev" exists
    And two watcher sessions exist for "Main Dev": "W1", "W2"
    And I am viewing watcher session "W1"
    When I press Shift+Left
    Then I should be viewing watcher session "W2"
    When I press Shift+Left
    Then I should be viewing watcher session "W1"

  Scenario: Watchers of different parents are isolated
    Given two parent sessions exist: "Parent A" and "Parent B"
    And two watcher sessions exist for "Parent A": "W1", "W2"
    And two watcher sessions exist for "Parent B": "W3", "W4"
    And I am viewing watcher session "W1"
    When I press Shift+Right
    Then I should be viewing watcher session "W2"
    And I should not navigate to "W3" or "W4"
