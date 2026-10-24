@provider-abstraction
@PROV-148
@providers
@streaming
@integration
Feature: Codex terminal lifecycle
  Codex users need reliable completion and bounded continuation without changing other providers.

  """
  The Codex model owns its SSE source and drops it at the first terminal event.
  Missing end_turn retains the ordinary fallback: return text or hand tools to the generic loop.
  Explicit false permits three text-only continuations. Interrupted incomplete text may continue;
  content filtering, budget exhaustion and incomplete tools never retry.
  PROV-160 supersedes the original transport interruption policy: premature EOF retracts
  abandoned output and retries on the same transport, without changing ordinary Responses EOF behavior.
  Tools are buffered until successful completion. No shared Responses defaults change.
  Completed answer sections fill missing deltas without duplicating streamed text.
  """

  Scenario: Completion finishes before the socket closes
    Given a Codex stream whose socket stays open after completion
    When the caller consumes the stream
    Then one final response arrives without waiting for socket closure

  Scenario: Premature EOF retracts abandoned text before replacement
    Given a Codex stream ending after a text delta followed by a successful replacement
    When the caller consumes the stream within a bounded deadline
    Then only replacement text remains and the original request is replayed once

  Scenario: Backend failures never dispatch buffered tools
    Given a Codex stream with tool arguments followed by failure
    When failed and top-level error events arrive
    Then actionable codes and messages survive without tool dispatch or success

  Scenario: Incomplete outcomes retain usage and reject unsafe continuation
    Given incomplete Codex responses with filtering budget and interrupted tool outcomes
    When the caller consumes each stream
    Then each reason and usage survive without tool dispatch or success

  Scenario: Explicit false continues text with a bounded policy
    Given a completed text-only Codex response with end_turn false
    When subsequent responses complete or continue indefinitely
    Then text history is replayed and at most three follow-ups occur

  Scenario: Interrupted text resumes without replaying tool effects
    Given an interrupted incomplete text-only response
    When the response includes replayable assistant output
    Then a follow-up completes without dispatching tools

  Scenario: True absent and tool responses use the appropriate fallback
    Given completed Codex responses with true absent or tool-bearing false turn markers
    When the caller consumes each stream
    Then text ends and completed tools return once to the generic loop

  Scenario: Unknown events are ignored but malformed terminals fail
    Given optional unknown events and malformed known terminal events
    When the caller consumes each stream
    Then unknown events permit completion but malformed terminals never succeed

  Scenario: Duplicate completion does not duplicate usage or tools
    Given a Codex stream with repeated completed events
    When the caller consumes the stream
    Then usage tools and final response each appear once

  Scenario: Ordinary Responses retains its existing EOF behavior
    Given identical partial streams for Codex and ordinary Responses models
    When both callers consume independently scripted streams within bounded deadlines
    Then Codex recovers with replacement text while ordinary Responses rejects missing usage without retrying

  Scenario: Completed answers fill missing stream delivery
    Given completed answer and refusal sections with absent or partial deltas
    When item completion and successful terminal output repeat those sections
    Then each unseen answer suffix appears once without another request
