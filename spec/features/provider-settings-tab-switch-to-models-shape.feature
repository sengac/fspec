@done
@validation
@provider-settings
@tui
@regression
@ts-parity
@keyboard-navigation
@source-shape
@rust
@RPC-152
Feature: Provider settings list: Tab keybind emits SwitchToModels event

  """
  [0] This card complements the full integration coverage already provided by RPC-160 (codelet/fspec-tui/tests/provider_settings_tab_switch_to_models_rpc160.rs). Pattern matches RPC-077 / RPC-149 / RPC-151 / RPC-156 fast structural source-string regression-shape complement to slow integration tests.
  [1] Test file: codelet/fspec-tui/tests/rpc152_tab_switch_to_models_shape.rs — sub-millisecond execution, no key event simulation, just source-string scanning of mod.rs (enum) + list.rs (Tab arm + filter_mode gate).
  [2] Source paths: codelet/fspec-tui/src/views/provider_settings/mod.rs (enum `ProviderSettingsEvent` lines 57-70 — `SwitchToModels,` variant at line 69) + codelet/fspec-tui/src/views/provider_settings/list.rs (`handle_list_key` line 30, `if view.filter_mode { return handle_filter_key(view, key); }` line 34, `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels,` line 64, `handle_filter_key` line 161).
  """

  Background: User Story
    As a fspec maintainer
    I want to have fast regression-shape tests pinning the Tab → SwitchToModels event variant and its list-mode dispatch
    So that the TS-parity Tab keybind cannot silently regress (lose the variant, swap to Close, or move outside the list-mode handler) without paying the full ratatui integration-test compile cost on every CI run

  Scenario: ProviderSettingsEvent enum declares SwitchToModels variant
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/mod.rs
    When I extract the body of the "pub enum ProviderSettingsEvent" declaration
    Then the enum body must contain "SwitchToModels,"

  Scenario: handle_list_key body binds Tab to SwitchToModels via an expression arm
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the body must contain "KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"

  Scenario: handle_list_key checks filter_mode BEFORE dispatching Tab to SwitchToModels
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key body
    Then the body must contain "if view.filter_mode {"
    And the body must contain "KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"
    And the offset of "if view.filter_mode {" must be less than the offset of "KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"

  Scenario: handle_filter_key body does NOT emit SwitchToModels
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I extract the handle_filter_key function body
    Then the function body must NOT contain "SwitchToModels"
