@tui
@done
@session-management
@agent-core
@rust
@napi
@navigation
@BUG-124
Feature: Shift+Arrow navigation skips sessions when supervisor has multiple subordinates
  """
  Fix lives in codelet/napi/src/navigation.rs build_navigation_list() — replace hierarchy-aware traversal with sessions.keys().copied().collect()
  chain_of_command parameter remains in the signature but is unused (prefix with underscore) — preserves ABI/test compatibility
  Tests live in codelet/napi/tests/navigation_hierarchy_test.rs (Rust integration test file run via cargo test). The file already provides MockBackgroundSession + MockChainOfCommand and a local copy of build_navigation_list — the local mock copy and the production navigation.rs copy must both be updated.
  The trigger that exposes the bug is INSERTION ORDER: when the supervisor session is inserted into the IndexMap BEFORE its subordinates (real-world spawn pattern). Existing tests in navigation_hierarchy_test.rs always insert children adjacent to or after their named groupings, which happens to mask the duplication. New regression tests must insert the supervisor FIRST, then the subordinates.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. build_navigation_list must include every session in the SessionManager IndexMap exactly once
  #   2. Sessions in the navigation list must appear in IndexMap insertion order (spawn order)
  #   3. No session UUID may appear more than once in the navigation list, regardless of supervisor/subordinate relationships
  #   4. Repeatedly pressing Shift+Right from the board must visit every session exactly once before reaching the create-session dialog
  #   5. The chain_of_command parameter is preserved in the function signature for ABI stability but is no longer consulted
  #
  # EXAMPLES:
  #   1. Supervisor with no spawned subordinates is on the board and presses Shift+Right; the create-session dialog appears because there are no sessions to visit
  #   2. Supervisor has spawned exactly one subordinate, presses Shift+Right from the board, and lands on that subordinate
  #   3. Supervisor has spawned five subordinates and repeatedly presses Shift+Right from the board, visiting all five subordinates exactly once in spawn order before reaching the create-session dialog
  #   4. Supervisor has spawned five subordinates and repeatedly presses Shift+Left from the last subordinate, visiting every prior subordinate exactly once in reverse spawn order before reaching the board
  #   5. Supervisor has spawned two subordinates, presses Shift+Right twice from the board, lands on the second subordinate (regression: previously got stuck looping on the first)
  #   6. Supervisor on the last session in spawn order presses Shift+Right and the create-session dialog appears
  #   7. Supervisor on the first session in spawn order presses Shift+Left and lands back on the board
  #
  # ========================================
  Background: User Story
    As a supervisor agent with multiple spawned subordinates
    I want to press Shift+Left or Shift+Right to cycle through sessions
    So that I can reach every spawned agent in a clean, deterministic order

  Scenario: Empty session manager produces an empty navigation list
    Given the session manager contains no sessions
    When the navigation list is built
    Then the navigation list is empty
    And pressing Shift+Right from the board shows the create-session dialog

  Scenario: Single subordinate appears once in the navigation list
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned one subordinate "s1"
    When the navigation list is built
    Then the navigation list contains the supervisor and "s1" exactly once each in insertion order

  Scenario: Five subordinates appear once each in spawn order
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned subordinates "s1", "s2", "s3", "s4", "s5" in that order
    When the navigation list is built
    Then the navigation list is exactly [supervisor, s1, s2, s3, s4, s5]
    And no UUID appears more than once

  Scenario: Shift+Right from the board cycles through every session exactly once
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned subordinates "s1", "s2", "s3", "s4", "s5" in that order
    And I am on the board
    When I press Shift+Right repeatedly until I reach the create-session dialog
    Then I visit each of [supervisor, s1, s2, s3, s4, s5] exactly once in that order
    And the next press shows the create-session dialog

  Scenario: Shift+Left from the last subordinate cycles back to the board exactly once per session
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned subordinates "s1", "s2", "s3", "s4", "s5" in that order
    And I am viewing "s5"
    When I press Shift+Left repeatedly until I reach the board
    Then I visit each of [s4, s3, s2, s1, supervisor] exactly once in that order
    And the next press returns to the board

  Scenario: Two subordinates do not loop on the first subordinate (regression)
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned subordinates "s1" and "s2" in that order
    And I am on the board
    When I press Shift+Right twice
    Then I am viewing "s1" after the first press
    And I am viewing "s2" after the second press

  Scenario: Shift+Right from the last session shows the create-session dialog
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned subordinates "s1" and "s2" in that order
    And I am viewing "s2"
    When I press Shift+Right
    Then the create-session dialog appears

  Scenario: Shift+Left from the first session returns to the board
    Given the session manager contains the supervisor inserted first
    And the supervisor has spawned subordinates "s1" and "s2" in that order
    And I am viewing the supervisor
    When I press Shift+Left
    Then I am on the board
