@done
@tui
@ts-parity
@agent-view
@provider-settings
@RPC-104
Feature: Provider settings: per-row icons, indents and color coding

  """
  Test plan: new `codelet/fspec-tui/tests/provider_settings_row_render.rs` integration suite using ratatui `TestBackend` + `Buffer::diff` for cell-level assertions. Eight scenarios (1) yellow selection band on provider, (2) cyan on profile, (3) magenta on oauth-login, (4) green on oauth-status, (5) green on add-profile, (6) 4-space inner indent on every child kind, (7) no inner indent on provider, (8) ▼/▶ glyph flip on `expanded`. Pure widget tests — no NAPI, no async, no real terminal — so they run inside `cargo test -p codelet-fspec-tui` in <100ms.
  Implementation:
  - extract row painting into a new `row_render.rs` module (≤180 LoC) keyed off a `RowKind` enum (Provider, Profile, OauthLogin, OauthStatus, ApiKey, AddProfile). Each kind owns a `row_style(kind, selected) -> Style` returning the fg/bg pair from the visual matrix. A separate `icons.rs` exposes glyph constants (EXPANDED, COLLAPSED, FOLDER, KEY, PLUS, INDENT, SEL, NOSEL). The existing `list::render_list` loop becomes a `RowKind` dispatch — adding the new file keeps `list.rs` under the 300-LoC ceiling once colours/inline decorations are wired in.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Provider rows render the selection marker '> ' when selected and '  ' (2 spaces) when unselected (ProviderSettingsPanel.tsx:591); child rows (profile, api-key, oauth-login, oauth-status, add-profile) prepend an additional 4-space inner indent after the marker (L654, L694, L713, L734, L766)
  #   2. Provider rows show the expand glyph '▼ ' when provider.isExpanded === true and '▶ ' when collapsed (ProviderSettingsPanel.tsx:592); no other nav-item kind shows an expand glyph
  #   3. Each child nav-item kind carries its own icon: profile = '📁 ' (L654), oauth-login = '🔑 ' (L694), api-key = '🔑 ' (L734), add-profile = '+ ' (L766); oauth-status has no prefix icon — the icon lives inside the dynamic label string
  #   4. Selected rows invert the row tint into a background band: provider/api-key=yellow→black (L587-588, L729-730), profile=cyan→black (L649-650), oauth-login=magenta→black (L689-690), oauth-status/add-profile=green→black (L708-709, L761-762); unselected rows use the same tint as foreground on the default black background
  #   5. On the provider row, inline status decorations are appended after the name: ' ✓ <masked-key> [<source>]' (green/dim) when configured (L594-604), ' (not configured)' (gray) when not (L605-610), and ' (N profile[s])' (dim) for openai only when profileCount > 0 (L611-617); on a selected row the decoration fg flips to black to remain readable on the inverted band
  #
  # EXAMPLES:
  #   1. An unselected configured OpenAI provider row with 2 profiles renders: '  ▶ OpenAI ✓ sk-…abcd [env] (2 profiles)' with the leading 2-space marker, collapsed glyph, white name, green check+masked-key, dim [env] source tag, and dim (2 profiles) badge
  #   2. A selected expanded oauth provider 'Anthropic' (claude) row renders: '> ▼ Anthropic (not configured)' with bg:yellow fg:black on the whole line and fg:black on the '(not configured)' suffix (no green check because hasKey is false)
  #   3. A selected profile row under OpenAI named 'dev' with baseUrl 'http://localhost:8080' renders: '>     📁 dev → http://localhost:8080' with bg:cyan fg:black on the marker/icon/name and fg:black-dim on the ' → http://localhost:8080' arrow span
  #   4. An unselected oauth-login row for github-copilot with label 'Sign in to GitHub' renders: '      🔑 Sign in to GitHub' with fg:magenta on the whole line (4-space inner indent after the 2-space marker)
  #
  # ========================================

  Background: User Story
    As a Rust frontend user opening /provider
    I want to see each nav-item row painted with the same icon, indent, and colour band as the TS Ink reference
    So that the screen is visually identical to the TS implementation and provides instant kind-recognition (yellow = api-key/provider, magenta = oauth, cyan = profile, green = oauth-status/add-profile)


  Scenario: Selected provider row paints a yellow background band
    Given a ProviderSettings row of kind Provider labelled "OpenAI"
    And the row is in the selected state
    When the row is painted into a TestBackend buffer of width 40
    Then every cell on the row carries bg=Yellow and fg=Black
    And the row uses Modifier::BOLD


  Scenario: Unselected provider row paints white foreground on default background
    Given a ProviderSettings row of kind Provider labelled "OpenAI"
    And the row is in the unselected state
    When the row is painted into a TestBackend buffer of width 40
    Then the name span carries fg=White and bg=Reset
    And no Modifier::REVERSED flag is set on the row


  Scenario: Selected profile row paints a cyan background band
    Given a ProviderSettings row of kind Profile labelled "dev"
    And the row is in the selected state
    When the row is painted into a TestBackend buffer of width 40
    Then every cell on the row carries bg=Cyan and fg=Black


  Scenario: Selected oauth-login row paints a magenta background band
    Given a ProviderSettings row of kind OauthLogin labelled "Sign in to GitHub"
    And the row is in the selected state
    When the row is painted into a TestBackend buffer of width 40
    Then every cell on the row carries bg=Magenta and fg=Black


  Scenario: Selected oauth-status row paints a green background band
    Given a ProviderSettings row of kind OauthStatus labelled "✓ Signed in as user"
    And the row is in the selected state
    When the row is painted into a TestBackend buffer of width 40
    Then every cell on the row carries bg=Green and fg=Black


  Scenario: Selected add-profile row paints a green background band
    Given a ProviderSettings row of kind AddProfile labelled "Add Profile"
    And the row is in the selected state
    When the row is painted into a TestBackend buffer of width 40
    Then every cell on the row carries bg=Green and fg=Black


  Scenario: Selected api-key row paints a yellow background band
    Given a ProviderSettings row of kind ApiKey labelled "API Key"
    And the row is in the selected state
    When the row is painted into a TestBackend buffer of width 40
    Then every cell on the row carries bg=Yellow and fg=Black


  Scenario: Unselected child rows are tinted by their kind on the default background
    Given a ProviderSettings row of kind Profile labelled "dev"
    And the row is in the unselected state
    When the row is painted into a TestBackend buffer of width 40
    Then the label span carries fg=Cyan and bg=Reset


  Scenario: Every non-provider row prepends a 4-space inner indent after the selection prefix
    Given a ProviderSettings row of kind Profile labelled "dev"
    When the row is painted into a TestBackend buffer of width 40
    Then cells at indices 0 and 1 are the selection prefix "  "
    And cells at indices 2, 3, 4, and 5 are spaces forming the inner indent


  Scenario: Provider rows have no 4-space inner indent — the expand glyph follows the marker directly
    Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=true
    When the row is painted into a TestBackend buffer of width 40
    Then cells at indices 0 and 1 are the selection prefix "  "
    And cell at index 2 is the expanded glyph "▼"
    And cell at index 2 is NOT a space


  Scenario: Provider row paints the ▼ expanded glyph when expanded is true
    Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=true
    When the row is painted into a TestBackend buffer of width 40
    Then the expand glyph at cell index 2 is "▼"


  Scenario: Provider row paints the ▶ collapsed glyph when expanded is false
    Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=false
    When the row is painted into a TestBackend buffer of width 40
    Then the expand glyph at cell index 2 is "▶"


  Scenario: Selected row prefix is "> " and unselected prefix is "  "
    Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=false
    When the row is painted selected into a TestBackend buffer of width 40
    Then cells at indices 0 and 1 are "> "
    When the same row is painted unselected into a TestBackend buffer of width 40
    Then cells at indices 0 and 1 are "  "


  Scenario: Profile row carries the 📁 folder icon directly after the inner indent
    Given a ProviderSettings row of kind Profile labelled "dev"
    When the row is painted into a TestBackend buffer of width 40
    Then the icon cell at index 6 starts with "📁"


  Scenario: OauthLogin row carries the 🔑 key icon directly after the inner indent
    Given a ProviderSettings row of kind OauthLogin labelled "Sign in to GitHub"
    When the row is painted into a TestBackend buffer of width 40
    Then the icon cell at index 6 starts with "🔑"


  Scenario: AddProfile row carries the "+ " glyph directly after the inner indent
    Given a ProviderSettings row of kind AddProfile labelled "Add Profile"
    When the row is painted into a TestBackend buffer of width 40
    Then the icon cell at index 6 is "+"
