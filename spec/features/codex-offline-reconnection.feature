@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex offline reconnection
  """
  Connection failures wait cancellably for 5, 10, 20, 40 then 60 seconds in all sessions, separately from the five-stream-retry budget. Recovery preserves full-current-DAG input, completed tool results, selected transport, admission, generation and credential authority. Dropping the owning stream cancels setup, body reads and backoff without a detached retry task.
  """

  Background: User Story
    As a Codex session owner
    I want my original turn to survive lost network reception
    So that completed work is preserved without synthetic messages

  Scenario: Offline sessions recover the original turn after reception returns
    Given a Codex session without network reception
    When connection attempts fail for longer than seven seconds before reception returns
    Then the original full request succeeds without a synthetic user message or transport change

  Scenario: Cancellation drops setup reads and offline backoff
    Given a Codex request awaiting setup or body data or offline backoff
    When the caller drops the pending request stream
    Then cancellation clears reconnect state promptly and no later request occurs
