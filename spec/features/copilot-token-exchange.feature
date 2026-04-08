@done
@authentication
@providers
@PROV-057
Feature: Copilot GitHub OAuth to Copilot API token exchange
  """
  PROV-057 L2 (exchange half): After OAuth completes, the long-lived gho_*
  token must be exchanged at GET /copilot_internal/v2/token for a
  short-lived (~25 min) Copilot API token before any request is sent to
  api.githubcopilot.com. Lives in codelet/providers/src/copilot/token_exchange.rs
  and is exercised by token_exchange_tests.rs.

  Request shape (critical details):
  - Authorization: token <gho_*> (NOT Bearer)
  - Editor-Version, Editor-Plugin-Version, User-Agent, Accept: application/json
  """

  Background: User Story
    As a fspec user
    I want the provider to exchange my GitHub OAuth token for a short-lived Copilot API token
    So that requests to api.githubcopilot.com carry a valid Copilot credential instead of a 401 gho_* token

  @copilot
  @token-exchange
  Scenario: GitHub OAuth token is exchanged for a short-lived Copilot token before any API call
    Given a valid github_oauth_token is stored in copilot_auth.json
    And no copilot_token is currently cached
    When CopilotProvider issues a chat completion request
    Then a GET request is sent to "https://api.github.com/copilot_internal/v2/token"
    And the request uses header "Authorization: token <gho_*>"
    And the request includes headers "Editor-Version", "Editor-Plugin-Version", "User-Agent" and "Accept: application/json"
    And the response is parsed for "token", "expires_at", and "endpoints.api"
    And the parsed values are persisted to copilot_auth.json before the chat request is sent
