@done
@RPC-024
@rust
@tui
@agent-view
@session-switch
Feature: RPC-024 App::dispatch routes SessionPrev/Next/ChunkReceived through multi-session AgentViewStore
  """
  RPC-024 (App dispatch slice) — wire the new multi-session
  AgentViewStore primitives into App::dispatch:
  - Action::SessionCreated → calls `append_session` (was set_current_session).
  - Action::SessionPrev / SessionNext → save outgoing draft, cycle, restore incoming draft.
  - Action::ChunkReceived(id, chunk) → route scrollback into the
  SessionContext whose id matches `id` (was `navigator.agent.record_chunk`).

  These were already emitted by RPC-019's MultiLineInput on Shift+←/→
  but until RPC-024 the catch-all `_ => {}` arm in App::dispatch
  swallowed them. RPC-024 adds the routing arms.

  Tests: codelet/fspec-tui/tests/app_dispatch_rpc024.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want App::dispatch to route Shift+←/→ and per-session chunks through the multi-session AgentViewStore
    So that switching sessions in the running TUI preserves scrollback, input drafts, and background chunks the same way the TS Ink TUI already does

  Scenario: Action::SessionCreated appends a fresh SessionContext and focuses it
    Given a default AgentViewStore with no open sessions
    When App::dispatch handles Action::SessionCreated for "s-1"
    Then open_sessions has length 1
    And open_sessions[0].id equals "s-1"
    And current_session_index is 0
    And session_index() returns (1, 1)
    And current_session() returns Some("s-1")

  Scenario: Successive SessionCreated dispatches accumulate in creation order
    Given a default AgentViewStore with no open sessions
    When App::dispatch handles Action::SessionCreated for "s-1"
    And App::dispatch handles Action::SessionCreated for "s-2"
    And App::dispatch handles Action::SessionCreated for "s-3"
    Then open_sessions has length 3
    And open_sessions ids in order are "s-1", "s-2", "s-3"
    And current_session_index is 2
    And session_index() returns (3, 3)
    And current_session() returns Some("s-3")

  Scenario: Action::SessionPrev decrements current_session_index without wrap when not at zero
    Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    And current_session_index is 2
    When App::dispatch handles Action::SessionPrev
    Then current_session_index is 1
    And session_index() returns (2, 3)
    And current_session() returns Some("s-2")

  Scenario: Action::SessionPrev wraps around from index 0 to len-1; SessionNext wraps back
    Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    And current_session_index is 0
    When App::dispatch handles Action::SessionPrev
    Then current_session_index is 2
    And current_session() returns Some("s-3")
    When App::dispatch handles Action::SessionNext
    Then current_session_index is 0
    And current_session() returns Some("s-1")

  Scenario: SessionPrev and SessionNext are self-loops when only one session is open
    Given an AgentViewStore with open_sessions ["s-1"]
    And current_session_index is 0
    When App::dispatch handles Action::SessionPrev
    Then current_session_index is 0
    When App::dispatch handles Action::SessionNext
    Then current_session_index is 0
    And current_session() returns Some("s-1")

  Scenario: Switching sessions saves the outgoing draft and restores the incoming draft
    Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    And current_session_index is 1
    And the MultiLineInput buffer is "hello world"
    When App::dispatch handles Action::SessionPrev
    Then open_sessions[1].input_draft equals "hello world"
    And the MultiLineInput buffer equals open_sessions[0].input_draft
    When App::dispatch handles Action::SessionNext
    Then the MultiLineInput buffer is "hello world"

  Scenario: Each session retains its own scrollback across cycling
    Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    And open_sessions[0].scrollback contains 3 chunks
    And open_sessions[1].scrollback contains 5 chunks
    And current_session_index is 1
    When App::dispatch handles Action::SessionPrev
    Then the AgentView render paints "s-1"'s 3 chunks
    When App::dispatch handles Action::SessionNext
    Then the AgentView render paints "s-2"'s 5 chunks
    And no chunks have been dropped from either session's scrollback

  Scenario: A StreamChunk for a non-current session lands in that session's scrollback
    Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    And current_session_index is 0
    And open_sessions[1].scrollback contains 0 chunks
    When App::dispatch handles Action::ChunkReceived("s-2", StreamChunk::text("background"))
    Then open_sessions[1].scrollback contains 1 chunk
    And current_session_index is still 0
    And the AgentView render still paints "s-1"'s scrollback
    When App::dispatch handles Action::SessionNext
    Then the AgentView render now includes the "background" chunk

  Scenario: Action::ChunkReceived for an unknown session id is dropped silently
    Given an AgentViewStore with open_sessions ["s-1"]
    When App::dispatch handles Action::ChunkReceived("s-ghost", StreamChunk::text("orphan"))
    Then App::dispatch does not panic
    And open_sessions[0].scrollback is unchanged
    And no other SessionContext exists for "s-ghost"
