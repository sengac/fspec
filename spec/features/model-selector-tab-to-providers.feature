@done
@tui
@model-selector
@RPC-345
Feature: Model selector missing Tab to return to Provider Settings
  """
  (a) Add ModelSelectorEvent::SwitchToProviders variant (model_selector/mod.rs enum ~:33-38). Pure UI nav, no Action payload. TS analog: onSwitchToSettings (ModelSelectorScreen.tsx:145).
  (b) Add `KeyCode::Tab => ModelSelectorEvent::SwitchToProviders` in handle_key before the `_` catch-all (mod.rs ~:353). Filter mode is handled earlier in handle_filter_key, so Tab while typing a filter stays consumed and does NOT navigate (mirror of provider_settings/list.rs:62).
  (c) Add navigator translation arm in handle_model_selector_event (navigator_events.rs after Emit ~:75): SwitchToProviders => send Action::OpenProviderSettingsView; EventResult::consumed(). No new ViewMode/apply_action needed — Action::OpenProviderSettingsView already handled at navigator.rs:111-112 setting ViewMode::ProviderSettings (mirror of SwitchToModels arm :45-50).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing Tab in the model selector (non-filter mode) emits a SwitchToProviders event
  #   2. SwitchToProviders is pure UI navigation with no Action payload on the model-selector side (mirrors TS onSwitchToSettings)
  #   3. The Navigator translates SwitchToProviders into Action::OpenProviderSettingsView, switching the active view to Provider Settings
  #   4. Pressing Tab while in filter mode does NOT navigate away; the filter handler consumes the key and the selector stays open
  #   5. Adding the Tab arm does not disturb existing key handling (Esc=Close, Enter=select, /=filter, r=refresh, arrows=navigate)
  #
  # EXAMPLES:
  #   1. In the model selector with providers loaded and not filtering, pressing Tab returns SwitchToProviders from handle_key
  #   2. Navigator forwards a Tab key to the model selector; the resulting SwitchToProviders sends Action::OpenProviderSettingsView and active_view becomes ProviderSettings
  #   3. While in filter mode (after pressing /), pressing Tab keeps filter_mode active and does NOT return SwitchToProviders
  #   4. Pressing Esc in non-filter mode still returns Close (Tab arm did not change the catch-all/Close behavior)
  #
  # ========================================
  Background: User Story
    As a Codelet user in the model selector
    I want to press Tab to jump back to Provider Settings
    So that I can toggle between the two screens in both directions without reaching for Esc

  Scenario: Tab in non-filter mode emits SwitchToProviders
    Given the model selector has loaded providers and is not in filter mode
    When I press the Tab key
    Then the model selector returns a SwitchToProviders event

  Scenario: SwitchToProviders switches the active view to Provider Settings
    Given the Navigator is showing the model selector with providers loaded
    When I press the Tab key in the model selector
    Then the Navigator sends the OpenProviderSettingsView action
    And the active view becomes Provider Settings

  Scenario: Tab while filtering does not navigate away
    Given the model selector is in filter mode after I pressed "/"
    When I press the Tab key
    Then the model selector stays in filter mode
    And no SwitchToProviders event is returned

  Scenario: Esc still closes the selector after the Tab arm is added
    Given the model selector has loaded providers and is not in filter mode
    When I press the Esc key
    Then the model selector returns a Close event
