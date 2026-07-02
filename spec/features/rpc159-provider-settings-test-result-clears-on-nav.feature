@done
@tui
@provider-settings
@rust
@ts-parity
@agent-view
@RPC-159
Feature: Provider settings: testResult clears on Up/Down arrow navigation
  """
  Test parity vs TS: TS clears testResult only inside the `if (key.upArrow && selectedIndex > 0)` and `if (key.downArrow && selectedIndex < navItems.length - 1)` blocks — i.e. only when movement actually happens. Rust mirrors this by detecting whether move_clamped changed selected_index and clearing only on actual movement.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Up arrow that moves focus clears the inline test_result
  #   2. Down arrow that moves focus clears the inline test_result
  #   3. Up arrow at index 0 (no movement) does NOT clear the test_result
  #   4. Down arrow at the last visible row (no movement) does NOT clear the test_result
  #   5. Non-navigation keys (Enter, Tab, '/', Esc, action keys) do NOT clear the test_result
  #   6. Clearing on navigation is pure state — leaves selected_index, scroll_offset, mode, filter, filter_mode, expanded, nav_items, and status fields alone (apart from the index/scroll move that Up/Down already do)
  #   7. Up/Down arrows in filter_mode are routed to handle_filter_key and do NOT clear test_result via the list-mode path
  #
  # EXAMPLES:
  #   1. Given a view with 3 visible providers, selected_index=1, test_result=Some(openai/Ok), When Down arrow is pressed, Then selected_index=2 and test_result=None
  #   2. Given a view with 3 visible providers, selected_index=2, test_result=Some(openai/Err), When Up arrow is pressed, Then selected_index=1 and test_result=None
  #   3. Given selected_index=0 with test_result=Some(openai/Testing), When Up arrow is pressed, Then selected_index remains 0 and test_result remains Some(openai/Testing)
  #   4. Given selected_index at last visible row with test_result=Some(openai/Ok), When Down arrow is pressed, Then selected_index unchanged and test_result still Some(openai/Ok)
  #   5. Given test_result=Some(openai/Ok), When Enter is pressed on a Provider row, Then expansion toggles and test_result remains Some(openai/Ok)
  #   6. Given test_result=Some(openai/Ok), When Tab is pressed, Then the returned event is SwitchToModels and test_result remains Some(openai/Ok)
  #   7. Given test_result=Some(openai/Ok), When '/' is pressed to activate filter mode, Then filter_mode is true and test_result remains Some(openai/Ok)
  #   8. Given filter_mode=true and test_result=Some(openai/Ok), When Up arrow is pressed, Then routing goes through handle_filter_key and test_result is unchanged (the list-mode clear path is not entered)
  #   9. Given selected_index=1 with test_result=None, When Down arrow is pressed, Then selected_index=2 and test_result is still None (no spurious mutation)
  #   10. Given a view with 0 visible providers (no nav items) and test_result=Some(openai/Ok), When Up or Down arrow is pressed, Then selected_index stays 0 and test_result remains Some(openai/Ok)
  #   11. Given selected_index=1, scroll_offset=0, test_result=Some(openai/Ok), When Down arrow is pressed and visible_rows constrains scroll, Then both scroll_offset adjusts AND test_result clears (no field is left out)
  #
  # ========================================
  Background: User Story
    As a user navigating the provider settings list
    I want to press Up or Down arrows after running a connection test
    So that the inline test result clears so it does not visually persist onto a different focused row

  @navigation
  @clear
  Scenario: Down arrow that moves focus clears the inline test_result
    Given a ProviderSettingsView in List mode with three providers
    And selected_index is 1
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Down arrow key is dispatched to handle_list_key
    Then selected_index is 2
    And test_result is None

  @navigation
  @clear
  Scenario: Up arrow that moves focus clears the inline test_result
    Given a ProviderSettingsView in List mode with three providers
    And selected_index is 2
    And test_result is set to Some(provider_id="openai", status=Err{message="boom"})
    When the Up arrow key is dispatched to handle_list_key
    Then selected_index is 1
    And test_result is None

  @navigation
  @boundary
  Scenario: Up arrow at index 0 does not clear test_result
    Given a ProviderSettingsView in List mode with three providers
    And selected_index is 0
    And test_result is set to Some(provider_id="openai", status=Testing)
    When the Up arrow key is dispatched to handle_list_key
    Then selected_index is still 0
    And test_result is still Some(provider_id="openai", status=Testing)

  @navigation
  @boundary
  Scenario: Down arrow at last visible row does not clear test_result
    Given a ProviderSettingsView in List mode with three providers
    And selected_index is at the last visible nav-item index
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Down arrow key is dispatched to handle_list_key
    Then selected_index is unchanged
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})

  @non-navigation
  @preserve
  Scenario: Enter on a Provider row toggles expansion and preserves test_result
    Given a ProviderSettingsView in List mode with three providers
    And the focused nav item is a Provider row
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Enter key is dispatched to handle_list_key
    Then the focused provider's expansion is toggled
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})

  @non-navigation
  @preserve
  Scenario: Tab in list mode emits SwitchToModels and preserves test_result
    Given a ProviderSettingsView in List mode with three providers
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Tab key is dispatched to handle_list_key
    Then the returned event is SwitchToModels
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})

  @non-navigation
  @preserve
  Scenario: Slash activates filter mode and preserves test_result
    Given a ProviderSettingsView in List mode with three providers
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the '/' key is dispatched to handle_list_key
    Then filter_mode is true
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})

  @filter-mode
  @routing
  Scenario: Up arrow while filter_mode is true does not enter the list-mode clear path
    Given a ProviderSettingsView with three providers
    And filter_mode is true
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Up arrow key is dispatched to handle_list_key
    Then the call is routed through handle_filter_key
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})

  @navigation
  @no-op-clear
  Scenario: Down arrow that moves focus with test_result already None remains None
    Given a ProviderSettingsView in List mode with three providers
    And selected_index is 1
    And test_result is None
    When the Down arrow key is dispatched to handle_list_key
    Then selected_index is 2
    And test_result is still None

  @navigation
  @empty-list
  Scenario: Up or Down arrow with zero visible providers does not clear test_result
    Given a ProviderSettingsView in List mode with zero visible providers
    And selected_index is 0
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Down arrow key is dispatched to handle_list_key
    Then selected_index is still 0
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Up arrow key is dispatched to handle_list_key
    Then selected_index is still 0
    And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})

  @navigation
  @scroll-and-clear
  Scenario: Down arrow that scrolls and moves focus both adjusts scroll and clears test_result
    Given a ProviderSettingsView in List mode with many providers requiring scrolling
    And selected_index is 1
    And scroll_offset is 0
    And visible_rows is 2
    And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    When the Down arrow key is dispatched to handle_list_key
    Then selected_index is 2
    And scroll_offset has advanced
    And test_result is None
