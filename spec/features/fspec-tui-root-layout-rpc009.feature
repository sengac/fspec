@done
@tui
@rust
@infrastructure
@parity
@rpc
@RPC-009
@critical
Feature: Root layout + Tab focus cycling + footer hint bar (RPC-009)
  """
  RootView is the new top-level Component (priority Background, id "root") replacing HelloComponent in App::new. State: `RootView { work_units: WorkUnitsListView, repl: AgentReplView, footer: FooterView, focused_pane: FocusedPane }` where `enum FocusedPane { WorkUnits, Repl }`. Layout: outer `Layout::vertical([Constraint::Min(0), Constraint::Length(1)])` reserves the bottom row for the footer hint bar; the upper region splits horizontally via `Layout::horizontal([Constraint::Length(32), Constraint::Min(0)])`; the right column splits vertically via `Layout::vertical([Constraint::Min(0), Constraint::Length(3)])` into scrollback + 3-row input box. handle_event intercepts Tab → emits Action::FocusNext on the action bus and returns Consumed (no callback); otherwise forwards to whichever sub-pane is focused. Sub-panes are CHILDREN of RootView (NOT separate compositor layers) — only the help dialog uses the compositor's modal layering. Footer hint bar (FooterView): 1-row Component rendering `?`-help, `q`-quit, `Tab`-switch-pane via styled `Spans` against the existing Theme. NO tui-prompts, NO throbber-widgets-tui.
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want RootView to compose WorkUnitsListView + AgentReplView + FooterView via Layout::horizontal([Length(32), Min(0)]) and Layout::vertical([Min(0), Length(3)]) plus a 1-row footer hint bar at Constraint::Length(1), to forward events to whichever sub-pane is focused, and to emit Action::FocusNext on Tab
    So that the basic frontend has its locked two-pane shape with footer hints AND Tab cycles focus between panes

  Scenario: RootView is constructed at Priority::Background with id "root" replacing HelloComponent
    Given a freshly constructed `App::new(Arc::new(MockBackend::default()))`
    Then the App's compositor contains exactly one layer
    And that layer's id() returns "root"
    And that layer's priority() returns Priority::Background
    And that layer's is_active() returns true

  Scenario: Outer layout reserves a 1-row footer at the bottom and a Length(32) left column for the work-units list
    Given a RootView rendered onto an 80x24 TestBackend
    Then the bottom-most row of the buffer contains the footer hint bar text
    And the left column band of width 32 contains the WorkUnitsListView's bordered Block
    And the right column band of width (80 - 32) contains the AgentReplView's scrollback + 3-row input area
    And the input area's height is exactly 3 rows

  Scenario: Tab at the RootView level emits Action::FocusNext and returns Consumed
    Given a RootView with focused_pane = WorkUnits
    When the view processes a synthetic Key(Tab) event
    Then handle_event returns `EventResult::Consumed(None)`
    And the action bus receives `Action::FocusNext`

  Scenario: Action::FocusNext alternates focused_pane between WorkUnits and Repl
    Given a RootView with focused_pane = WorkUnits, child WorkUnitsListView focused = true, child AgentReplView focused = false
    When the App dispatches `Action::FocusNext`
    Then RootView's focused_pane = Repl
    And the WorkUnitsListView's focused field is false
    And the AgentReplView's focused field is true
    When the App dispatches `Action::FocusNext` again
    Then RootView's focused_pane = WorkUnits
    And the WorkUnitsListView's focused field is true
    And the AgentReplView's focused field is false

  Scenario: Footer hint bar renders the three keybinding hints on the bottom row
    Given a RootView rendered onto an 80x24 TestBackend
    When the bottom row text of the buffer is read out
    Then the row contains the substring "?"
    And the row contains the substring "q"
    And the row contains the substring "Tab"

  Scenario: Non-Tab events are forwarded to the focused sub-pane
    Given a RootView with focused_pane = WorkUnits and the WorkUnitsListView seeded with three entries at index 0
    When the RootView processes a synthetic Key('j') event
    Then the WorkUnitsListView's state.selected() returns Some(1)
    And the AgentReplView's input.value() is unchanged
    Given a RootView with focused_pane = Repl and an empty AgentReplView input
    When the RootView processes a synthetic Key('h') event
    Then the AgentReplView's input.value() equals "h"
    And the WorkUnitsListView's state.selected() is unchanged
