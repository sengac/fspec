@model-selection
@done
@ts-parity
@rust
@model-selector
@tui
@RPC-337
Feature: Model selector navigator wiring
  """
  Navigator wiring: add ViewMode::ModelSelector, field model_selector: ModelSelectorView, handle_model_selector_event, render arm, apply_action arms for Action::OpenModelSelectorView/CloseModelSelectorView. Tab in ProviderSettings List mode (not filter mode) flips the Navigator into ViewMode::ModelSelector, wiring the existing ProviderSettingsEvent::SwitchToModels stub.
  """

  Background: User Story
    As a fspec TUI user
    I want Provider Settings to hand off to the model selector via Tab
    So that switching between the two full-screen views is seamless

  Scenario: Switch from Provider Settings to the model selector with Tab
    Given I am in the Provider Settings view in list mode
    When I press Tab
    Then the Navigator flips to the model selector mode-view full-screen
