@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex network failure classification
  """
  Network-down setup errors on either selected transport are connection failures,
  including Unix ENETDOWN and Windows WSAENETDOWN. They use cancellable offline
  waiting rather than terminating the turn. Accepted WebSocket protocol, UTF-8,
  capacity and unsupported binary-message failures are permanent, not interrupted
  streams. Only connection closure and transient network I/O use bounded stream
  recovery; permanent failures never send a replacement or publish a checkpoint.
  """

  Background: User Story
    As a Codex session owner
    I want network outages distinguished from permanent transport failures
    So that recovery waits only when retrying can restore my turn

  Scenario: Network-down setup errors enter cancellable offline waiting on either transport
    Given HTTP or WebSocket setup reports that the network interface is down
    When Codex classifies the setup error and applies its recovery policy
    Then it waits for network reception until cancellation instead of terminating

  Scenario: Permanent accepted WebSocket failures never send replacement requests
    Given an accepted Codex WebSocket response with invalid protocol or UTF-8 or excessive frame size or a binary message
    When Codex reads the invalid response
    Then the original failure ends the turn without reconnecting or sending a replacement
    And no completion or checkpoint is published

  Scenario: Abrupt WebSocket connection loss retains same-transport recovery
    Given an accepted Codex WebSocket response whose connection closes without a closing handshake
    When a replacement WebSocket request completes successfully
    Then bounded stream recovery publishes one completion and checkpoint on the selected transport
