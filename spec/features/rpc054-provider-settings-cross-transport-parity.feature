@done
@session-management
@provider-settings
@agent-view
@rpc
@tui
@rust
@RPC-054
Feature: /provider provider-credentials cross-transport parity (Embedded + WebSocket)

  """
  Mirrors RPC-049 / RPC-050 cross-transport parity tests for the new provider-credentials RPC surface. Both EmbeddedFspecBackend and WebSocketFspecBackend are constructed against the SAME StubSessionManagerHandle; identical scripted calls flow through each transport and the stub's per-method call counters confirm both round-trips landed.
  """

  Background: User Story
    As a Rust ratatui frontend developer
    I want the new credential-surface RPCs to behave identically across embedded + WebSocket transports
    So that the AgentView can rely on a single FspecBackend abstraction regardless of how the user is connected

  Scenario: Embedded and WebSocket transports both reach the same StubSessionManagerHandle
    Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When set_provider_credentials("openai", ApiKey{"sk-1"}) is called via the embedded transport
    And set_provider_credentials("openai", ApiKey{"sk-2"}) is called via the WebSocket transport
    Then the stub's set_provider_credentials_calls counter equals 2

  Scenario: Embedded and WebSocket test_provider_connection both reach the stub
    Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When test_provider_connection("openai") is called via the embedded transport
    And test_provider_connection("openai") is called via the WebSocket transport
    Then the stub's test_provider_connection_calls counter equals 2
    And both calls returned a TestConnectionResult with success=true
