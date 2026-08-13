@done
@authentication
@providers
@PROV-057
Feature: Copilot provider token refresh and endpoints.api routing
  """
  PROV-057 L2 (runtime half): CopilotProvider must refresh the cached
  Copilot token when it is within 60 seconds of expiry, reuse it otherwise,
  and honour the endpoints.api URL returned by the token-exchange response
  (so enterprise deployments hit copilot-api.<their-host> instead of a
  hard-coded api.githubcopilot.com). Lives in
  rust/providers/src/copilot/provider.rs and is exercised by
  provider_tests.rs.
  """

  Background: User Story
    As a fspec user
    I want the Copilot provider to transparently refresh tokens and route to the correct endpoint
    So that long sessions and enterprise deployments both work without user intervention

  @copilot
  @token-refresh
  Scenario: Cached Copilot token is refreshed when within 60 seconds of expiry
    Given copilot_auth.json contains a copilot_token with copilot_token_expires_at set to 30 seconds from now
    When CopilotProvider issues a chat completion request
    Then the provider re-calls the token exchange endpoint before sending the chat request
    And copilot_auth.json is updated with the new copilot_token and expires_at

  @copilot
  @token-refresh
  Scenario: Cached Copilot token is reused when not near expiry
    Given copilot_auth.json contains a copilot_token with copilot_token_expires_at set to 20 minutes from now
    When CopilotProvider issues a chat completion request
    Then the provider does NOT re-call the token exchange endpoint
    And the cached copilot_token is used as the Bearer token for the request

  @copilot
  @endpoints-api
  @enterprise
  Scenario: Chat requests honour the endpoints.api URL from the token exchange response
    Given copilot_auth.json contains an "endpoints_api" value of "https://copilot-api.ghe.example.com"
    When CopilotProvider issues a chat completion request
    Then the request URL host is "copilot-api.ghe.example.com"
    And the request URL host is NOT "api.githubcopilot.com"
