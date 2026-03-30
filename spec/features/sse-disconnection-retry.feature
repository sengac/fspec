@done
@NET-001
Feature: SSE Disconnection Retry — transient network error recovery in stream loop

  """
  Retry at stream_loop level (not SSE level), matching existing PROV-040 truncation and PROV-041 thinking exhaustion recovery patterns
  New module recovery_network.rs with MAX_NETWORK_RETRIES (3) and network_retry_delay() (exponential: 1s→2s→4s). Error classifier is_transient_network_error() in error_classifiers.rs detects 17+ patterns.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Transient network errors (connection reset, DNS timeout, broken pipe, SSL errors, unexpected EOF) must be detected and classified separately from fatal API errors
  #   2. Maximum 3 retry attempts with exponential backoff (1s, 2s, 4s = 7s total)
  #   3. Retry counter resets on successful data receipt (Text, ToolCall, Usage, FinalResponse)
  #   4. Any text generated before disconnection must be preserved in message history
  #   5. Retry must send a Continue re-prompt so the model picks up where it left off
  #   6. Retry must respect user interruption (Esc key) — check is_interrupted after each sleep
  #   7. Retry must also work in post-compaction streams and DeepSearch sub-agent streams
  #   8. After all retries exhausted, the original error must propagate as fatal
  #
  # EXAMPLES:
  #   1. Network blip during streaming — connection reset detected, waits 1s, retries with Continue prompt, model resumes generating
  #   2. Three consecutive DNS timeouts — retries at 1s, 2s, 4s intervals, all fail, session terminates with error message
  #   3. Network error during streaming, partial text already received — partial text preserved, retry succeeds, model continues from where it left off
  #   4. First retry fails but second succeeds — retry counter advances, backoff increases, session recovers on second attempt
  #   5. User presses Esc during retry backoff — retry loop aborts immediately instead of waiting
  #   6. Non-network API error (400 bad request, 401 auth) — not classified as transient, no retry attempted, error propagates immediately
  #   7. Network error during post-compaction retry stream — same retry logic applies in compaction_retry.rs
  #   8. Network error during DeepSearch sub-agent — sub-agent retries independently without crashing parent session
  #   9. Successful streaming after earlier retry — retry counter resets so future errors get full 3 attempts again
  #
  # ========================================

  Background: User Story
    As a user
    I want to have my session automatically recover from transient network disconnections during SSE streaming
    So that my conversation is not lost when a brief network blip occurs

  @network-recovery
  Scenario: Recover from a single network blip during streaming
    Given an active SSE streaming session with the LLM
    When a transient connection reset occurs mid-stream
    Then the error is classified as a transient network error
    And the system waits 1 second before retrying
    And a Continue re-prompt is sent to the LLM
    And the model resumes generating from where it left off

  @network-recovery @exhaustion
  Scenario: All retry attempts exhausted after consecutive failures
    Given an active SSE streaming session with the LLM
    When three consecutive DNS timeout errors occur
    Then the system retries at 1s, 2s, and 4s intervals
    And after all 3 retries are exhausted the original error propagates as fatal
    And the session terminates with an error message

  @network-recovery @partial-text
  Scenario: Partial text preserved on network error during streaming
    Given an active SSE streaming session that has already received partial text
    When a transient network error occurs
    Then the partial text generated before disconnection is preserved in message history
    And the retry succeeds with a Continue re-prompt
    And the model continues from where it left off

  @network-recovery @backoff
  Scenario: Retry succeeds on second attempt with increasing backoff
    Given an active SSE streaming session with the LLM
    When a transient network error occurs and the first retry also fails
    Then the first retry waits 1 second
    And the second retry waits 2 seconds
    And the second retry succeeds
    And the session recovers

  @network-recovery @interruption
  Scenario: User interruption during retry backoff aborts immediately
    Given the system is waiting during retry backoff after a network error
    When the user presses Esc
    Then the retry loop aborts immediately without waiting for the full delay

  @network-recovery @classifier
  Scenario: Non-network API errors are not retried
    Given an active SSE streaming session with the LLM
    When a non-transient error occurs such as 400 bad request or 401 unauthorized
    Then the error is not classified as a transient network error
    And no retry is attempted
    And the error propagates immediately

  @network-recovery @compaction
  Scenario: Network retry works in post-compaction retry streams
    Given a post-compaction retry stream is active
    When a transient network error occurs during the compaction retry stream
    Then the same retry logic applies with exponential backoff
    And the compaction retry stream recovers

  @network-recovery @deepsearch
  Scenario: Network retry works in DeepSearch sub-agent streams
    Given a DeepSearch sub-agent is streaming a response
    When a transient network error occurs in the sub-agent stream
    Then the sub-agent retries independently
    And the parent session is not crashed

  @network-recovery @counter-reset
  Scenario: Retry counter resets after successful data receipt
    Given a session that previously recovered from a network error
    When the stream successfully receives data events
    Then the retry counter resets to zero
    And future network errors get the full 3 retry attempts again

  @network-recovery @classifier
  Scenario: Transient network error patterns are correctly detected
    Given the error classifier for transient network errors
    When various error messages are evaluated for transient classification
    Then it detects connection reset, connection refused, and connection closed errors
    And it detects broken pipe, DNS error, and network unreachable errors
    And it detects timeout, hyper, unexpected EOF, and SSL errors
    And it detects SSE HTTP client errors with nested error wrapping
    And it does not classify rate limits, auth errors, or content policy violations as transient
