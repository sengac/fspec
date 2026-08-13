@done
@RPC-024
@rust
@tui
@agent-view
@store
@session-switch
Feature: RPC-024 AgentViewStore multi-session state primitives
  """
  RPC-024 (store slice) — Replace the single-slot
  `current_session: Option<SessionId>` on `AgentViewStore` with a
  `Vec<SessionContext>` plus `current_session_index: usize`.

  Adds:
  - `pub fn append_session(SessionContext)` — sole producer.
  - `pub fn cycle_session(delta: isize)` — wraps with `.rem_euclid`,
  no-op for length 0 / 1.
  - `pub fn current_session_context[_mut]()` — focused SessionContext.
  - `pub fn session_context_mut_for(&SessionId)` — id-keyed lookup.
  - `pub fn set_input_draft(idx, String)` — persist outgoing draft.

  Removes:
  - `pub fn set_current_session(...)` — implicit via `append_session`.
  - `pub fn set_session_index(...)` — replaced by derived getter.

  SessionContext lives in a sibling module
  `rust/fspec-tui/src/store/agent_view/session_context.rs` so
  agent_view.rs stays under the 300-LoC ceiling.

  Tests: rust/fspec-tui/tests/store_agent_view_multisession_rpc024.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the AgentViewStore to hold multiple concurrent SessionContexts and let me cycle between them with wrap-around
    So that the App layer can route Shift+←/→ into a real multi-session model without losing per-session state

  Scenario: An empty AgentViewStore reports zero open sessions and a no-op cycle_session
    Given a default AgentViewStore
    Then open_sessions is empty
    And session_index() returns (0, 0)
    And current_session() returns None
    When cycle_session(1) is called
    Then current_session_index is still 0
    And open_sessions is still empty

  Scenario: append_session appends a fresh SessionContext and focuses it
    Given a default AgentViewStore with no open sessions
    When append_session is called for "s-1"
    Then open_sessions has length 1
    And open_sessions[0].id equals "s-1"
    And current_session_index is 0
    And session_index() returns (1, 1)
    And current_session() returns Some("s-1")

  Scenario: Successive append_session calls accumulate in creation order
    Given a default AgentViewStore with no open sessions
    When append_session is called for "s-1"
    And append_session is called for "s-2"
    And append_session is called for "s-3"
    Then open_sessions has length 3
    And open_sessions ids in order are "s-1", "s-2", "s-3"
    And current_session_index is 2
    And session_index() returns (3, 3)
    And current_session() returns Some("s-3")

  Scenario: cycle_session(-1) decrements current_session_index without wrap when not at zero
    Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    And current_session_index is 2
    When cycle_session(-1) is called
    Then current_session_index is 1
    And session_index() returns (2, 3)
    And current_session() returns Some("s-2")

  Scenario: cycle_session(-1) wraps from index 0 to len-1; cycle_session(1) wraps back
    Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    And current_session_index is 0
    When cycle_session(-1) is called
    Then current_session_index is 2
    And current_session() returns Some("s-3")
    When cycle_session(1) is called
    Then current_session_index is 0
    And current_session() returns Some("s-1")

  Scenario: cycle_session is a self-loop when only one session is open
    Given an AgentViewStore with open_sessions ["s-1"]
    And current_session_index is 0
    When cycle_session(-1) is called
    Then current_session_index is 0
    When cycle_session(1) is called
    Then current_session_index is 0
    And current_session() returns Some("s-1")

  Scenario: set_input_draft writes into the indexed SessionContext
    Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    And current_session_index is 1
    When set_input_draft(1, "hello world") is called
    Then open_sessions[1].input_draft equals "hello world"
    When cycle_session(-1) is called
    Then the incoming draft is the saved (empty) draft on s-1
    When cycle_session(1) is called
    Then the outgoing draft was preserved on s-2

  Scenario: SessionContext owns its own scrollback
    Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    When three text chunks are recorded on s-1's SessionContext
    And five text chunks are recorded on s-2's SessionContext
    Then s-1 still has 3 scrollback chunks
    And s-2 still has 5 scrollback chunks

  Scenario: session_context_mut_for routes chunks by id, not focus
    Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    And current_session_index is 0
    And open_sessions[1].scrollback contains 0 chunks
    When session_context_mut_for(SessionId("s-2")) records a "background" chunk
    Then open_sessions[1].scrollback contains 1 chunk
    And current_session_index is still 0

  Scenario: session_context_mut_for returns None for unknown ids
    Given an AgentViewStore with open_sessions ["s-1"]
    When session_context_mut_for(SessionId("s-ghost")) is called
    Then the lookup returns None (caller drops the chunk)
    And open_sessions[0].scrollback is unchanged

  Scenario: session_index() is derived from current_session_index + 1 and open_sessions.len()
    Given the AgentViewStore type
    Then session_index() returns (0, 0) for an empty store
    When sessions are appended
    Then session_index() returns (current_session_index + 1, len)
