@done
@RPC-024
@rust
@tui
@agent-view
@source-shape
Feature: RPC-024 source-shape regressions for the multi-session AgentViewStore refactor
  """
  RPC-024 (source-shape slice) — pin the file layout invariants for
  the AgentViewStore multi-session refactor:
  - `rust/fspec-tui/src/store/agent_view/session_context.rs` exists
  and is under 300 LoC.
  - `rust/fspec-tui/src/store/agent_view.rs` stays under 300 LoC
  after the field replacement.
  - No file under `rust/fspec-tui/src/views/` imports forbidden crates.
  - The RPC-018 `set_session_index` setter is GONE; the
  `session_index()` getter is derived.
  - AgentView no longer owns `pub scrollback: ScrollbackList` or
  `pub next_seq: u64` — both moved to SessionContext.

  Tests: rust/fspec-tui/tests/source_shape_rpc024.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the source-shape invariants from RPC-002 to keep holding after the multi-session refactor
    So that the parity matrix continues to work and the agent_view.rs orchestrator does not bloat past 300 LoC

  Scenario: SessionContext lives in its own sub-module under the 300-LoC ceiling
    Given the rust/fspec-tui crate
    Then a file exists at rust/fspec-tui/src/store/agent_view/session_context.rs
    And that file is under 300 lines
    And the file rust/fspec-tui/src/store/agent_view.rs is under 300 lines
    And no file under rust/fspec-tui/src/views/ imports codelet_core, codelet_napi, tarpc, or tokio_tungstenite

  Scenario: SessionContext module declares the required public surface
    Given the rust/fspec-tui crate
    Then session_context.rs declares "pub struct SessionContext"
    And session_context.rs declares the "scrollback" field
    And session_context.rs declares the "input_draft" field

  Scenario: AgentViewStore exposes the multi-session surface
    Given rust/fspec-tui/src/store/agent_view.rs after RPC-024 lands
    Then the file declares an "open_sessions" field
    And the file declares a "current_session_index" field
    And the file declares "pub fn append_session"
    And the file declares "pub fn cycle_session"
    And the file declares "pub fn set_input_draft"
    And the file declares "pub fn session_context_mut_for"
    And the file declares "pub fn current_session_context"
    And the file declares "pub fn open_sessions"

  Scenario: Removing set_session_index closes the explicit-setter regression hole
    Given the AgentViewStore type
    Then there is no public method named set_session_index on AgentViewStore
    And the session_index() getter is computed from current_session_index and open_sessions.len()

  Scenario: AgentView no longer owns the scrollback / next_seq fields
    Given rust/fspec-tui/src/views/agent.rs after RPC-024 lands
    Then the file does NOT declare "pub scrollback: ScrollbackList"
    And the file does NOT declare "pub next_seq: u64"
