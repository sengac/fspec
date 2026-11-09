@done
@provider-settings
@tui
@validation
@regression
@ts-parity
@keyboard-navigation
@source-shape
@rust
@RPC-149
Feature: Provider settings list: no Rust-only r/R refresh-models or wrap-around (PageUp/PageDown/Home/End added by RPC-353)
  """
  RPC-149 was already resolved during RPC-157 (no r/R refresh-models, no wrap-around). RPC-353 later added PageUp/PageDown/Home/End paging to list mode (list.rs header: "no wrap-around; PgUp/PgDn/Home/End added by RPC-353"). This card adds fast regression-shape tests pinning the absence of r/R and wrap-around and the presence of the RPC-353 paging keys, so the list-mode surface cannot regress without paying the TUI compile cost on every CI run.
  Pattern: source-string structural assertions over rust/fspec-tui/src/views/provider_settings/list.rs and mod.rs. Mirrors RPC-077 fast regression-shape complement to the slow integration test (skeleton_invariants pattern).
  Test file: rust/fspec-tui/tests/rpc149_list_mode_keybinds_shape.rs. Sub-millisecond execution — no key event simulation, just source-string scanning.
  """

  Background: User Story
    As a fspec developer
    I want to see Rust provider-settings list mode bind ONLY ↑/↓/Enter/Esc/Tab//d/D (no r/R refresh-models, no wrap-around, no PageUp/PageDown/Home/End)
    So that list-mode key bindings match the TS ProviderSettings contract exactly with no Rust-only deviations

  Scenario: list.rs handle_list_key has no refresh-models r/R keybind arms
    Given I read the source of rust/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the source must NOT contain "KeyCode::Char('r')"
    And the source must NOT contain "KeyCode::Char('R')"

  Scenario: list.rs handle_list_key binds PageUp/PageDown/Home/End (RPC-353)
    Given I read the source of rust/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the source MUST contain "KeyCode::PageUp" (RPC-353)
    And the source MUST contain "KeyCode::PageDown" (RPC-353)
    And the source MUST contain "KeyCode::Home" (RPC-353)
    And the source MUST contain "KeyCode::End" (RPC-353)

  Scenario: move_clamped clamps at boundary instead of wrapping
    Given I read the source of rust/fspec-tui/src/views/provider_settings/mod.rs
    When I locate the move_clamped function body
    Then the source must contain ".clamp("
    And the source must NOT contain "% total"
    And the source must NOT contain "% max"

  Scenario: handle_list_key match arms enumerate exactly the TS contract surface
    Given I read the source of rust/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the source must contain "KeyCode::Esc"
    And the source must contain "KeyCode::Char('/')"
    And the source must contain "KeyCode::Tab"
    And the source must contain "KeyCode::Up"
    And the source must contain "KeyCode::Down"
    And the source must contain "KeyCode::Enter"
    And the source must contain "KeyCode::Char('d') | KeyCode::Char('D')"
