@done
@authentication
@providers
@PROV-057
Feature: Copilot select_model re-detects credentials after login
  """
  PROV-057 bonus stale-cache fix: ProviderManager snapshots
  ProviderCredentials::detect() once at construction. After OAuth login
  writes copilot_auth.json, select_model must call
  ProviderCredentials::detect() AGAIN before the has_credentials check
  so the freshly-written credential is honoured without a process
  restart. Lives in rust/providers/src/manager.rs and is exercised
  end-to-end by copilot_select_model_stale_cache_test.rs.
  """

  Background: User Story
    As a fspec user
    I want to select a github-copilot model in the same session I just logged in from
    So that I don't have to restart fspec after completing the Copilot OAuth flow

  @copilot
  @stale-cache
  Scenario: Selecting a github-copilot model right after login succeeds without restart
    Given ProviderManager was constructed before copilot_auth.json existed
    And copilot_auth.json has just been written by the OAuth login flow
    When the user calls select_model("github-copilot/gpt-4o") in the same session
    Then ProviderCredentials::detect() is re-invoked before the has_credentials check
    And the selection succeeds without a "requires credentials" error

  @copilot
  @end-to-end
  Scenario: User selects github-copilot model after logging in and chats successfully
    Given the user has completed the Copilot OAuth flow with the corrected client_id
    And copilot_auth.json contains a valid github_oauth_token
    When the user selects "github-copilot/gpt-4o" from the model picker
    And the user sends a chat message
    Then select_model succeeds (no "requires credentials" error)
    And the token exchange step mints a Copilot token
    And the agent loop dispatches to CopilotProvider
    And a streamed response is returned to the user
