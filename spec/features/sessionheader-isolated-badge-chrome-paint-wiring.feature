@done
@header
@isolation
@agent-view
@tui
@bug-178
@BUG-178
Feature: SessionHeader [ISOLATED] badge chrome_paint wiring
  """
  BUG-178: `chrome_paint::paint_header_and_role` hardcodes
  `is_isolated: false` when constructing `SessionHeader`, so the
  green [ISOLATED] badge NEVER paints even when the per-session
  store slot is populated by `StreamChunk::IsolationStateChange`.

  The store slot (`AgentViewStore::isolation_state_by_session`) is
  already correctly written by
  `App::handle_stream_chunk_state_updates` and is already read by
  the `/isolation` slash-command toggle.  The only missing link is
  the one-line read in `paint_header_and_role`.

  Fix: read `store.isolation_state_for(sid)` exactly the same way
  `is_debug_enabled` reads `store.debug_enabled_for(sid)`, and
  pass the result into `SessionHeader::is_isolated`.
  """

  Background: User Story
    As a TUI user
    I want to see a green [ISOLATED] badge in the SessionHeader
    So that I know which sessions are running in an isolated git worktree

  @tui
  Scenario: paint_header_and_role paints [ISOLATED] when store says session is isolated
    Given an App with a single session s-1 in AgentView
    And the store's isolation_state_by_session[s-1] is IsolationState { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }
    When the App renders
    Then the header row contains the substring "[ISOLATED]"

  @tui
  Scenario: paint_header_and_role does not paint [ISOLATED] when store says session is not isolated
    Given an App with a single session s-1 in AgentView
    And the store's isolation_state_by_session[s-1] is IsolationState { is_isolated: false, worktree_path: None, base_commit: None }
    When the App renders
    Then the header row does NOT contain the substring "[ISOLATED]"

  @tui
  Scenario: paint_header_and_role does not paint [ISOLATED] when no isolation chunk has arrived
    Given an App with a single session s-1 in AgentView
    When the App renders
    Then the header row does NOT contain the substring "[ISOLATED]"

  @tui
  Scenario: IsolationStateChange(true, ...) chunk dispatched before render makes badge appear
    Given an App with a single session s-1 in AgentView
    When Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }) is dispatched
    And the App renders
    Then the header row contains the substring "[ISOLATED]"

  @tui
  Scenario: IsolationStateChange(false, None) chunk dispatched before render removes badge
    Given an App with a single session s-1 in AgentView
    And Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }) is dispatched
    When Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: false, worktree_path: None, base_commit: None }) is dispatched
    And the App renders
    Then the header row does NOT contain the substring "[ISOLATED]"
