@high
@providers
@authentication
@BUG-634
Feature: Anthropic client-local browser authorization
  """
  The client reuses the standalone Anthropic browser form and validation without
  exchanging or storing credentials. The daemon completes token exchange.
  """

  Background: User Story
    As a remote fspec user
    I want the original browser-hosted Anthropic paste form on my computer
    So that credentials remain owned by the daemon

  Scenario: Capture browser authorization without claiming token success
    Given a client-local Anthropic authorization listener
    When I open its browser URL and submit a valid code and state
    Then the original form links to the authorization URL
    And completion returns the code with state and matching PKCE verifier
    And the browser tells me to return to fspec for the result

  Scenario: Reject missing and mismatched state
    Given a client-local Anthropic authorization listener
    When I submit a code with missing or mismatched state
    Then the browser shows an error and completion fails with a typed error

  Scenario: Empty submissions and unknown routes do not terminate authorization
    Given a client-local Anthropic authorization listener
    When I visit an unknown route or submit an empty code
    Then I can still cancel through the original browser cancel link
    And completion reports cancellation

  Scenario: Dropping authorization closes the local listener
    Given a client-local Anthropic authorization listener
    When I drop the pending authorization
    Then its local port is released

  Scenario: Cancelling a pending completion closes active browser connections
    Given a client-local Anthropic authorization listener with an incomplete browser request
    When I cancel its pending completion
    Then its listener and active browser connection are closed

  Scenario: Authorization expires after five minutes
    Given a client-local Anthropic authorization listener
    When five minutes pass without completion
    Then completion reports a typed timeout and releases the local port

  Scenario: Failed HTTP connections do not terminate authorization
    Given a client-local Anthropic authorization listener
    When an incomplete or malformed HTTP request disconnects
    Then the failed connection is rejected and its real error is logged
    And a subsequent browser GET and valid code submission complete authorization
