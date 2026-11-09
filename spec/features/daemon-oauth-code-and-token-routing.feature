@BUG-634
@high
@tui
@providers
@authentication
@integration-test
Feature: Daemon OAuth code and token routing
  """
  WebSocket delegates to the same OAuth RPC methods as embedded, preserving
  daemon errors and interactive device deadlines. Browser launching belongs
  to the client and is not an OAuth browser RPC forwarding responsibility.
  Tests use the shared real WebSocket fixture and temporary credentials;
  invalid inputs stop before any provider network request.
  """

  Background: User Story
    As a user connected to a daemon
    I want OAuth code and token operations to reach my daemon
    So that authentication works without embedded transport

  Scenario: Headless login returns PKCE and validates pasted state
    Given a client connected to a real WebSocket daemon
    When Anthropic headless login starts
    Then it returns a fresh authorization URL and PKCE verifier
    And completing with a mismatched state returns daemon CSRF validation

  Scenario: Token reads and clearing use daemon credential storage
    Given daemon OAuth credentials in a temporary directory
    When the client reads and clears Anthropic tokens
    Then the persisted tokens are removed and subsequent reads report absent
    And clearing absent tokens succeeds
    And malformed credentials return the daemon read error

  Scenario: Device operations preserve daemon validation errors
    Given a client connected to a real WebSocket daemon
    When device start and polling request an unsupported provider
    Then both return the daemon unsupported provider error
    And Copilot polling rejects a malformed device handle
    And Copilot start rejects a malformed enterprise URL without network access

  Scenario: Disconnected OAuth operations fail explicitly
    Given a WebSocket client without an active connection
    When it requests code device or token operations
    Then every operation returns a disconnected error
