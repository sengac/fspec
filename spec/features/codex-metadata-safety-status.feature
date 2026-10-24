@done
@BUG-241
@provider-abstraction
@PROV-153
@providers
@streaming
@integration
Feature: Codex metadata and safety status
  Codex users see useful server status without exposing private routing or moderation payloads.

  """
  Pinned audit: openai/codex bb64b89 codex-api/src/sse/responses.rs.
  Capture HTTP headers before converting the response to SSE. Event response.headers
  override top-level headers, case insensitively. Reasoning accounting is presence-based.
  Keep opaque turn state private for future continuation policy. Faster-model advice
  never changes the selected model. BUG-241 supersedes boolean clearing and visible
  moderation receipt: only valid objects activate buffering, invalid values are
  ignored, and routine metadata remains diagnostic-only. Typed waiting state is
  separate from actionable verification notifications and generic token usage.
  """

  Scenario: HTTP metadata reaches session diagnostics before text
    Given a Codex response with mixed-case diagnostic rate-limit and opaque headers
    When the response streams text and completes
    Then sanitized request and serving-model status reaches the observer before text
    And accounting etag rate limits and private turn state remain available

  Scenario: Event headers override top-level metadata without duplicating output
    Given conflicting top-level and response headers and a rate-limit update
    When the Codex stream consumes metadata and completion events
    Then response headers win and usage and text are delivered exactly once

  Scenario: Safety buffering updates without delaying text
    Given buffering notifications and advisory faster-model headers
    When output arrives between buffering updates and an explicit false
    Then output dismisses buffering and the false is ignored
    And the selected model never changes

  Scenario: Verification and moderation expose only sanitized diagnostics
    Given known and unknown verification names and private moderation payloads
    When metadata reaches the Codex status observer
    Then only recognized verification and moderation presence are retained in diagnostics
    And credentials routing payloads and opaque reasoning never appear in diagnostics

  Scenario: Missing unknown and malformed optional metadata remain compatible
    Given Codex and ordinary Responses streams with absent or unknown optional metadata
    When each stream completes
    Then normal text and existing usage remain unchanged
    And only the Codex stream can update Codex status

  Scenario: Ending or cancelling a stream clears transient buffering
    Given a Codex stream with active safety buffering
    When the stream completes fails or is dropped
    Then buffering clears while safe diagnostic metadata remains available

  Scenario: Arbitrary optional payloads cannot leak through status rendering
    Given arbitrary private optional metadata strings
    When Codex metadata is parsed and rendered
    Then private strings are absent from status and debug output
