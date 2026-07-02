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
Feature: Provider settings list: remove Rust-only keybinds (r/R, wrap-around, PageUp/PageDown/Home/End)
  """
  RPC-149 was already resolved during RPC-157 — list.rs comment at line 8 explicitly notes "no wrap-around, no PgUp/PgDn/Home/End — RPC-157". This card adds fast regression-shape tests pinning the absence so the Rust-only deviations cannot regress without paying the TUI compile cost on every CI run.
  Pattern: source-string structural assertions over codelet/fspec-tui/src/views/provider_settings/list.rs and mod.rs. Mirrors RPC-077 fast regression-shape complement to the slow integration test (skeleton_invariants pattern).
  Test file: codelet/fspec-tui/tests/rpc149_list_mode_keybinds_shape.rs. Sub-millisecond execution — no key event simulation, just source-string scanning.
  """

  Background: User Story
    As a fspec developer
    I want to see Rust provider-settings list mode bind ONLY ↑/↓/Enter/Esc/Tab//d/D (no r/R refresh-models, no wrap-around, no PageUp/PageDown/Home/End)
    So that list-mode key bindings match the TS ProviderSettings contract exactly with no Rust-only deviations

  Scenario: list.rs handle_list_key has no refresh-models r/R keybind arms
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the source must NOT contain "KeyCode::Char('r')"
    And the source must NOT contain "KeyCode::Char('R')"

  Scenario: list.rs handle_list_key has no PageUp/PageDown/Home/End jump-key arms
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the source must NOT contain "KeyCode::PageUp"
    And the source must NOT contain "KeyCode::PageDown"
    And the source must NOT contain "KeyCode::Home"
    And the source must NOT contain "KeyCode::End"

  Scenario: move_clamped clamps at boundary instead of wrapping
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/mod.rs
    When I locate the move_clamped function body
    Then the source must contain ".clamp("
    And the source must NOT contain "% total"
    And the source must NOT contain "% max"

  Scenario: handle_list_key match arms enumerate exactly the TS contract surface
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    When I scan the handle_list_key match body
    Then the source must contain "KeyCode::Esc"
    And the source must contain "KeyCode::Char('/')"
    And the source must contain "KeyCode::Tab"
    And the source must contain "KeyCode::Up"
    And the source must contain "KeyCode::Down"
    And the source must contain "KeyCode::Enter"
    And the source must contain "KeyCode::Char('d') | KeyCode::Char('D')"
