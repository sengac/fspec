@testing
@providers
@persistence
@integration
@provider-abstraction
@PROV-149
Feature: Codex session continuation
  """
  Session-owned optional versioned codexContinuation metadata survives manifest
  envelopes and blob rehydration. Reload restores the last checkpoint; legacy
  records retain ordinary history. Clear, model switch and local compaction
  invalidate stale opaque state without altering non-Codex provider behavior.
  """

  Scenario: A backend checkpoint survives session reload and a new user turn
    Given a completed Codex turn persisted by the backend
    When its session envelopes are loaded into a fresh live session
    And a new user turn is sampled using a newly created provider
    Then the request replays the saved typed items before the new user input

  Scenario: Legacy and non-Codex sessions keep ordinary history behavior
    Given legacy envelopes without Codex state and a non-Codex session
    When their histories are saved restored and sampled
    Then no Codex state is added and ordinary history is unchanged

  Scenario: Conversation replacement invalidates typed continuation
    Given a live Codex session with saved opaque reasoning
    When its history is cleared compacted or switched to another model
    Then subsequent requests and reloads do not replay stale typed items
