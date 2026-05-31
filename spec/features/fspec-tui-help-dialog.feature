@done
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: HelpDialog (Critical-priority modal)
  Critical-priority modal triggered by the `?` key at App-level. Body
  lists exactly the `?`, ESC, and `q` keybindings via a tui_popup::Popup
  wrapped in a SizedWidgetRef adapter (per RPC-002 Q5 — production
  code path is the adapter, not a hand-rolled centered_rect helper).
  Rendering is byte-equal across runs (insta snapshot).

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want HelpDialog to render via tui_popup::Popup wrapping a SizedWidgetRef adapter at Priority::Critical with a static body listing exactly ?, ESC, and q
    So that the App-level help affordance is visually consistent and byte-equal across runs

  Scenario: HelpDialog renders via the tui-popup adapter at Priority::Critical
    Given an isolated HelpDialog component with id "help-dialog"
    When I inspect its priority()
    Then it returns Priority::Critical
    When I inspect its render(...) implementation
    Then it constructs a `tui_popup::Popup` wrapping a `SizedWidgetRef` adapter
    And it does NOT use a hand-rolled `centered_rect` helper as the production code path

  Scenario: HelpDialog static body lists exactly the '?', ESC, and 'q' keybindings
    Given an isolated HelpDialog component
    When I render it onto an 80x24 TestBackend buffer
    Then the buffer contains a line including "?"
    And the buffer contains a line including "ESC"
    And the buffer contains a line including "q"

  Scenario: HelpDialog rendering is byte-equal across runs (insta snapshot)
    Given an isolated HelpDialog component
    When I render it onto an 80x24 TestBackend buffer
    And I serialise the buffer cell grid via `insta::assert_yaml_snapshot!`
    Then the serialised output matches the snapshot file "help_dialog__centered_popup_80x24.snap"
