@cli
@context-management
@compaction
@regression
Feature: Stream recovery invokes the registered compaction builder
  """
  Stream recovery uses the session's registered durable DAG builder when one
  exists, just like pre-prompt compaction. Preparing an in-view instruction is
  not a completed compaction: the unchanged oversized history can immediately
  retrigger the hook before any provider request. Standalone sessions without
  a registered builder retain their in-view DAG protocol.
  """

  Scenario: Hook-triggered recovery commits a builder DAG before resuming
    Given conversation history exceeds the compaction threshold during streaming
    And the session has a registered compaction builder
    When the compaction hook cancels inference
    Then recovery invokes the builder once and durably commits its DAG
    And inference resumes with reduced history without exhausting compaction retries

  Scenario: Recovery admits the resume prompt before committing a replacement
    Given a builder DAG and a resume prompt each fit within the usable context budget
    But their combined request exceeds the provider capacity
    When hook-triggered recovery attempts to commit the replacement
    Then recovery rejects the replacement before promotion
    And no provider request or compaction completion occurs and inference remains blocked

  Scenario: Builder failure stops hook-triggered recovery without retrying oversized history
    Given conversation history exceeds the compaction threshold during streaming
    And the registered compaction builder fails
    When the compaction hook cancels inference
    Then recovery returns the builder failure after one invocation
    And no provider request or compaction completion occurs and inference remains blocked

  Scenario: Provider overflow invokes the registered compaction builder
    Given a streaming provider rejects the conversation as exceeding its context limit
    And the session has a registered compaction builder
    When stream recovery handles the provider rejection
    Then recovery invokes the builder once and durably commits its DAG
    And inference resumes with reduced history without exhausting compaction retries
