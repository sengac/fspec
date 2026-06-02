@done
@RPC-160 @rust-frontend @provider-settings @wip
Feature: Provider settings: Tab keybind in list mode emits SwitchToModels event

  """
  TS reference: src/tui/inputHandlers/listModeHandler.ts lines 57-60 —
  `if (key.tab) { onSwitchToModels(); return; }`. The callback is invoked
  as a sibling of `onClose()` and returns immediately, so subsequent
  navigation logic is skipped.

  Rust port (RPC-160):
    * codelet/fspec-tui/src/views/provider_settings/mod.rs adds
      `ProviderSettingsEvent::SwitchToModels` next to Consumed / Ignored /
      Emit(Action) / Close.
    * list.rs::handle_list_key adds `KeyCode::Tab =>
      ProviderSettingsEvent::SwitchToModels` BEFORE the catch-all
      `_ => Consumed`.
    * Filter-mode Tab continues to fall through handle_filter_key's
      catch-all (Consumed) because filter_mode is checked first.
    * navigator.rs::handle_provider_settings_event gets an arm
      `ProviderSettingsEvent::SwitchToModels => EventResult::consumed()`
      so the new variant compiles. Actual Navigator-to-models-view
      transition is deferred to a follow-up card.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing Tab in List mode emits a new
  #      ProviderSettingsEvent::SwitchToModels variant — distinct from
  #      Consumed/Close/Emit/Ignored.
  #   2. Tab MUST NOT emit any Action — SwitchToModels is a pure UI
  #      navigation event, not a backend command.
  #   3. Tab in List mode MUST NOT mutate selected_index, scroll_offset,
  #      filter, filter_mode, mode, or any other view state.
  #   4. Tab is only bound in List mode. Detail mode Tab and dialog Tab
  #      retain their pre-existing behaviour.
  #   5. Tab in filter sub-mode (filter_mode == true) MUST NOT trigger
  #      SwitchToModels — it is silently Consumed by the catch-all so
  #      typing into the filter is not interrupted.
  #
  # ========================================

  Background: User Story
    As a user navigating provider settings
    I want to press Tab in list mode
    So that I can switch to the models view without leaving the settings flow

  Scenario: Tab in List mode with providers emits SwitchToModels
    Given a ProviderSettingsView in List mode with 17 providers loaded
    And the cursor is on index 5
    When the user presses Tab with no modifiers
    Then handle_key returns ProviderSettingsEvent::SwitchToModels
    And selected_index is still 5
    And view.mode is still ProviderSettingsMode::List
    And no Action is emitted

  Scenario: Tab in List mode with an empty provider list still emits SwitchToModels
    Given a ProviderSettingsView in List mode with zero providers loaded
    When the user presses Tab with no modifiers
    Then handle_key returns ProviderSettingsEvent::SwitchToModels
    And view.mode is still ProviderSettingsMode::List

  Scenario: Tab in List mode with an active filter preserves filter state
    Given a ProviderSettingsView in List mode with 17 providers loaded
    And the filter is "open" and filter_mode is false
    And the nav_items list has been rebuilt to 4 entries
    When the user presses Tab with no modifiers
    Then handle_key returns ProviderSettingsEvent::SwitchToModels
    And view.filter equals "open"
    And view.filter_mode equals false
    And nav_items length is still 4

  Scenario: Tab in filter sub-mode does not trigger SwitchToModels
    Given a ProviderSettingsView in List mode with filter_mode true
    And the filter draft is "ant"
    When the user presses Tab with no modifiers
    Then handle_key returns ProviderSettingsEvent::Consumed
    And view.filter_mode is still true
    And view.filter is still "ant"

  Scenario: Tab in EditApiKey detail mode is silently Consumed
    Given a ProviderSettingsView in Detail::EditApiKey { draft: "sk-abc" } mode
    When the user presses Tab with no modifiers
    Then handle_key returns ProviderSettingsEvent::Consumed
    And view.mode is still Detail::EditApiKey
    And the draft is still "sk-abc"

  Scenario: Tab while delete-confirm dialog is open routes to dialog focus cycling
    Given a ProviderSettingsView in List mode
    And the delete-credentials ConfirmDialog is open
    When the user presses Tab with no modifiers
    Then handle_key returns ProviderSettingsEvent::Consumed
    And no SwitchToModels event is emitted

  Scenario: Direct unit invocation of Tab on a default List-mode view
    Given a freshly constructed ProviderSettingsView in default List mode
    When KeyEvent { code: KeyCode::Tab, modifiers: NONE } is dispatched
    Then handle_key returns ProviderSettingsEvent::SwitchToModels
    And no view-state field has changed from its default

  Scenario: Up then Tab then Down preserves arrow nav contract
    Given a ProviderSettingsView in List mode with 5 providers and cursor at index 2
    When the user presses Up
    And the user presses Tab
    And the user presses Down
    Then the Up keystroke returns ProviderSettingsEvent::Consumed and selected_index becomes 1
    And the Tab keystroke returns ProviderSettingsEvent::SwitchToModels and selected_index stays 1
    And the Down keystroke returns ProviderSettingsEvent::Consumed and selected_index becomes 2
