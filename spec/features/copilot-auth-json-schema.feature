@done
@authentication
@providers
@PROV-057
Feature: Copilot auth.json schema separates GitHub OAuth and Copilot tokens
  """
  PROV-057 L2 (schema half): CopilotAuthJson must track github_oauth_token,
  copilot_token, copilot_token_expires_at, and endpoints_api as separate
  fields so the token-exchange layer can cache and refresh the short-lived
  Copilot API token without losing the long-lived GitHub OAuth token.
  Lives in rust/providers/src/copilot/auth.rs.
  """

  Background: User Story
    As a fspec user
    I want Copilot credentials persisted with separate GitHub and Copilot token fields
    So that the provider can refresh the short-lived Copilot token without re-doing the OAuth dance

  @copilot
  @auth-schema
  Scenario: CopilotAuthJson schema separates GitHub OAuth and Copilot tokens
    Given a successful Copilot OAuth login completes
    When copilot_auth.json is written to ~/.fspec/credentials
    Then the file contains a non-empty "github_oauth_token" field starting with "gho_" or "ghu_"
    And the file contains a "copilot_token" field that may be initially absent
    And the file contains a "copilot_token_expires_at" field that may be initially absent
    And the file contains an "endpoints_api" field that may be initially absent
    And the file mode is 0600 on Unix
