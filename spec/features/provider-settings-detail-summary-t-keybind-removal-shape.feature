@done
@rpc-154
@rust
@source-shape
@keyboard-navigation
@ts-parity
@regression
@tui
@provider-settings
@validation
@RPC-154
Feature: Provider settings api-key edit: empty-Enter cancels silently (no validation, no Detail hop)
  """
  Existing rpc054 test `t inside Detail::Summary emits TestProviderConnection` (provider_settings_view_rpc054.rs:149) and `r inside Detail::Summary emits RefreshProviderModels` (line 173) — only the `t` test is superseded by RPC-154; the `r` test stays since RPC-154 explicitly scopes to `t` only
  RPC-163 (delete-key) test at provider_settings_api_key_delete_key_rpc163.rs:292-300 uses `t` to set up Testing state. Removing the `t` arm WILL BREAK that test setup. The test must be migrated to set view.test_result directly (or to drive TestProviderConnection via the dispatch layer, which still receives the action when called externally)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. handle_summary_key (codelet/fspec-tui/src/views/provider_settings/detail.rs) MUST NOT match KeyCode::Char('t') or KeyCode::Char('T') — there must be no `t` arm at all
  #   2. Pressing 't' (lowercase) in Detail::Summary mode MUST return ProviderSettingsEvent::Consumed with NO Action emitted and view.mode remains Detail::Summary with last_status preserved
  #   3. Pressing 'T' (uppercase) in Detail::Summary mode MUST return ProviderSettingsEvent::Consumed with NO Action::TestProviderConnection emitted (same parity as lowercase t)
  #   4. view.status MUST NOT be set to "Testing…" by any handle_summary_key path — the only writer to that string in handle_summary_key (the `t` arm) is removed
  #   5. The TS reference (src/tui/inputHandlers/listModeHandler.ts) MUST contain no `key.t` / `'t'.test` / `key === 't'` binding for TestProviderConnection — confirming the absence-in-source on the Rust side mirrors the absence on the TS canonical side
  #
  # EXAMPLES:
  #   1. View is in Detail::Summary { last_status: None } and user presses 't' — handle_key returns Consumed, no Action emitted, view stays in Detail::Summary { last_status: None }
  #   2. View is in Detail::Summary { last_status: Some(Testing) } (legacy state) and user presses 'T' uppercase — no second TestProviderConnection is emitted; the existing last_status is preserved
  #   3. Source-shape audit: grepping handle_summary_key body in detail.rs for the literal substring `KeyCode::Char('t')` or `KeyCode::Char('T')` returns zero matches
  #   4. Source-shape audit: handle_summary_key body in detail.rs MUST NOT contain Action::TestProviderConnection construction (the dispatch site of the `t` keybind)
  #   5. TS reference parity assertion: reading src/tui/inputHandlers/listModeHandler.ts shows no binding for `key.t` / `'t'`, confirming TS has no `t` keybind to mirror
  #
  # ========================================
  Background: User Story
    As a developer maintaining provider settings TS parity
    I want to ensure the Detail::Summary `t` (test connection) keybind from Rust does not exist (TS has no such keybind on any Detail screen)
    So that Rust ProviderSettings key handling matches the TS canonical surface, removing a Rust-only deviation

  Scenario: Pressing lowercase t in Detail::Summary is silently ignored
    Given a ProviderSettingsView seeded with one api_key provider "anthropic"
    And the view has been transitioned into Detail::Summary { last_status: None } for "anthropic"
    When the user presses KeyCode::Char('t')
    Then the returned ProviderSettingsEvent is Consumed
    And no Action::TestProviderConnection is emitted
    And view.mode remains Detail::Summary for provider "anthropic" with last_status None
    And view.status is the empty string

  Scenario: Pressing uppercase T in Detail::Summary is silently ignored even with existing Testing status
    Given a ProviderSettingsView seeded with one api_key provider "anthropic"
    And view.mode is manually constructed as Detail::Summary { last_status: Some(Testing) } for "anthropic"
    When the user presses KeyCode::Char('T')
    Then the returned ProviderSettingsEvent is Consumed
    And no Action::TestProviderConnection is emitted
    And view.mode remains Detail::Summary for "anthropic" with last_status Some(Testing) preserved

  Scenario: handle_summary_key source body contains zero KeyCode::Char('t') or KeyCode::Char('T') matches
    Given the file codelet/fspec-tui/src/views/provider_settings/detail.rs
    When the byte range delimited by "fn handle_summary_key(" through the next top-level "fn " is extracted
    Then the substring "KeyCode::Char('t')" occurs zero times in that range
    And the substring "KeyCode::Char('T')" occurs zero times in that range

  Scenario: handle_summary_key source body contains zero Action::TestProviderConnection construction sites
    Given the file codelet/fspec-tui/src/views/provider_settings/detail.rs
    When the byte range delimited by "fn handle_summary_key(" through the next top-level "fn " is extracted
    Then the substring "Action::TestProviderConnection" occurs zero times in that range
    And the substring "Testing…" (the legacy status text the `t` arm wrote) occurs zero times in that range

  Scenario: TS reference confirms no `t` keybind exists for TestProviderConnection
    Given the TS canonical file src/tui/inputHandlers/listModeHandler.ts (under .fspec/worktrees/3ce722ec-0b61-4601-813b-023909a2a45a/)
    When the file body is scanned for any of the substrings "key.t " / "key.t&&" / "input === 't'" / "input === 'T'"
    Then zero matches are found
    And this absence-in-TS justifies the absence-in-Rust required by RPC-154
