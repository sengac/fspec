@bug
@tools
@providers
@streaming
@BUG-104
Feature: DeepSearch Codex sub-agent still fails because Responses API requires stream=true
  """
  DeepSearch currently runs through rust/napi/src/deep_search_handler.rs using RigAgent::prompt(), while Codex Responses API forces request.stream = Some(true) only in rust/patches/rig-core/src/providers/openai/responses_api/streaming.rs.
  Low-risk implementation should preserve BUG-102 provider/model inheritance and provider-specific request shaping, while collecting any Codex-required streaming execution internally and still returning a final String result.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. DeepSearch must use a Codex-compatible execution path that satisfies the Responses API requirement for `stream=true` while still returning a final synthesized string to the caller
  #   2. The BUG-102 provider/model inheritance and provider-specific request configuration fixes must remain intact; this bug only addresses the missing execution-mode compatibility gap
  #   3. Existing non-Codex DeepSearch providers must keep their current successful behavior; the bug fix must not regress Claude, OpenAI, Gemini, or Z.AI.
  #
  # EXAMPLES:
  #   1. Codex-backed DeepSearch inherits provider/model correctly but still fails at runtime with HTTP 400 and "Stream must be set to true".
  #   2. From a Codex-backed session, invoking DeepSearch on a normal code scope returns a synthesized answer instead of failing with a 400 "Stream must be set to true" error.
  #   3. The DeepSearch caller still receives one final text answer rather than raw streaming events or partial chunks.
  #   4. A Claude, OpenAI, Gemini, or Z.AI session can still invoke DeepSearch successfully with the existing final-answer contract unchanged.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the fix route only Codex through a streaming collection path, or should DeepSearch adopt a provider-agnostic streaming execution abstraction for all providers if that remains low-risk?
  #   A: Use the lowest-risk fix first: keep non-Codex providers on the current execution path, and route Codex through an internal streaming collection path that consumes the stream to completion and returns the same final synthesized string contract. Generalize later only if the abstraction stays simple and safe.
  #
  # ASSUMPTIONS:
  #   1. Use the lowest-risk fix first: keep non-Codex providers on the current execution path, and route Codex through an internal streaming collection path that consumes the stream to completion and returns the same final synthesized string contract. Generalize later only if the abstraction stays simple and safe.
  #
  # ========================================
  Background: User Story
    As a developer using DeepSearch with Codex-backed sessions
    I want to invoke DeepSearch and receive a final synthesized answer without transport-mode errors
    So that I can rely on DeepSearch across providers, including Codex, without the sub-agent failing at runtime

  Scenario: Codex DeepSearch uses a streaming-compatible execution path
    Given a DeepSearch sub-agent is constructed for provider "codex"
    When the sub-agent executes the query
    Then the execution path consumes a streaming response to completion
    And the final synthesized answer is returned as one String result

  Scenario: Streaming collection returns only the final DeepSearch answer contract
    Given a DeepSearch streaming response contains intermediate assistant chunks
    When the streaming collection completes
    Then DeepSearch returns only the final synthesized answer text to the caller
    And raw streaming chunks are not returned from the DeepSearch tool call

  Scenario: Non-Codex providers keep the existing non-streaming execution path
    Given a DeepSearch sub-agent is constructed for provider "claude"
    When the sub-agent executes the query
    Then the execution path remains non-streaming
    And the final synthesized answer contract remains unchanged
