@PROV-026
Feature: Anthropic provider routing and model availability with subscription auth
  """
  claude_auth.rs: Add read_claude_auth_sync() using std::fs (not tokio::fs) for use in sync contexts (credentials.rs, manager.rs). Follows same pattern as Codex's read_codex_auth() which is already sync.
  credentials.rs (providers): Add has_claude_auth() that calls read_claude_auth_sync() to check for OAuth tokens — exactly mirrors has_codex_auth(). Include in claude_available: std::env::var(ANTHROPIC_API_KEY).is_ok() || std::env::var(CLAUDE_CODE_OAUTH_TOKEN).is_ok() || has_claude_auth()
  manager.rs get_claude(): Check read_claude_auth_sync() first. If Some(auth) found with non-empty access_token and refresh_token, use ClaudeProvider::from_oauth_tokens(auth.access_token, auth.refresh_token, Some(0), CLAUDE_TOKEN_ENDPOINT_BASE, model). Fall through to existing ClaudeProvider::new_with_model() if no auth file.
  resolver.rs (napi/credentials): Add fallback for anthropic provider that checks claude_auth.json via read_claude_auth_sync(). When found, return access_token as the credential and set CLAUDE_CODE_OAUTH_TOKEN env var. This ensures session_manager.rs resolve_and_set_env_var() works for OAuth-only users.
  modelInitializationService.ts: Add async checkClaudeOAuthTokens() using claudeOauthGetTokens() from NAPI. In buildCloudSections(), check both hasCodexOAuth and hasClaudeOAuth. When hasClaudeOAuth is true, override hasCredentials=true for the anthropic section (even if no API key exists). No synthetic section needed — models are already under 'anthropic' in models.dev.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When Claude OAuth tokens exist in claude_auth.json (from PROV-021/022/025 login flows), the Anthropic section in the model selector must show hasCredentials=true even without an ANTHROPIC_API_KEY env var or stored API key
  #   2. credentials.rs (providers) must check claude_auth.json for OAuth tokens in claude_available detection — add read_claude_auth_sync() using std::fs (like Codex's sync read_codex_auth) since credential detection runs in sync context
  #   3. manager.rs get_claude() must check for OAuth tokens in claude_auth.json and use ClaudeProvider::from_oauth_tokens() with expires_in_secs=Some(0) to force immediate refresh — mirrors CodexProvider::new() pattern (line 121 of codex/mod.rs)
  #   4. modelInitializationService.ts buildCloudSections() must check for Claude OAuth tokens via claudeOauthGetTokens() NAPI binding (async) and set hasCredentials=true for the Anthropic section when tokens exist — mirrors checkCodexOAuthTokens() pattern
  #   5. When user selects an Anthropic model from model selector, session creates with 'anthropic/model-id' format — no synthetic section needed (unlike Codex which extracts from OpenAI). Claude models are already under 'anthropic' provider in models.dev
  #   6. Users with BOTH ANTHROPIC_API_KEY env var AND Claude OAuth tokens: OAuth takes precedence in manager.rs get_claude() — check claude_auth.json first, fall back to env var. In model selector, Anthropic section shows hasCredentials=true from either source
  #   7. resolver.rs (napi/credentials) for anthropic provider must also check claude_auth.json as a fallback credential source — when OAuth tokens found, set access_token as CLAUDE_CODE_OAUTH_TOKEN env var for provider initialization
  #   8. Non-OAuth providers (OpenAI, Gemini, etc.) are completely unaffected — Claude OAuth token check only runs for Anthropic provider. Codex OAuth behavior unchanged.
  #   9. Persisted model 'anthropic/claude-sonnet-4-20250514' with OAuth tokens must be restored correctly on startup — model selector restores it via the existing findModelInSections() mechanism since Anthropic section now shows hasCredentials=true
  #
  # EXAMPLES:
  #   1. Happy path - OAuth only: User logs in with Claude OAuth (PROV-025) → claude_auth.json created → opens model selector → models.dev returns 'anthropic' provider with claude-sonnet-4, claude-opus-4 models → checkClaudeOAuthTokens() returns true → Anthropic section shows with hasCredentials=true → user selects claude-sonnet-4 → session creates with 'anthropic/claude-sonnet-4-20250514' → get_claude() reads claude_auth.json → uses from_oauth_tokens() with Some(0) → RefreshingClaudeClient refreshes token → API request succeeds
  #   2. No OAuth tokens, no API key: User has not logged in via OAuth and has no ANTHROPIC_API_KEY → checkClaudeOAuthTokens() returns false → claude_available=false in credentials.rs → Anthropic section has hasCredentials=false → Anthropic not shown in model selector (unchanged behavior)
  #   3. Both OAuth and API key: User has ANTHROPIC_API_KEY env var AND OAuth tokens in claude_auth.json → Anthropic section has hasCredentials=true (from either source) → model selector shows Anthropic models → session creates with 'anthropic/model' → get_claude() checks claude_auth.json first, finds OAuth tokens → uses from_oauth_tokens() (OAuth takes precedence over API key)
  #   4. API key only (no OAuth): User has ANTHROPIC_API_KEY but no claude_auth.json → credentials.rs detects claude_available=true from env var → model selector shows Anthropic section → session creates → get_claude() finds no claude_auth.json → falls back to ClaudeProvider::new_with_model() which uses env var (unchanged behavior)
  #   5. Persisted model restore with OAuth: User had 'anthropic/claude-sonnet-4-20250514' as lastUsedModel → restarts app → checkClaudeOAuthTokens() returns true → Anthropic section has hasCredentials=true → findSectionForPersistedModel finds matching section → model restored on startup
  #   6. OAuth token expired on session create: claude_auth.json has expired access_token → get_claude() passes Some(0) for expires_in_secs → from_oauth_tokens() creates RefreshingClaudeClient → first API request triggers refresh → refresh succeeds → request completes (PROV-023 handles this transparently)
  #   7. Credential resolver for session creation: session_manager.rs calls resolve_and_set_env_var('anthropic') → resolver checks credentials store, then env vars, then .env → none found → checks claude_auth.json as final fallback → finds OAuth tokens → sets CLAUDE_CODE_OAUTH_TOKEN env var with access_token → provider initialization succeeds
  #
  # ASSUMPTIONS:
  #   1. All Rust OAuth modules (claude_oauth.rs, claude_auth.rs, claude_refreshing_client.rs) are already implemented and tested by PROV-020/021/022/023. All NAPI bindings (claude_oauth.rs) are implemented by PROV-024. TUI provider settings are implemented by PROV-025. This card only wires up credential detection, model selector visibility, and session routing.
  #   2. NAPI .node binary already exports claudeOauthGetTokens (from PROV-024 build). index.d.ts already declares it.
  #
  # ========================================
  Background: User Story
    As a user with Claude Max/Pro subscription
    I want to see and use Claude models after OAuth login
    So that use my subscription without needing an API key

  Scenario: Anthropic models appear in model selector when OAuth tokens exist
    Given I have authenticated with Claude via OAuth
    And claude_auth.json exists with valid access and refresh tokens
    And models.dev returns the anthropic provider with Claude models
    And I have no ANTHROPIC_API_KEY environment variable set
    When models are loaded for the model selector
    Then I should see the Anthropic section with Claude models
    And the Anthropic section should have hasCredentials true

  Scenario: No Anthropic section when no OAuth tokens and no API key
    Given I have not authenticated with Claude via OAuth
    And I have no ANTHROPIC_API_KEY environment variable set
    When models are loaded for the model selector
    Then I should not see the Anthropic section in the model selector

  Scenario: Both API key and OAuth tokens show Anthropic section
    Given I have authenticated with Claude via OAuth
    And I have an ANTHROPIC_API_KEY environment variable set
    And models.dev returns the anthropic provider with Claude models
    When models are loaded for the model selector
    Then I should see the Anthropic section with Claude models
    And the Anthropic section should have hasCredentials true

  Scenario: API key only shows Anthropic section without OAuth
    Given I have not authenticated with Claude via OAuth
    And I have an ANTHROPIC_API_KEY environment variable set
    And models.dev returns the anthropic provider with Claude models
    When models are loaded for the model selector
    Then I should see the Anthropic section with Claude models

  Scenario: Persisted Anthropic model restored on startup with OAuth tokens
    Given I have authenticated with Claude via OAuth
    And my last used model was anthropic/claude-sonnet-4-20250514
    And models.dev returns the anthropic provider with Claude models
    When models are loaded for the model selector
    Then the persisted model should be restored as the current model
    And the model providerId should be anthropic

  Scenario: Non-OAuth providers unaffected by Claude OAuth changes
    Given I have authenticated with Claude via OAuth
    And I have no OpenAI API key configured
    When models are loaded for the model selector
    Then I should not see any OpenAI section
    And Codex OAuth behavior should be unchanged
