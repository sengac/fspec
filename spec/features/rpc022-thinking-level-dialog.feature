@done
@RPC-022
@rust
@tui
@dialog
@modal
@thinking
@agent-view
Feature: ThinkingLevelDialog component for picking Off/Low/Medium/High

  """
  ThinkingLevelDialog lives at codelet/fspec-tui/src/components/thinking_level_dialog.rs
  and is the Rust port of src/tui/components/ThinkingLevelDialog.tsx
  (TUI-054). It exposes four radio options — Off / Low / Medium / High —
  matching the codelet_rpc_types::ThinkingLevel enum from RPC-018.

  Renders at Priority::Foreground (numeric 900), same as
  ModelSelectorDialog, so HelpDialog / DisconnectDialog
  (Priority::Critical) still win when both are pushed. Uses the
  tui_popup::Popup adapter pattern from HelpDialog (NO hand-rolled
  centered_rect helper).

  Pushed onto the Compositor by App::dispatch when the user submits
  `/thinking` via the input (handled by parse_slash_command in
  app/dispatch_rpc022.rs). On Enter the dialog emits
  Action::ThinkingLevelSelected(SessionId, ThinkingLevel) and removes
  itself from the Compositor. On Esc it removes itself with no side
  effects.
  """

  Background: User Story
    As a Rust fspec TUI user
    I want to pick a thinking/reasoning level from a modal dialog opened via /thinking
    So that I can tune the agent's reasoning effort without dropping back to the Ink TS TUI

  @priority @foreground @smoke
  Scenario: ThinkingLevelDialog renders at Priority::Foreground
    Given a fresh ThinkingLevelDialog with id "thinking-level-dialog"
    When its priority() method is invoked
    Then the result is Priority::Foreground

  @ui-rendering @tui-popup
  Scenario: ThinkingLevelDialog renders the four canonical labels
    Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Off
    When the dialog is rendered onto an 80x24 TestBackend
    Then the rendered buffer contains the substring "Thinking Level"
    And the rendered buffer contains the substring "Off"
    And the rendered buffer contains the substring "Low"
    And the rendered buffer contains the substring "Medium"
    And the rendered buffer contains the substring "High"
    And the production source uses Popup::new(...) wrapping a SizedWidgetRef adapter
    And the production source does NOT define a hand-rolled centered_rect helper

  @ui-rendering @initial-selection
  Scenario: Dialog opens with the currently-active level pre-selected
    Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Medium
    When the dialog is rendered
    Then the Medium row is rendered with the selection marker "▸"
    And the Off / Low / High rows are rendered without the selection marker

  @navigation @keyboard-navigation
  Scenario Outline: Arrow keys navigate the 4 levels with wrap-around
    Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Off
    When the user presses <key> <count> times
    Then the highlighted row has label <expected>
    Examples:
      | key       | count | expected |
      | Down      | 1     | Low      |
      | Down      | 2     | Medium   |
      | Down      | 3     | High     |
      | Down      | 4     | Off      |
      | Up        | 1     | High     |
      | Up        | 2     | Medium   |
      | Up        | 4     | Off      |

  @navigation @selection
  Scenario: Enter on a level emits Action::ThinkingLevelSelected
    Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Off
    And the dialog was constructed against SessionId::new("s-1")
    When the user navigates Down 3 times so High is highlighted
    And the user presses Enter
    Then handle_event returns EventResult::Consumed with a callback
    And the callback emits Action::ThinkingLevelSelected(SessionId::new("s-1"), ThinkingLevel::High)
    And the callback removes the dialog from the Compositor via its id

  @navigation @dismiss
  Scenario: Esc dismisses the ThinkingLevelDialog without side effects
    Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::High
    When the user presses Esc
    Then handle_event returns EventResult::Consumed with a callback
    And the callback removes the dialog from the Compositor via its id
    And no Action::ThinkingLevelSelected is emitted

  @line-budget @source-shape
  Scenario: thinking_level_dialog.rs stays under 300 lines
    Given the file codelet/fspec-tui/src/components/thinking_level_dialog.rs after RPC-022 lands
    When a test counts the line-count of the file
    Then the file has fewer than 300 lines
