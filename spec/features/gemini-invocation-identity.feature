@done
@p1
@agent-core
@providers
@tool-execution
@persistence
@gemini
@BUG-253
Feature: Allow repeated Gemini tool invocations without duplicate journal admission failures
  """
  Allocate opaque per-invocation IDs at the Gemini adapter boundary. Keep function names and thought signatures in their existing protocol fields; request serialization resolves result names through the paired original call. The same identity flows through executor dispatch, durable background journal, results and cancellation. Dispatch remains fail-closed and cold replay is display-only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each Gemini function invocation has a distinct internal identity across response attempts; provider function names and thought signatures remain unchanged, duplicate dispatch is refused, and cold replay never reexecutes tools.
  #
  # EXAMPLES:
  #   1. Given a Gemini provider that requests the same reading tool twice with different paths and signed calls, When a real background streaming turn consumes both requests and a subsequent response attempt repeats that tool, Then each invocation executes once with a distinct durable identity and authentic result while physical continuations preserve function names and signatures
  #   2. Given an acknowledged Gemini tool invocation, When its internal identity is dispatched twice and the process restores before capturing the genuine result, Then duplicate dispatch is refused and repeated cold replay retains outcome-unknown without repeating the side effect or merging another invocation result
  #
  # ========================================
  Background: User Story
    As a agent user
    I want to invoke the same Gemini tool repeatedly
    So that each invocation executes once and recovers its own authentic result

  Scenario: Repeated Gemini calls retain separate authentic results
    Given a Gemini provider requesting signed reads of different files
    When a background turn consumes repeated reads and a later response attempt reads again
    Then each invocation executes once with a distinct durable identity and authentic result
    And physical continuations preserve function names and thought signatures

  Scenario: Duplicate dispatch and cold recovery never repeat a side effect
    Given an acknowledged Gemini invocation whose result has not been captured
    When its internal identity is dispatched twice and the journal is reopened repeatedly
    Then duplicate dispatch is refused and replay retains outcome-unknown without reexecution
