@done
@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Custom provider physical serializer parity
  """
  Admission uses the selected Rhai streaming request builder, including its
  mandatory context, tool schemas and thinking configuration. Preparation is
  synchronous and performs no HTTP. The physical streaming adapter invokes the
  same serializer on the blocking pool, without nesting a Tokio runtime.
  """

  Scenario: Custom preparation matches the selected streaming request
    Given a custom provider script with different ordinary and streaming request bodies
    When mandatory context including retrieval documents tools and thinking configuration are prepared then streamed
    Then prepared bytes equal the physical streaming body without an extra request
    And the source request remains unchanged

  Scenario: Custom preparation rejects a script builder failure
    Given a selected custom provider whose streaming request builder fails
    When admission prepares its next request
    Then preparation returns an explicit error without physical inference
