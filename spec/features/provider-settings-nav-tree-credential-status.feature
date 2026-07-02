@done
@PROV-098
@tui
@ts-parity
@provider-settings
@configuration
@rust
Feature: Provider Settings nav tree shows credential status (masked key, source, not configured)
  """
  PROV-098 — The RPC-103/349 rich nav tree dropped credential display data.
  ProviderDisplayInfo (nav_item.rs) gains masked_key: Option<String> and
  source: Option<String>; projection.rs::project_one copies them verbatim from
  the backend ProviderCredentialInfo (populated by PROV-099); and
  list_nav_render.rs::row_kind_and_label appends the credential annotation to
  the Provider header row and the ApiKey child row, mirroring the TS
  ProviderSettingsPanel render shape (✓ {masked} [{source}] / (not configured)).

  DISPLAY masked_key/source priority in project_one (NOT the backend field):
  1. backend info.masked_key.is_some() (env api key)  -> copy info.masked_key/info.source verbatim
  2. else if is_oauth && info.configured (OAuth login) -> masked_key=Some("OAuth"), source=Some(oauth_label)
  3. else                                              -> None, None
  oauth_label (oauthProviderLabels.ts): anthropic->"Claude", codex->"ChatGPT",
  github-copilot->"GitHub Copilot", fallback->"OAuth".

  Render rule (TS parity, src/tui/components/ProviderSettingsPanel.tsx:594-608,734-746):
  Provider row:  "{name} ✓ {masked} [{source}]"  when display masked_key is Some
  "{name} ✓ {masked}"             when source is None
  "{name} (not configured)"        when display masked_key is None
  ApiKey  row:   "API Key ✓ {masked} [{source}]" when display masked_key is Some
  "API Key (not set)"              when display masked_key is None

  CRITICAL: has_oauth_tokens is computed from the BACKEND info.masked_key.is_none()
  (PROV-099, UNCHANGED) — it reads the backend field, not the new DISPLAY field —
  so an OAuth-logged-in provider renders BOTH the synthetic "✓ OAuth [label]"
  header annotation AND its separate "Logout from OAuth [..]" child row. An
  OAuth-logged-in provider must NEVER show "(not configured)". render_nav_items
  is left untouched, so the RPC-150 source-shape guard and the RPC-158 inline
  test-result decoration (painted at end_x AFTER the row label) remain valid.
  """

  Scenario: ProviderDisplayInfo defaults masked_key and source to None
    Given a default-constructed ProviderDisplayInfo
    Then its masked_key field is None
    And its source field is None

  Scenario: Projection copies env api-key masked_key and source verbatim from the backend record
    Given a ProviderCredentialInfo for "openai" whose masked_key is Some "sk-••••••••cdef" and source is Some "env"
    When project_display_infos projects the credential list
    Then the resulting openai ProviderDisplayInfo masked_key is Some "sk-••••••••cdef"
    And the resulting openai ProviderDisplayInfo source is Some "env"

  Scenario: Projection synthesizes OAuth display masked_key and source for an OAuth-logged-in provider
    Given a ProviderCredentialInfo for "codex" of credential type "oauth" that is configured with masked_key None
    When project_display_infos projects the credential list
    Then the resulting codex ProviderDisplayInfo masked_key is Some "OAuth"
    And the resulting codex ProviderDisplayInfo source is Some "ChatGPT"
    And the resulting codex ProviderDisplayInfo has_oauth_tokens is true

  Scenario: Configured provider row shows checkmark, masked key and source tag
    Given a ProviderSettings view loaded with an "openai" provider whose masked_key is Some "sk-••••••••cdef" and source is Some "env"
    When the nav tree is rendered into a buffer
    Then the openai provider row contains "OpenAI API ✓ sk-••••••••cdef [env]"

  Scenario: Configured provider row with no source omits the bracket tag
    Given a ProviderSettings view loaded with an "openai" provider whose masked_key is Some "sk-••••••••cdef" and source is None
    When the nav tree is rendered into a buffer
    Then the openai provider row contains "OpenAI API ✓ sk-••••••••cdef"
    And the openai provider row does not contain "["

  Scenario: Unconfigured provider row shows the not-configured annotation
    Given a ProviderSettings view loaded with a "cohere" provider whose masked_key is None
    When the nav tree is rendered into a buffer
    Then the cohere provider row contains "Cohere (not configured)"

  Scenario: Configured ApiKey child row shows checkmark, masked key and source tag
    Given a ProviderSettings view loaded with a "gemini" provider whose masked_key is Some "AIza••••••••H3Ck" and source is Some "env"
    And the "gemini" provider row is expanded
    When the nav tree is rendered into a buffer
    Then the gemini ApiKey child row contains "API Key ✓ AIza••••••••H3Ck [env]"

  Scenario: Unconfigured ApiKey child row shows the not-set annotation
    Given a ProviderSettings view loaded with a "gemini" provider whose masked_key is None
    And the "gemini" provider row is expanded
    When the nav tree is rendered into a buffer
    Then the gemini ApiKey child row contains "API Key (not set)"

  Scenario: OAuth-logged-in provider header shows synthetic OAuth annotation plus a separate logout row
    Given a ProviderSettings view loaded with an "anthropic" provider configured via OAuth with backend masked_key None
    And the "anthropic" provider row is expanded
    When the nav tree is rendered into a buffer
    Then the anthropic provider header row contains "Anthropic ✓ OAuth [Claude]"
    And no row in the buffer contains "Anthropic (not configured)"
    And a separate child row contains "Logout from OAuth [Anthropic]"

  Scenario: OAuth-logged-in codex header matches the screenshot case
    Given a ProviderSettings view loaded with a "codex" provider configured via OAuth with backend masked_key None
    When the nav tree is rendered into a buffer
    Then the codex provider header row contains "Codex (ChatGPT) ✓ OAuth [ChatGPT]"
