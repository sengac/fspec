@provider-abstraction
@PROV-154
@PROV-160
@providers
@streaming
@integration
Feature: Codex stream recovery
  Codex users need bounded safe recovery and cancellation without replaying tool effects.

  """
  PROV-160 supersedes the prior no-replacement policy. Typed Codex-owned recovery may replace accepted interrupted requests on the same transport with full-current-DAG input; completed tool results are retained, uncommitted tool calls never execute, and no synthetic Continue or generic CLI recovery is permitted. OAuth refresh and explicit rejection retries remain bounded. Dropping the stream cancels all owned setup, read and backoff futures.
  """

  Scenario: Cancellation aborts network waits and retry backoff
    Given a Codex request waiting on network data or retry advice
    When the caller cancels the stream
    Then no further requests occur and cancellation finishes promptly

  Scenario: Unauthorized setup refreshes credentials once
    Given valid-looking OAuth credentials rejected by the server
    When refreshed credentials succeed or are rejected again
    Then one refresh occurs and at most two setup attempts are made

  Scenario: Safe retries honor advice and stop within budget
    Given setup rejected with rate limiting, unavailable service, gateway failure or a permanent error
    When Codex attempts recovery
    Then retryable rejections use bounded delays and permanent errors never retry

  Scenario: Accepted streams never replay tool effects
    Given a Codex continuation containing a completed tool result
    When its accepted response disconnects
    Then same-transport replacement retains the completed result without executing that tool again
