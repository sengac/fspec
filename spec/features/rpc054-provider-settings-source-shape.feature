@done
@RPC-054
@rust
@tui
@source-shape
@provider-settings
Feature: ProviderSettingsView — source-shape invariants

  """
  Source-shape regression tests that lock in the architectural decisions
  for ProviderSettingsView. The view MUST follow the full-screen mode-view
  pattern from RPC-026's ResumeSessionView, reuse the same render helpers,
  and use ConfirmDialog for destructive actions. The slash command registry
  MUST contain exactly one provider-related entry (no /providers alias).
  """

  Background: User Story
    As a developer reviewing the Rust TUI source tree
    I want the ProviderSettingsView source layout, import list, and
    slash command registry to remain consistent with the RPC-026
    full-screen mode-view pattern
    So that future contributors cannot regress the architecture without
    a compile-time failure caught by the source-shape tests

  @slash-registry
  @no-alias
  Scenario: SlashCommandAction enum contains no Providers variant
    Given codelet/fspec-tui/src/views/agent/slash_commands.rs after the 2026-06-01 revision
    When the source is parsed for SlashCommandAction variants
    Then the enum contains "Provider" exactly once
    And the enum does NOT contain a "Providers" variant
    And the SLASH_COMMANDS const contains exactly one entry whose action is SlashCommandAction::Provider
    And no entry in SLASH_COMMANDS has the name "providers"

  @slash-dispatch
  @no-alias
  Scenario: dispatch_slash_commands.rs has no Providers arm
    Given codelet/fspec-tui/src/app/dispatch_slash_commands.rs after the 2026-06-01 revision
    When the file is read
    Then it contains exactly one arm matching "SlashCommandAction::Provider =>"
    And it does NOT contain "SlashCommandAction::Providers"
    And it does NOT contain "| SlashCommandAction::Providers"

  @view-module
  Scenario: The ProviderSettingsView module exists at the expected path
    Given the workspace root
    When codelet/fspec-tui/src/views/provider_settings/mod.rs is read
    Then the file exists
    And it declares a pub struct ProviderSettingsView
    And it declares a pub enum ProviderSettingsMode with variants List and Detail
    And it declares a pub enum DetailSub with variants Summary, EditApiKey, OAuthNotice

  @view-imports
  @rpc-026-parity
  Scenario: ProviderSettingsView imports the canonical full-screen helpers
    Given codelet/fspec-tui/src/views/provider_settings/mod.rs
    When the use statements are parsed
    Then the file imports ratatui::widgets::Clear
    And the file imports crate::components::scroll_viewport::ensure_visible
    And the file does NOT import crate::components::scroll_viewport::wrap_index (RPC-157: clamped nav)
    And the file imports crate::views::agent::confirm_dialog::ConfirmDialog
    And the file imports crate::views::agent::mode_view_render::{render_title_with_count, render_footer_hint}

  @view-imports
  @forbidden
  Scenario: ProviderSettingsView does NOT import Block / Borders
    Given codelet/fspec-tui/src/views/provider_settings/mod.rs
    When the use statements are parsed
    Then the file does NOT import ratatui::widgets::Block
    And the file does NOT import ratatui::widgets::Borders

  @view-render
  @rpc-026-parity
  Scenario: render() starts with Clear and uses the 4-constraint Layout
    Given codelet/fspec-tui/src/views/provider_settings/mod.rs
    When the source of ProviderSettingsView::render is inspected
    Then the first statement is "Clear.render(area, buf);"
    And the body splits area with Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])

  @file-size
  Scenario: Every file under views/provider_settings/ stays under 300 lines
    Given codelet/fspec-tui/src/views/provider_settings/
    When the file sizes are measured
    Then every .rs file under that directory is < 300 lines

  @action-bus
  Scenario: components/mod.rs declares the new ConfirmDeleteProviderCredentials action
    Given codelet/fspec-tui/src/components/mod.rs
    When the Action enum is parsed
    Then it contains a variant ConfirmDeleteProviderCredentials(String)
    And the existing variants OpenProviderSettingsView, CloseProviderSettingsView, ProviderCredentialsLoaded, SaveProviderCredentials, TestProviderConnection, ProviderTestComplete, RefreshProviderModels, ProviderModelsRefreshed, DeleteProviderCredentials, ProviderSettingsStatus all remain

  @filter-mode
  @ts-parity
  Scenario: ProviderSettingsView declares filter + filter_mode fields
    Given codelet/fspec-tui/src/views/provider_settings/mod.rs
    When the ProviderSettingsView struct is parsed
    Then the struct contains a field "filter: String" (or equivalent type holding the filter string)
    And the struct contains a field "filter_mode: bool" (or equivalent flag for whether filter input is active)

  @filter-mode
  @ts-parity
  Scenario: List mode key dispatcher routes "/" to enter filter mode
    Given codelet/fspec-tui/src/views/provider_settings/mod.rs
    When the list mode key handler is inspected
    Then a "/" keypress in List mode (with filter_mode false) sets filter_mode to true
    And does NOT insert the "/" character anywhere

  @filter-mode
  @ts-parity
  Scenario: Esc-cascade clears filter before closing the view
    Given codelet/fspec-tui/src/views/provider_settings/mod.rs
    When the list mode Esc handler is inspected
    Then the Esc handler first checks for filter_mode = true → clears filter and sets filter_mode = false
    And else if filter is non-empty → clears filter and stays in List
    And else (filter_mode false, filter empty) → emits ProviderSettingsEvent::Close
