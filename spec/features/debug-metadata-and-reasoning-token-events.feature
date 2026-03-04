@RIG-011
Feature: Debug Metadata and Reasoning Token Events

  """
  Key files: codelet/cli/src/interactive/stream_loop.rs (debug events + api.request model bug),
  codelet/cli/src/interactive/repl_loop.rs (metadata model bug),
  codelet/napi/src/session_manager.rs (NAPI metadata model bug)
  """

  Background: User Story
    As a developer
    I want debug metadata to show the correct model identity and include reasoning tokens in capture events
    So that I can diagnose model behavior, understand costs, and trust the debug capture data

  # ----- Layer 4: Debug metadata model identity -----

  Scenario: Debug metadata records correct model identity in repl_loop
    Given a session with provider "codex" and model_id "gpt-5.3-codex"
    When debug capture session metadata is set via repl_loop
    Then the SessionMetadata model field should be "gpt-5.3-codex"
    And the SessionMetadata provider field should be "codex"

  Scenario: Debug capture api.request event shows correct model
    Given a debug-enabled session with provider "codex" and model_id "gpt-5.3-codex"
    When an api.request event is captured in stream_loop
    Then the event data should have model "gpt-5.3-codex"
    And the event data should have provider "codex"

  # ----- Layer 5: Debug capture events include reasoning tokens -----

  Scenario: Debug capture includes reasoning tokens in aggregatedUsage
    Given a completed API response with a completion::Usage containing reasoning_tokens Some(5000)
    When the api.response.end event is captured in stream_loop
    Then the aggregatedUsage should include reasoningTokens 5000

  Scenario: Debug capture includes reasoning tokens in token.update
    Given a completed API response with reasoning_tokens visible in final_update
    When the token.update event is captured in stream_loop
    Then the event data should include reasoningTokens 5000

  # ----- Layer 6: NAPI session_manager debug metadata -----

  Scenario: NAPI session_update_debug_metadata uses model_id
    Given a NAPI session with provider "codex" and selected_model_id "gpt-5.3-codex"
    When session_update_debug_metadata is called
    Then the SessionMetadata model field should be "gpt-5.3-codex"
    And the SessionMetadata provider field should be "codex"

  Scenario: NAPI session_toggle_debug uses model_id
    Given a NAPI session with provider "codex" and selected_model_id "gpt-5.3-codex"
    When session_toggle_debug enables debug capture
    Then the SessionMetadata model field should be "gpt-5.3-codex"
    And the SessionMetadata provider field should be "codex"
