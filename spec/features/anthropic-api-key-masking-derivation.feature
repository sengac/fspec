@done
@provider-settings
@configuration
@providers
@PROV-099
Feature: Anthropic API key masking derivation in list_providers_info
  """
  management.rs::list_providers_info: change the masked_key/source derivation from `match auth_type { ApiKey if available && !env_var.is_empty() => mask }` to mask whenever `available && !env_var.is_empty()` and the env var holds a non-empty value, independent of AuthType. Codex/github-copilot have empty env_var so stay None.
  Offline tests: management-layer tests are #[serial] and use an RAII env guard to save/restore ANTHROPIC_API_KEY + FSPEC_HOME; the OAuth-auth-file case writes claude_auth.json into a tempdir pointed to by FSPEC_HOME (read_claude_auth_sync -> oauth::fspec_home honors it). No network.
  Regression guard: RPC-108 list_providers_info_keeps_masked_key_none_for_oauth_only_providers clears ANTHROPIC_API_KEY so anthropic stays masked_key=None (still passes).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. list_providers_info derives masked_key/source for any provider whose declared env_var is actually set, regardless of AuthType (so anthropic's OAuth auth_type no longer suppresses masking)
  #   2. OAuth-only providers with an empty env_var (codex, github-copilot) still carry masked_key=None and source=None
  #
  # EXAMPLES:
  #   1. ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnop, no auth file -> anthropic masked_key=Some("sk-ant-••••••••mnop"), source=Some("env")
  #   2. claude_auth.json present (FSPEC_HOME injected), ANTHROPIC_API_KEY unset -> anthropic available=true, masked_key=None, source=None
  #   3. codex and github-copilot (empty env_var) -> masked_key=None, source=None
  #
  # ========================================
  Background: User Story
    As a developer using the pure-Rust fspec Provider Settings screen
    I want to see my ANTHROPIC_API_KEY shown as a masked env-sourced api-key credential
    So that I am not wrongly told I am logged in via OAuth when I only set an env API key

  Scenario: Anthropic env API key is masked as an env-sourced credential
    Given ANTHROPIC_API_KEY is set to "sk-ant-api03-abcdefghijklmnop" with no OAuth auth file
    When list_providers_info is called
    Then the anthropic entry masked_key is Some("sk-ant-••••••••mnop")
    And the anthropic entry source is Some("env")

  Scenario: Anthropic configured by an OAuth auth file carries no masked key
    Given a claude_auth.json OAuth file exists under an injected FSPEC_HOME and ANTHROPIC_API_KEY is unset
    When list_providers_info is called
    Then the anthropic entry available is true
    And the anthropic entry masked_key is None
    And the anthropic entry source is None

  Scenario: OAuth-only providers with an empty env var carry no masked key
    Given no provider environment variables are set
    When list_providers_info is called
    Then the codex entry masked_key is None and source is None
    And the github-copilot entry masked_key is None and source is None
