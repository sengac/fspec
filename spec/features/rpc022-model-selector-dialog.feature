@done
@RPC-022
@rust
@tui
@dialog
@modal
@model-selector
@agent-view
Feature: ModelSelectorDialog component for picking provider + model

  """
  ModelSelectorDialog lives at codelet/fspec-tui/src/components/model_selector_dialog.rs
  and is the Rust port of src/tui/components/ModelSelectorScreen.tsx +
  src/tui/components/ModelSelectorView.tsx (custom model creation/deletion
  out of scope — RPC-022 ships read-only provider/model selection only).

  The dialog renders at Priority::Foreground (a NEW variant introduced
  by this card with numeric value 900, between High=800 and
  Critical=1000) so HelpDialog / DisconnectDialog (Priority::Critical)
  still win when they share the screen. The dialog uses the
  tui_popup::Popup adapter pattern from HelpDialog / DisconnectDialog —
  NO hand-rolled centered_rect helper.

  Pushed onto the Compositor by App::dispatch when the user types
  `/model` + Enter (handled by parse_slash_command in
  app/dispatch_rpc022.rs). On Enter inside a model row the dialog emits
  Action::ModelSelected(SessionId, provider_key, model_id) and removes
  itself from the Compositor. On Esc it removes itself with no side
  effects.

  Custom model creation/deletion is OUT of scope for RPC-022 — the
  dialog shows a static hint line "Custom models: not yet supported"
  where the TS CustomModelFormView / DeleteCustomModelConfirmView code
  paths used to live.
  """

  Background: User Story
    As a Rust fspec TUI user
    I want to pick a provider + model from a modal dialog opened via /model
    So that I can switch the agent's model without dropping back to the Ink TS TUI

  @priority @foreground @smoke
  Scenario: ModelSelectorDialog renders at Priority::Foreground
    Given a fresh ModelSelectorDialog with id "model-selector-dialog"
    When its priority() method is invoked
    Then the result is Priority::Foreground
    And Priority::Foreground has discriminant 900
    And Priority::Foreground sorts strictly between Priority::High (800) and Priority::Critical (1000)

  @ui-rendering @tui-popup
  Scenario: ModelSelectorDialog renders via the tui-popup adapter pattern
    Given a ModelSelectorDialog seeded with two providers (anthropic with [opus-4.6], openai with [gpt-5.1-codex])
    When the dialog is rendered onto a 100x30 TestBackend
    Then the rendered buffer contains the substring "Select Model"
    And the rendered buffer contains the substring "anthropic"
    And the rendered buffer contains the substring "openai"
    And the production source uses Popup::new(...) wrapping a SizedWidgetRef adapter
    And the production source does NOT define a hand-rolled centered_rect helper

  @navigation @keyboard-navigation
  Scenario: Arrow keys navigate the flat provider+model list with wrap-around
    Given a ModelSelectorDialog seeded with anthropic[opus-4.6] and openai[gpt-5.1-codex]
    And the dialog is initialised with selected_index = 0 (anthropic header)
    When the user presses Down four times
    Then selected_index wraps back to 0 after exhausting the visible list

  @navigation @selection
  Scenario: Enter on a model row emits Action::ModelSelected
    Given a ModelSelectorDialog seeded with anthropic[opus-4.6] and openai[gpt-5.1-codex]
    And selected_index points at the openai[gpt-5.1-codex] row
    And the dialog was constructed against SessionId::new("s-1")
    When the user presses Enter
    Then handle_event returns EventResult::Consumed with a callback
    And the callback emits Action::ModelSelected(SessionId::new("s-1"), "openai", "gpt-5.1-codex")
    And the callback removes the dialog from the Compositor via its id

  @navigation @dismiss
  Scenario: Esc dismisses the ModelSelectorDialog without side effects
    Given a ModelSelectorDialog seeded with anthropic[opus-4.6]
    When the user presses Esc
    Then handle_event returns EventResult::Consumed with a callback
    And the callback removes the dialog from the Compositor via its id
    And no Action::ModelSelected is emitted

  @ui-rendering @empty
  Scenario: ModelSelectorDialog with zero providers shows a 'No providers available' hint
    Given a ModelSelectorDialog seeded with Vec::<ProviderInfo>::new()
    When the dialog is rendered onto a 80x24 TestBackend
    Then the rendered buffer contains the substring "No providers available"
    And pressing Enter on the empty list emits NO Action::ModelSelected

  @ui-rendering @help-text
  Scenario: ModelSelectorDialog footer documents the out-of-scope custom model creation
    Given a ModelSelectorDialog with at least one provider
    When the dialog is rendered onto a 100x30 TestBackend
    Then the rendered buffer contains the substring "Custom models: not yet supported"

  @ui-rendering @capabilities
  Scenario: Each model row paints capability badges [R] [V] [Nk]
    Given a ModelSelectorDialog seeded with anthropic[opus-4.6] where opus-4.6 supports reasoning AND vision AND has context_window 200000
    When the dialog is rendered
    Then the row for opus-4.6 contains the substring "[R]"
    And the row for opus-4.6 contains the substring "[V]"
    And the row for opus-4.6 contains the substring "[200k]"

  @line-budget @source-shape
  Scenario: model_selector_dialog.rs stays under 300 lines
    Given the file codelet/fspec-tui/src/components/model_selector_dialog.rs after RPC-022 lands
    When a test counts the line-count of the file
    Then the file has fewer than 300 lines
