@WATCH-002
Feature: WatchGraph and Session Relationship Tracking

  """
  Architecture notes:
  - WatchGraph is a struct with two RwLock-protected HashMaps living inside SessionManager
  - SessionManager gains a watch_graph: WatchGraph field initialized in new()
  - All WatchGraph methods are exposed via SessionManager delegation methods
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. WatchGraph is a data structure owned by SessionManager that tracks parent-watcher relationships
  #   2. WatchGraph uses two HashMaps: parent_to_watchers (Uuid → Vec<Uuid>) and watcher_to_parent (Uuid → Uuid)
  #   3. One watcher can only watch one parent (1:1 from watcher side), but one parent can have multiple watchers (1:N from parent side)
  #   4. Circular watching is prevented - a watcher cannot watch its own parent chain
  #   5. Watch relationships are ephemeral - they do not persist across session restarts
  #   6. When a parent session is removed, all its watchers must be cleaned up from the WatchGraph
  #   7. When a watcher session is removed, its entry must be cleaned up from both HashMaps
  #
  # EXAMPLES:
  #   1. add_watcher(parent_id: abc, watcher_id: xyz) → parent_to_watchers[abc] = [xyz], watcher_to_parent[xyz] = abc
  #   2. get_watchers(parent_id: abc) returns [xyz, def] when parent abc has two watchers attached
  #   3. get_parent(watcher_id: xyz) returns Some(abc) when watcher xyz is watching parent abc
  #   4. remove_watcher(watcher_id: xyz) removes xyz from parent_to_watchers[abc] and deletes watcher_to_parent[xyz]
  #   5. Attempting to add_watcher where watcher_id already has a parent returns an error (watcher can only watch one parent)
  #   6. Attempting add_watcher(A, B) when B is already watching A's parent returns error (circular watching prevented)
  #   7. get_parent(session_id: abc) returns None when abc is a regular session (not a watcher)
  #
  # ========================================

  Background: User Story
    As a watcher session subsystem
    I want to track parent-watcher relationships between sessions
    So that watchers can efficiently find their parent and parents can find all their watchers

  @unit
  Scenario: Register a watcher for a parent session
    Given a WatchGraph with no relationships
    And a parent session "abc" exists
    And a watcher session "xyz" exists
    When I call add_watcher with parent_id "abc" and watcher_id "xyz"
    Then get_watchers for "abc" should return ["xyz"]
    And get_parent for "xyz" should return "abc"

  @unit
  Scenario: Parent with multiple watchers
    Given a WatchGraph with no relationships
    And a parent session "abc" exists
    And watcher sessions "xyz" and "def" exist
    When I call add_watcher with parent_id "abc" and watcher_id "xyz"
    And I call add_watcher with parent_id "abc" and watcher_id "def"
    Then get_watchers for "abc" should return ["xyz", "def"]

  @unit
  Scenario: Query parent for a watcher
    Given a WatchGraph with no relationships
    And session "xyz" is watching session "abc"
    When I call get_parent with watcher_id "xyz"
    Then it should return "abc"

  @unit
  Scenario: Remove a watcher relationship
    Given a WatchGraph with no relationships
    And session "xyz" is watching session "abc"
    When I call remove_watcher with watcher_id "xyz"
    Then get_watchers for "abc" should return an empty list
    And get_parent for "xyz" should return None

  @unit
  Scenario: Watcher cannot watch multiple parents
    Given a WatchGraph with no relationships
    And session "xyz" is watching session "abc"
    When I call add_watcher with parent_id "def" and watcher_id "xyz"
    Then it should return an error "watcher already has a parent"

  @unit
  Scenario: Circular watching is prevented
    Given a WatchGraph with no relationships
    And session "B" is watching session "A"
    When I call add_watcher with parent_id "B" and watcher_id "A"
    Then it should return an error "circular watching not allowed"

  @unit
  Scenario: Regular session has no parent
    Given a WatchGraph with no relationships
    And a regular session "abc" exists that is not a watcher
    When I call get_parent with session_id "abc"
    Then it should return None

  @unit
  Scenario: Cleanup watchers when parent session is removed
    Given a WatchGraph with no relationships
    And session "xyz" is watching session "abc"
    And session "def" is watching session "abc"
    When parent session "abc" is removed
    Then get_parent for "xyz" should return None
    And get_parent for "def" should return None
    And the WatchGraph should have no entries
