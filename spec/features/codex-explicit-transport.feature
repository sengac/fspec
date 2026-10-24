@done
@CMPCT-081
@PROV-160
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Remove automatic Codex WS downgrade and delta-to-full mode fallback
  """
  PROV-160 explicitly supersedes the accepted-WebSocket no-replacement criterion: typed bounded same-transport recovery is allowed, while rejected upgrades remain terminal and no transport downgrade, delta fallback or checkpoint mutation before success is permitted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Configured WS fails with typed transport diagnostics on rejected upgrade; zero automatic HTTP attempts or other transport/model changes follow failure.
  #   3. Preserve committed lineage and declared same-transport retry semantics. Explicit full mode's first request is valid and must not be confused with fallback.
  #   4. Support explicit full-current-DAG Codex requests only. Remove delta mode, baseline bootstrap and implicit delta-to-full downgrade paths; incompatible or removed modes must not send a request.
  #   5. Support explicit full-current-DAG requests only. Remove delta mode and its baseline bootstrap/downgrade paths rather than silently selecting full input.
  #
  # EXAMPLES:
  #   1. Given configured WS returning each of 404/405/426/501, when connecting, then typed failure occurs and the HTTP fixture records zero fallback requests.
  #   2. Given a removed delta-mode configuration, when requests are prepared, then unsupported mode sends nothing and explicitly selected full mode works.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Before finalizing CMPCT-081/078 delta tests, what explicit protocol establishes the initial compatible delta baseline and handles a durable generation reset, or is delta explicitly outside supported scope? No implicit delta-to-full bootstrap is permitted.
  #   A: Support explicit full-current-DAG Codex requests only. Remove delta mode, baseline bootstrap and implicit delta-to-full downgrade paths; incompatible or removed modes must not send a request.
  #
  # ========================================
  Background: User Story
    As a Codex session owner
    I want to send only explicitly selected full-current-DAG requests on the selected transport
    So that transport failures never trigger alternate request paths

  Scenario Outline: Rejected WebSocket upgrades never downgrade to HTTP
    Given an explicitly selected Codex WebSocket transport
    And the scripted provider rejects its upgrade with status <status>
    When I request a completion
    Then I receive a typed transport failure identifying the rejected upgrade
    And the scripted provider records zero HTTP completion requests
    And no completion or checkpoint mutation is published

    Examples:
      | status |
      | 404    |
      | 405    |
      | 426    |
      | 501    |

  Scenario: Every explicitly selected full-current-DAG WebSocket request sends full input
    Given an explicitly selected full-current-DAG Codex WebSocket session
    When I send its first request and a compatible continuation on the same socket
    Then both captured requests contain their complete current DAG input
    And neither request establishes or uses a previous-response delta baseline

  Scenario: Removed delta mode cannot silently become full input
    Given a Codex request explicitly selecting the removed delta mode
    When I prepare the provider request
    Then I receive an explicit unsupported-mode failure before transport activity
    And no full-input request or baseline bootstrap is sent

  Scenario: Accepted WebSocket failures recover only on the selected transport
    Given an explicitly selected Codex WebSocket request accepted by the provider
    When the scripted transport disconnects before successful completion
    Then a bounded replacement WebSocket request completes successfully
    And no HTTP request is sent and both WebSocket requests contain full input
    And the checkpoint changes only after the successful replacement
