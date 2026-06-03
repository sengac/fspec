@done
@validation
@provider-settings
@tui
@regression
@ts-parity
@keyboard-navigation
@source-shape
@rust
@RPC-151
Feature: Provider settings list: testResult clears on ↑/↓ arrow navigation

  """
  [0] This card complements the full integration coverage already provided by RPC-159 (codelet/fspec-tui/tests/provider_settings_clear_test_result_on_nav_rpc159.rs). Pattern matches RPC-077 / RPC-149 / RPC-156 fast structural source-string regression-shape complement to slow integration tests.
  [1] Test file: codelet/fspec-tui/tests/rpc151_test_result_clear_on_nav_shape.rs — sub-millisecond execution, no key event simulation, just source-string scanning of list.rs handle_list_key body.
  [2] Source path: codelet/fspec-tui/src/views/provider_settings/list.rs handle_list_key (lines 30-87). Up arm at lines 65-77; Down arm at lines 78-87. RPC-159 comments at lines 66-70 and 79-80 document the TS-parity rationale.
  """

  Background: User Story
    As a fspec maintainer
    I want to have fast regression-shape tests pinning the test_result-clear-on-arrow-nav behavior in list.rs
    So that the TS-parity 'clear only on actual movement' contract cannot silently regress without paying the full ratatui integration-test compile cost on every CI run

  Scenario: handle_list_key contains exactly two clear_test_result call sites
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the body must contain exactly 2 occurrences of "clear_test_result("

  Scenario: KeyCode::Up arm clears test_result only on actual movement
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I extract the KeyCode::Up arm body inside handle_list_key
    Then the arm body must contain "let before = view.selected_index;"
    And the arm body must contain "view.move_clamped(-1);"
    And the arm body must contain "if view.selected_index != before {"
    And the arm body must contain "view.clear_test_result();"

  Scenario: KeyCode::Down arm clears test_result only on actual movement
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I extract the KeyCode::Down arm body inside handle_list_key
    Then the arm body must contain "let before = view.selected_index;"
    And the arm body must contain "view.move_clamped(1);"
    And the arm body must contain "if view.selected_index != before {"
    And the arm body must contain "view.clear_test_result();"

  Scenario: Non-arrow arms must NOT clear test_result
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I extract each non-arrow arm body (Enter, Tab, Esc, '/', d/D) inside handle_list_key
    Then the Enter arm body must NOT contain "clear_test_result("
    And the Tab arm body must NOT contain "clear_test_result("
    And the Esc arm body must NOT contain "clear_test_result("
    And the '/' filter-mode arm body must NOT contain "clear_test_result("
    And the d/D arm body must NOT contain "clear_test_result("
