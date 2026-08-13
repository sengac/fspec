@done
@authentication
@providers
@PROV-057
Feature: Copilot OAuth device flow uses the well-known Copilot client_id
  """
  PROV-057 L1: The device-code OAuth request must use the well-known
  Copilot client_id (Iv1.b507a08c87ecfe98) rather than opencode's
  Ov23li8tweQw6odWQebz. Lives in rust/providers/src/copilot/oauth_types.rs
  and is exercised by oauth_device_code.rs.
  """

  Background: User Story
    As a fspec user
    I want the Copilot OAuth device flow to use the correct client_id
    So that GitHub accepts my device-code request and issues a real gho_* token

  @copilot
  @oauth
  Scenario: OAuth device flow uses the well-known Copilot client_id
    Given the Copilot OAuth device flow is invoked from the TUI
    When the device-code request is sent to GitHub
    Then the request body contains client_id "Iv1.b507a08c87ecfe98"
    And the request body does not contain client_id "Ov23li8tweQw6odWQebz"
