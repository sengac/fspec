@done
@tui-component
@agent-view
@tui
@rust
@RPC-096
Feature: AgentView Shift+Left/Right end-of-list parity: open Create Session dialog off-right, exit to BoardView off-left
  """
  Architecture target: codelet/fspec-tui/src/store/agent_view.rs — add `pub enum NavTarget { Session(usize), CreateDialog, Board }`, `pub fn navigate_next(&self) -> NavTarget`, `pub fn navigate_prev(&self) -> NavTarget`. Delete `pub fn cycle_session`. Extract to sibling agent_view/navigation.rs if file exceeds 300 LoC.
  Architecture target: codelet/fspec-tui/src/app/dispatch_session_cycle.rs — replace `agent_view_store.cycle_session(delta)` with `let target = if delta < 0 { store.navigate_prev() } else { store.navigate_next() };` then `match target { NavTarget::Session(idx) => self.switch_to_session_index(idx), NavTarget::CreateDialog => self.agent_view_store.request_create_session_dialog_no_auto(), NavTarget::Board => self.dispatch(Action::BackToBoard) }`. Need a new store method `request_create_session_dialog_no_auto()` that sets only `show_create_session_dialog = true` (not `should_auto_create_session`).
  Existing reusable infra: CreateSessionDialog component already exists at codelet/fspec-tui/src/components/create_session_dialog.rs with CREATE_SESSION_DIALOG_ID and Component impl. The store flag show_create_session_dialog is already plumbed and consumed by the compositor render path — no new dialog component work required. Action::BackToBoard already exists at codelet/fspec-tui/src/app/dispatch.rs:111-113 for the Board exit.
  Draft snapshot semantics: for NavTarget::Session(idx) preserve RPC-024 round-trip; for NavTarget::CreateDialog do NOT snapshot the draft (per Example [5] the user expects their typed text to persist as they 'open the dialog'); for NavTarget::Board snapshot the draft into current session's input_draft so a later switch back via the BoardView restores the typing the user had in flight.
  """

  Background: User Story
    As a Rust TUI user
    I want to press Shift+Right at the end of my session list or Shift+Left at the start of it
    So that I can open the Create Session dialog (off-right) or return to the BoardView (off-left), exactly like the TypeScript Ink original

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: navigate_next returns CreateDialog and navigate_prev returns Board when the store is empty
    Given an AgentViewStore with zero open sessions
    When I call navigate_next on the store
    Then the result is NavTarget::CreateDialog
    When I call navigate_prev on the store
    Then the result is NavTarget::Board

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: navigate_next returns CreateDialog at the last session index
    Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 2
    When I call navigate_next on the store
    Then the result is NavTarget::CreateDialog

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: navigate_prev returns Session(idx-1) when not at the first session index
    Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 2
    When I call navigate_prev on the store
    Then the result is NavTarget::Session(1)

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: navigate_next returns Session(idx+1) when not at the last session index
    Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 0
    When I call navigate_next on the store
    Then the result is NavTarget::Session(1)

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: navigate_prev returns Board at the first session index
    Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 0
    When I call navigate_prev on the store
    Then the result is NavTarget::Board

  @rust
  @tui
  @agent-view
  @integration
  Scenario: Shift+Right with a single attached session opens the Create Session dialog
    Given an App with one open session s-1 and current_session_index 0
    And navigator.active_view is ViewMode::Agent
    When the App dispatches Action::SessionNext
    Then agent_view_store.show_create_session_dialog() returns true
    And agent_view_store.should_auto_create_session() returns false
    And agent_view_store.current_session_index stays 0
    And navigator.active_view stays ViewMode::Agent

  @rust
  @tui
  @agent-view
  @integration
  Scenario: Shift+Left with a single attached session exits AgentView back to the Board
    Given an App with one open session s-1 and current_session_index 0
    And navigator.active_view is ViewMode::Agent
    When the App dispatches Action::SessionPrev
    Then navigator.active_view is ViewMode::Board
    And agent_view_store.current_session_index stays 0
    And agent_view_store.show_create_session_dialog() returns false

  @rust
  @tui
  @agent-view
  @integration
  Scenario: Shift+Right at the last index preserves the typed draft when opening the Create Session dialog
    Given an App with three open sessions s-1, s-2, s-3 and current_session_index 2
    And the MultiLineInput value is "goodbye"
    When the App dispatches Action::SessionNext
    Then agent_view_store.show_create_session_dialog() returns true
    And the MultiLineInput value is still "goodbye"
    And agent_view_store.current_session_index stays 2

  @rust
  @tui
  @agent-view
  @integration
  Scenario: Shift+Left at the first index snapshots the outgoing draft before exiting to the Board
    Given an App with three open sessions s-1, s-2, s-3 and current_session_index 0
    And the MultiLineInput value is "pending"
    When the App dispatches Action::SessionPrev
    Then navigator.active_view is ViewMode::Board
    And open_sessions[0].input_draft is "pending"
    And agent_view_store.current_session_index stays 0

  @rust
  @tui
  @agent-view
  @integration
  Scenario: Mid-list Shift+Right preserves the RPC-024 draft round-trip and session switch
    Given an App with three open sessions s-1, s-2, s-3 and current_session_index 1
    And the MultiLineInput value is "midway"
    And open_sessions[2].input_draft is "third"
    When the App dispatches Action::SessionNext
    Then agent_view_store.current_session_index is 2
    And open_sessions[1].input_draft is "midway"
    And the MultiLineInput value is "third"
    And agent_view_store.show_create_session_dialog() returns false
    And navigator.active_view stays ViewMode::Agent

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: cycle_session is removed from the AgentViewStore public API
    Given the source file codelet/fspec-tui/src/store/agent_view.rs
    Then it does not contain the substring "pub fn cycle_session"
    And it does contain the substring "pub fn navigate_next"
    And it does contain the substring "pub fn navigate_prev"
    And the file has fewer than 300 lines, OR navigate_next/navigate_prev live in the sibling module codelet/fspec-tui/src/store/agent_view/navigation.rs
