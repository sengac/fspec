@provider-abstraction
@PROV-155
@providers
@streaming
@integration
Feature: Codex cache affinity routing
  Codex sessions retain private cache affinity without leaking routing between accounts or turns.

  """
  Audited upstream bb64b89 core/src/client.rs uses a stable prompt_cache_key and session_id
  header, with first received x-codex-turn-state retained only within a user turn.
  Our keys are random opaque UUIDs, never derived from prompts, tokens or account identifiers.
  A session owns its handle; children and restored sessions start fresh. New user turns clear
  turn state, not affinity. Reset, account change and stale recovery rotate affinity and clear
  turn state. Missing handles use ordinary SSE. No cache hit is promised. Usage is untouched.
  HTTP requests bind routing after credential refresh. Late responses cannot repopulate reset
  state. Tool follow-ups and retries within a user turn retain the first valid turn state.
  """

  Scenario: Requests retain opaque affinity only within their session
    Given two independent Codex sessions including a subordinate session
    When each session makes repeated streamed requests
    Then each session sends a stable distinct opaque cache key and session header

  Scenario: Turn state survives follow-ups but not a new user turn
    Given a Codex session receiving server turn state
    When follow-ups and a new user turn send requests
    Then only follow-ups return the first valid turn state

  Scenario: Reset and account changes invalidate routing
    Given a session with cached affinity and server turn state
    When reset stale recovery or account change occurs
    Then requests use fresh affinity without old turn state and late responses are ignored

  Scenario: Missing affinity support preserves ordinary streaming and usage
    Given Codex without routing and a non-Codex Responses client
    When both stream responses with cached token usage
    Then neither receives routing additions and cached usage remains accurate
