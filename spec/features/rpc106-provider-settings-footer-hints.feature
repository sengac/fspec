@done
@provider-settings
@agent-view
@ts-parity
@tui
@RPC-106
Feature: Provider settings: TS-parity footer hints with context-sensitive per-row-type strings and bullet separators
  """
  Test plan: new `rust/fspec-tui/tests/provider_settings_footer_hints.rs` integration suite — 10 assertions (provider/oauth-status/oauth-login/api-key/profile/add-profile/None branches, bullet-not-pipe, lowercase-colon, render-into-bottom-row-with-dim-style). Pure string + widget tests using ratatui `TestBackend`. Depends on RPC-103 for the flat NavItem tree and on RPC-104 for `RowKind` — both are sibling cards in the same RPC-054 fan-out, so the test crate can refer to their exports.
  Implementation:
  - introduce `rust/fspec-tui/src/views/provider_settings/footer_hints.rs` exposing `FOOTER_COMMON` and `footer_hint_for(Option<RowKind>) -> String`. `mod.rs::footer_hint()` becomes a wrapper that looks up the currently-selected NavItem (via the flat tree from RPC-103), maps it to a `RowKind`, and passes it through `footer_hint_for`. Detail-mode hints (EditApiKey, OAuthNotice, Summary) keep their dedicated strings but adopt the bullet (`·`) separator + lowercase-colon style for visual consistency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Footer hint string is derived from the currently selected nav-item kind via a `getFooterHints(itemType)` dispatch (providerSettingsHelpers.ts:16-33), NOT from the higher-level panel mode; navigating between rows must update the footer in real time
  #   2. Every footer hint string is composed as '<per-row-type prefix> · ' + FOOTER_COMMON, where FOOTER_COMMON = '/ filter · Tab: Switch to models · Esc: close' (providerSettingsHelpers.ts:11); the separator is the U+00B7 MIDDLE DOT bullet character, NOT a pipe '|'
  #   3. Per-row-type prefixes are EXACTLY: provider → 'Enter: expand', oauth-status → 'Enter: logout', oauth-login → 'Enter: start login', api-key → 'Enter: edit · d: delete', profile → 'Enter: edit · d: delete', add-profile → 'Enter: create' (providerSettingsHelpers.ts:18-29); the default fallback (no selected item) returns FOOTER_COMMON only
  #   4. Key labels in hints use lowercase keys with colon separators ('Enter:', 'd:', 'Esc:'), NOT uppercase pipe-separated forms ('D Delete', 'Esc Cancel'); this matches the TS Ink convention at providerSettingsHelpers.ts:18-29
  #   5. The footer is rendered as a single dim-styled line through `render_footer_hint(footer_area, buf, hint)` (mod.rs:245); long hints are clipped at the right edge — no wrap, no truncation ellipsis
  #
  # EXAMPLES:
  #   1. When the user has 'OpenAI' (provider row) selected the footer shows: 'Enter: expand · / filter · Tab: Switch to models · Esc: close' rendered dim across the bottom row
  #   2. When the user navigates Down onto the 'API key' child row under OpenAI the footer changes to: 'Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close'
  #   3. When the user navigates Down onto the '+ Create new profile' row the footer changes to: 'Enter: create · / filter · Tab: Switch to models · Esc: close'
  #   4. When the user is on a github-copilot 'Sign in to GitHub' oauth-login row the footer reads: 'Enter: start login · / filter · Tab: Switch to models · Esc: close' (NOT the generic List-mode hint the current Rust impl shows)
  #
  # ========================================
  Background: User Story
    As a Rust frontend user navigating /provider
    I want to see the footer hint line update based on the currently selected nav-item kind (provider, api-key, profile, oauth-login, oauth-status, add-profile) using the same per-row-type strings and bullet separators as the TS Ink reference
    So that I always know the right keybinds for whatever row I'm on (Enter: edit · d: delete for an api-key, Enter: expand for a provider, etc.) without having to memorise a generic combined hint

  Scenario: Provider row selection yields the "Enter: expand" hint with FOOTER_COMMON suffix
    Given the ProviderSettingsView has a Provider nav-item selected
    When I read the footer hint
    Then the hint equals "Enter: expand · / filter · Tab: Switch to models · Esc: close"

  Scenario: ApiKey row selection yields the "Enter: edit · d: delete" hint with FOOTER_COMMON suffix
    Given the ProviderSettingsView has an ApiKey nav-item selected
    When I read the footer hint
    Then the hint equals "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"

  Scenario: Profile row selection yields the "Enter: edit · d: delete" hint with FOOTER_COMMON suffix
    Given the ProviderSettingsView has a Profile nav-item selected
    When I read the footer hint
    Then the hint equals "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"

  Scenario: AddProfile row selection yields the "Enter: create" hint with FOOTER_COMMON suffix
    Given the ProviderSettingsView has an AddProfile nav-item selected
    When I read the footer hint
    Then the hint equals "Enter: create · / filter · Tab: Switch to models · Esc: close"

  Scenario: OAuthLogin row selection yields the "Enter: start login" hint with FOOTER_COMMON suffix
    Given the ProviderSettingsView has an OAuthLogin nav-item selected
    When I read the footer hint
    Then the hint equals "Enter: start login · / filter · Tab: Switch to models · Esc: close"

  Scenario: OAuthStatus row selection yields the "Enter: logout" hint with FOOTER_COMMON suffix
    Given the ProviderSettingsView has an OAuthStatus nav-item selected
    When I read the footer hint
    Then the hint equals "Enter: logout · / filter · Tab: Switch to models · Esc: close"

  Scenario: Empty nav-item list (no selection) falls back to FOOTER_COMMON only
    Given the ProviderSettingsView has no nav-items
    When I read the footer hint
    Then the hint equals "/ filter · Tab: Switch to models · Esc: close"

  Scenario: Separator is U+00B7 MIDDLE DOT, never the pipe character
    Given the ProviderSettingsView has a Provider nav-item selected
    When I read the footer hint
    Then the hint contains the character "·" (U+00B7)
    And the hint does not contain the character "|"

  Scenario: Keybind labels use lowercase colon style, never uppercase pipe style
    Given the ProviderSettingsView has an ApiKey nav-item selected
    When I read the footer hint
    Then the hint contains "Enter:"
    And the hint contains "d: delete"
    And the hint does not contain "D Delete"
    And the hint does not contain "Esc Cancel"

  Scenario: Footer updates in real time when the user navigates between rows of different kinds
    Given the ProviderSettingsView lists OpenAI with one profile and the user starts on the OpenAI provider row
    When I move selection Down onto the ApiKey row
    Then the footer hint equals "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    When I move selection Down onto the AddProfile row
    Then the footer hint equals "Enter: create · / filter · Tab: Switch to models · Esc: close"

  Scenario: Footer hint is rendered into a single bottom row via render_footer_hint
    Given the ProviderSettingsView has a Provider nav-item selected
    When I render the view into a 80x10 buffer
    Then the bottom buffer row reads "Enter: expand · / filter · Tab: Switch to models · Esc: close" left-aligned

  Scenario: footer_hint_for(None) returns FOOTER_COMMON verbatim
    Given the canonical FOOTER_COMMON string "/ filter · Tab: Switch to models · Esc: close"
    When I call footer_hint_for with None
    Then the returned string equals FOOTER_COMMON exactly
