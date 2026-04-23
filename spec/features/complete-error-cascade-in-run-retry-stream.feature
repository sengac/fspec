@error-handling
@cli
@done
@CMPCT-027
Feature: Complete error cascade in run_retry_stream
  """
  The primary stream loop handles ALL compaction-recovery restarts in-loop via begin_compaction_recovery + execute_compaction_and_capture_events + stream reassignment + continue. Post-compaction errors therefore pass through the same error cascade (prompt-too-long, truncation, image, network, stall, PromptCancelled). A compaction_retry_count bounds cascaded compactions at MAX_COMPACTION_RETRIES = 3.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All post-compaction stream errors must route through the primary loop's error cascade
  #   2. A circuit breaker must bound compaction retries at MAX_COMPACTION_RETRIES (3)
  #   3. Post-compaction PromptCancelled must recover again by running compaction in-loop
  #   4. Post-compaction 'prompt is too long' must trigger a second compaction round
  #   5. Post-compaction truncation, image, and network errors must be handled by the shared cascade
  #
  # EXAMPLES:
  #   1. After compaction, the retry stream yields 'prompt is too long' — a second compaction round is executed, up to 3 total attempts
  #   2. After compaction, the retry stream yields PromptCancelled — compaction runs again via the same in-loop restart
  #   3. After 3 consecutive compaction-triggering errors, the session returns a clear budget-exhausted error
  #   4. Post-compaction stream yields a truncated tool call error — the primary truncation-recovery handler (PROV-040) runs
  #   5. Post-compaction stream yields a transient network error — NET-001 retry logic still applies
  #
  # ========================================
  Background: User Story
    As a developer
    I want to run the post-compaction stream through the primary error cascade
    So that recoverable errors after compaction don't kill the session

  Scenario: MAX_COMPACTION_RETRIES constant is exposed as 3
    Given the stream loop provides a bounded retry budget for cascaded compaction attempts
    When a test reads the MAX_COMPACTION_RETRIES constant from the interactive module
    Then its value is exactly 3

  Scenario: Budget exhausted message mentions the attempt count
    Given a caller exhausted the compaction retry budget
    When build_compaction_budget_exhausted_message is invoked with the attempt count
    Then the returned message contains the numeric attempt count and the word compaction

  Scenario: execute_compaction_and_capture_events runs compaction and resets token tracker
    Given a session with a populated conversation and a previously triggered compaction token state
    When execute_compaction_and_capture_events is invoked with the original user prompt
    Then execute_compaction has cleared the prior turns and injected the compaction instruction
    Then the session token tracker has been reset via reset_after_compaction

  Scenario: execute_compaction_and_capture_events emits compaction continuing event on success
    Given a session that will successfully run compaction
    When execute_compaction_and_capture_events is invoked with a recording StreamOutput
    Then exactly one CompactionContinuing event is emitted
    Then no CompactionStarted or CompactionProgress events are emitted from the helper

  Scenario: The obsolete run_retry_stream function has been removed from the compaction retry module
    Given the in-loop restart replaces the separate post-loop retry stream
    When the codebase is searched for a fn run_retry_stream definition
    Then no such definition exists in the cli crate sources

  Scenario: Stream loop wires in a compaction retry counter bounded by MAX_COMPACTION_RETRIES
    Given the stream loop must bound cascaded compaction attempts
    When the stream_loop.rs source is inspected
    Then it declares a mutable compaction_retry_count counter and checks it against MAX_COMPACTION_RETRIES before each cascaded compaction
