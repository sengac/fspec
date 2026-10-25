@done
@agent-core
@session-management
@persistence
@BUG-258
Feature: Journal terminal outcomes emitted by the outer agent error fallback
  """
  Human approval on 2026-10-25 supersedes the prior production hold, conditional on native ACDD card scope, status and Markdown attachments before implementation. The attached bounded phase extraction preserves ownership, locks, cancellation, registrations and recovery counters; its baseline must retain three reds and two greens before repair. Regressions exercise the actual background agent loop with offline loopback-only fixtures, invalid request limits, failed/exhausted outer recovery, applied-compaction budget exhaustion, inner errors and successful recovery. Cold replay uses safe typed metadata on the original request identity without model pollution. Final Error/Done must use the same request-owned BackgroundOutput; no global interception, replacement identity, immutable terminal rewriting or early recoverable failure admission.
  Owned overflow recovery borrows that same output instead of allocating another journal. An already acknowledged CompactionFailed terminal remains immutable; one actionable outer Error diagnostic may publish without another terminal. ProviderError duplicates remain suppressed. Cold replay retains ProviderError as a safe error and CompactionFailed as its existing safe warning notification.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Only an abandoned final outer fallback receives a safe durable terminal failure on the original request owner before publication; inner-stream terminal errors deduplicate and successful automatic recovery gains no spurious failure. Terminal prose never enters model originals.
  #
  # EXAMPLES:
  #   1. Invalid request preparation and failed/exhausted outer overflow recovery survive repeated cold replay as one safe ordered terminal outcome. An already-journaled inner error remains single; successful recovery has no final failure.
  #
  # QUESTIONS (ANSWERED):
  #   Q: The affected agent_loop.rs is 1536 lines, almost entirely one async function, and BackgroundOutput is 739 lines. Unlike the BUG-257 mechanical item split, satisfying the mandatory 300-line rule requires phase extraction with explicit async state/control-flow boundaries. Approve that broader focused refactor after review, or keep BUG-258 specifying pending review? No production changes made for BUG-258.
  #   A: Earlier approval was withheld pending a concrete plan and red evidence. Superseded 2026-10-25: human approves the attached bounded extraction and minimal fix conditional on updating the existing native card/status and attaching new Markdown documentation before starting; no broader redesign is authorized.
  #
  # ========================================
  Background: User Story
    As a session user
    I want to recover a final outer agent failure exactly once after restart
    So that terminal outcomes remain durable and safe without contaminating model history or successful recovery

  Scenario: Invalid request preparation retains one cold terminal failure
    Given a real background turn with invalid selected request limits
    When request preparation fails before streaming begins
    Then one safe terminal failure survives repeated cold replay without model pollution

  Scenario: Failed outer overflow recovery retains the original request outcome
    Given a real background request whose overflow recovery builder fails
    When the outer owner abandons recovery and publishes completion
    Then one safe failure is durable on the original request and cold replay retains it once

  Scenario: Exhausted outer recovery retains the final abandoned outcome
    Given a real background request repeatedly overflows after successful compaction
    When its outer recovery budget is exhausted
    Then the final abandoned request has one cold terminal failure and no further inference starts

  Scenario: Inner journaled errors are not duplicated by the fallback
    Given a real background stream that journals its own provider error
    When the outer error fallback also terminates that request
    Then durable and cold replay retain exactly one terminal failure

  Scenario: Successful automatic recovery adds no spurious failure
    Given a real background request with recoverable overflow
    When automatic recovery succeeds and the original task finishes
    Then durable and cold replay contain no provider failure and completion remains truthful

  Scenario: Applied compaction budget exhaustion retains failure instead of premature completion
    Given a real background task that repeatedly succeeds while applying compaction
    When the automatic continuation budget is exhausted after applied compaction
    Then the final request retains one cold terminal failure instead of success and no further inference starts

  Scenario: Failed explicit builder retry preserves actionable failure without another terminal
    Given a real background request whose automatic compaction builder failed
    When fresh user input retries the still failing builder
    Then its actionable live error and one cold failure per request survive without another terminal or inference
