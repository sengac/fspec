@done
@ts-parity
@provider-settings
@tui
@RPC-350
Feature: Provider settings list-mode visual parity regressions vs TypeScript
  """
  R1 must NOT modify the shared render_title_with_count (mode_view_render.rs) which other full-screen views depend on; use the render_full_screen_scaffold_with_title title-closure variant for a provider-specific two-span title.
  R4 requires a span-aware row painter: render_row currently applies a single Style to the whole row and has a wide-glyph band-repair loop (row_render.rs:148-150) that must be preserved so the full-width background band stays intact under emoji continuation cells.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The Provider Settings title renders as two segments — the name 'Provider Settings' in bold yellow and the ' (N items)' count in dim gray — matching ProviderSettingsPanel.tsx:550-555, without altering the shared blue title used by other full-screen views
  #   2. R2: When the openai provider has one or more profiles, its expanded header row appends a dim ' (N profile)' / ' (N profiles)' badge (pluralized), only for openai, matching ProviderSettingsPanel.tsx:611-617
  #   3. R3: The add-profile pseudo-row label text is 'Create new profile' (not 'Add Profile'); the '+ ' glyph is still supplied by the row prefix, matching ProviderSettingsPanel.tsx:766
  #   4. R4: Provider and api-key rows paint inline status decorations as per-color segments — name white, '✓ masked-key' green, '[source]' dim, '(not configured)'/'(not set)' gray — and on a selected row every segment foreground flips to black over the colour band, matching ProviderSettingsPanel.tsx:586-633 and 728-749
  #   5. R5: Existing non-provider full-screen views (Resume Session, Search History, Model Selector) keep their current title styling — the parity change is scoped to the provider settings view only
  #
  # EXAMPLES:
  #   1. Title row of the rendered view reads 'Provider Settings (19 items)' where cells 0-16 ('Provider Settings') are fg Yellow + BOLD and the ' (19 items)' cells are dim/DarkGray
  #   2. OpenAI expanded with one profile 'qwen' renders header '▼ OpenAI API (not configured) (1 profile)' with the '(1 profile)' span dim; a second profile makes it '(2 profiles)'
  #   3. The selected green add-profile row reads '> + Create new profile' (was '> + Add Profile')
  #   4. Unselected 'Google Gemini ✓ AIza••••••••H3Ck [env]' row paints the name white, '✓ AIza••••••••H3Ck' green, and '[env]' dim gray on the default background
  #   5. Unselected 'Cohere (not configured)' row paints the name white and '(not configured)' gray; the api-key child empty state uses '(not set)' gray instead
  #   6. A selected configured provider row paints a yellow background band with ALL segments (name, green-key, dim-source) flipped to black foreground for readability
  #
  # ========================================
  Background: User Story
    As a fspec user opening the Rust /provider settings screen
    I want to see the same colors, labels and badges the TypeScript implementation renders
    So that the ported Rust TUI is faithful visual parity with the original and nothing looks broken or inconsistent

  @tui
  @provider-settings
  @ts-parity
  Scenario: Title renders the name in bold yellow and the item count in dim gray
    Given the provider settings view has 19 nav items
    When the view is rendered to the terminal buffer
    Then the title row reads "Provider Settings (19 items)"
    And the "Provider Settings" name segment is foreground yellow and bold
    And the " (19 items)" count segment is foreground dim gray

  @tui
  @provider-settings
  @ts-parity
  Scenario: Expanded OpenAI provider with profiles shows a dim pluralized profile badge
    Given the openai provider is expanded with one profile named "qwen"
    When the view is rendered to the terminal buffer
    Then the openai header row contains the suffix " (1 profile)"
    And the " (1 profile)" badge segment is rendered dim
    And a second openai profile changes the badge to " (2 profiles)"

  @tui
  @provider-settings
  @ts-parity
  Scenario: Add-profile row label reads "Create new profile"
    Given the openai provider is expanded
    And the add-profile row is selected
    When the view is rendered to the terminal buffer
    Then the add-profile row label text is "Create new profile"
    And the row is prefixed with the "+ " glyph and selection marker

  @tui
  @provider-settings
  @ts-parity
  Scenario: Configured unselected provider row paints per-color status segments
    Given an unselected configured "Google Gemini" provider row with masked key "AIza••••••••H3Ck" and source "env"
    When the view is rendered to the terminal buffer
    Then the provider name segment is foreground white
    And the "✓ AIza••••••••H3Ck" masked-key segment is foreground green
    And the "[env]" source segment is foreground dim gray

  @tui
  @provider-settings
  @ts-parity
  Scenario: Unconfigured rows use gray empty-state text with distinct provider and api-key wording
    Given an unselected unconfigured "Cohere" provider row
    When the view is rendered to the terminal buffer
    Then the provider name segment is foreground white
    And the "(not configured)" segment is foreground gray
    And an unconfigured api-key child row uses "(not set)" in gray instead

  @tui
  @provider-settings
  @ts-parity
  Scenario: Selected configured provider row flips all segments to black over the colour band
    Given a selected configured provider row with a masked key and source
    When the view is rendered to the terminal buffer
    Then the entire row paints a yellow background band
    And the name, masked-key and source segments are all foreground black

  @tui
  @provider-settings
  @ts-parity
  Scenario: Non-provider full-screen views keep their existing title styling
    Given the Resume Session full-screen view with 5 sessions
    When that view is rendered to the terminal buffer
    Then its title row "Resume Session (5 available)" keeps the shared blue bold styling
    And the provider-specific two-span title change does not affect it
