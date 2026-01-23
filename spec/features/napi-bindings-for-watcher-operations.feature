@watcher
@codelet
@WATCH-007
Feature: NAPI Bindings for Watcher Operations

  """
  Add NAPI functions after existing session_* functions (around line 2780+)
  session_create_watcher must: 1) create session via SessionManager, 2) register in WatchGraph
  Broadcast subscription happens lazily when the watcher loop starts (via subscribe_to_stream())
  watcher_inject must use the watcher's SessionRole to format the message via format_watcher_input()
  Use WatchGraph methods: add_watcher(), get_parent(), get_watchers() already implemented in WATCH-002
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. session_create_watcher(parent_id, model, project, name) creates a new session as a watcher of parent_id and returns the watcher session ID
  #   2. session_get_parent(session_id) returns the parent session ID if the session is a watcher, None otherwise
  #   3. session_get_watchers(session_id) returns a list of watcher session IDs for the given parent session
  #   4. watcher_inject(watcher_id, message) formats and injects a watcher message into the parent session using receive_watcher_input()
  #   5. All NAPI bindings use #[napi] attribute and follow existing patterns in session_manager.rs
  #   6. Error handling returns napi::Error with descriptive messages matching existing NAPI function patterns
  #
  # EXAMPLES:
  #   1. session_create_watcher('parent-uuid', 'claude-sonnet-4', '/project', 'Code Reviewer') → creates watcher session, registers in WatchGraph → returns 'watcher-uuid' (broadcast subscription is lazy)
  #   2. session_get_parent('watcher-uuid') for a watcher session → returns 'parent-uuid'
  #   3. session_get_parent('regular-session-uuid') for a non-watcher session → returns None
  #   4. session_get_watchers('parent-uuid') with two watchers → returns ['watcher-1-uuid', 'watcher-2-uuid']
  #   5. session_get_watchers('session-with-no-watchers') → returns empty array []
  #   6. watcher_inject('watcher-uuid', 'Consider adding error handling') → formats message with watcher role prefix → queues on parent via receive_watcher_input()
  #   7. watcher_inject on a session without a role → error: Session has no watcher role set
  #   8. watcher_inject on a watcher with no parent registered → error: Watcher has no parent session
  #
  # ========================================

  Background: User Story
    As a TypeScript application
    I want to call NAPI bindings for watcher operations
    So that I can create watchers, get parent/watcher relationships, and inject watcher messages from the UI layer

  @wip
  Scenario: Create watcher session for a parent
    Given a parent session exists with id "parent-uuid"
    When I call session_create_watcher with parent "parent-uuid", model "claude-sonnet-4", project "/project", name "Code Reviewer"
    Then a new watcher session should be created and returned
    And the watcher should be registered in WatchGraph with parent "parent-uuid"
    # Note: Broadcast subscription happens lazily when watcher loop starts

  @wip
  Scenario: Get parent of a watcher session
    Given a watcher session "watcher-uuid" watching parent "parent-uuid"
    When I call session_get_parent with "watcher-uuid"
    Then it should return "parent-uuid"

  @wip
  Scenario: Get parent of a regular session returns None
    Given a regular session "regular-uuid" with no parent
    When I call session_get_parent with "regular-uuid"
    Then it should return None

  @wip
  Scenario: Get watchers of a parent session
    Given a parent session "parent-uuid"
    And watcher session "watcher-1-uuid" watching "parent-uuid"
    And watcher session "watcher-2-uuid" watching "parent-uuid"
    When I call session_get_watchers with "parent-uuid"
    Then it should return ["watcher-1-uuid", "watcher-2-uuid"]

  @wip
  Scenario: Get watchers of a session with no watchers
    Given a session "lonely-uuid" with no watchers
    When I call session_get_watchers with "lonely-uuid"
    Then it should return an empty array

  @wip
  Scenario: Inject watcher message into parent session
    Given a watcher session "watcher-uuid" with role "code-reviewer" and authority "Peer"
    And the watcher is watching parent "parent-uuid"
    When I call watcher_inject with watcher "watcher-uuid" and message "Consider adding error handling"
    Then the message should be formatted with watcher prefix
    And the message should be queued on the parent session

  @wip
  Scenario: Inject fails when session has no role
    Given a session "no-role-uuid" without a watcher role
    When I call watcher_inject with watcher "no-role-uuid" and message "Test"
    Then it should return error "Session has no watcher role set"

  @wip
  Scenario: Inject fails when watcher has no parent
    Given a session "orphan-uuid" with role "reviewer" but no parent registered
    When I call watcher_inject with watcher "orphan-uuid" and message "Test"
    Then it should return error "Watcher has no parent session"
