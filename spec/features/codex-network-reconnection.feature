@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex network reconnection
  """
  PROV-160 supersedes the PROV-154 and CMPCT-081 prohibitions on automatic same-transport replacement. One typed Codex-owned recovery policy retries full-current-DAG requests within the same validated generation. Five stream retries are separate from cancellable connection-failure waits of 5, 10, 20, 40 then 60 seconds for ALL sessions (explicit user decision). Preserve the 120-second raw-network idle deadline and existing bounded rejection advice and OAuth refresh. Never adopt HTTP fallback, delta mode, synthetic Continue or generic CLI recovery. Buffer tool calls until successful checkpoint publication; discard uncommitted calls between attempts. Session-scoped lifecycle notifications reuse StreamEvent Status and distinguish provider waiting from daemon disconnection.
  """

  Background: User Story
    As a Codex session owner
    I want to recover temporary network failures on the selected transport
    So that my current turn and completed tool results survive

  Scenario: Accepted SSE interruptions discard uncommitted tool calls
    Given a Codex response containing partial text and uncommitted tool calls
    When the accepted SSE stream disconnects and its replacement completes
    Then only the successful attempt publishes a checkpoint and dispatches its tools

  Scenario: Five stream retries exhaust without checkpoint publication
    Given six Codex accepted responses that disconnect before completion
    When Codex consumes its five stream retries
    Then a typed exhausted failure ends the turn without completion or checkpoint mutation

  Scenario: Idle deadlines consume the stream budget rather than waiting offline
    Given a Codex server that repeatedly sends partial text and stalls
    When raw network idle deadlines expire despite UI polling
    Then five bounded replacements occur and abandoned partial text is retracted on exhaustion

  Scenario: Permanent failures never enter network recovery
    Given Codex requests rejected by authentication or permanent errors or malformed events
    When the provider reports those failures
    Then no replacement or offline waiting occurs

  Scenario: Recovery revalidates generation and credential authority
    Given a Codex request waiting to reconnect
    When its committed generation or credential authority changes during the wait
    Then the stale attempt sends nothing and returns a terminal authority failure
