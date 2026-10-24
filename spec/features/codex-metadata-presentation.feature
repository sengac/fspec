@done
@BUG-241
@providers
@streaming
@integration
@provider-abstraction
Feature: Codex metadata presentation
  """
  Pinned upstream bb64b894. Separate diagnostic snapshots from presentation. Reuse existing transient status channels where possible; no automatic model switching or transport retry changes. Preserve private payload redaction, token usage and output order. Supersedes PROV-153 boolean clear and moderation-notice requirements.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Only valid upstream buffering objects produce buffering observations; top-level presence, event type, required arrays, and retry-model omission versus null follow pinned Codex semantics.
  #   2. Routine headers, rate limits, and moderation remain diagnostic-only; actionable verification and existing recovery notices remain distinct and deduplicated.
  #   3. Buffering presentation is transient and owned by the current turn, dismisses on assistant output or termination, and cannot be resurrected by stale streams or replay.
  #
  # EXAMPLES:
  #   1. Routine metadata stays silent: Given a Codex session with changed headers rates and moderation metadata; When those updates arrive; Then diagnostics update without user notifications.
  #   2. Buffering follows upstream objects: Given boolean malformed conflicting and nested buffering values; When metadata is parsed; Then only valid objects with correct precedence activate buffering.
  #   3. Waiting belongs to the active turn: Given a buffering Codex turn; When output cancellation completion or a new turn arrives; Then waiting clears and old streams cannot restore it.
  #
  # ========================================
  Background: User Story
    As a Codex session user
    I want to see accurate temporary buffering status without routine metadata notices
    So that understand genuine waiting conditions without misleading transcript noise

  Scenario: Routine metadata remains diagnostic only
    Given a Codex session receiving headers rate limits and moderation metadata
    When the diagnostic metadata changes during a turn
    Then session diagnostics update without user notifications
    And non Codex sessions emit no Codex presentation

  Scenario: Buffering is temporary waiting state
    Given a Codex turn receiving valid buffering observations
    When repeated observations arrive before and after assistant output
    Then waiting is shown once and dismissed when output begins
    And buffering never becomes transcript notifications or changes the selected model

  Scenario: Old streams cannot revive waiting state
    Given an old Codex stream and a newer active turn
    When old buffering events and stream cleanup arrive
    Then the newer waiting state is unchanged
    And completion failure cancellation and reset clear current waiting

  Scenario: Verification remains actionable and turn scoped
    Given a Codex session receiving verification and recovery events
    When verification repeats alongside unrelated metadata across turns
    Then verification is announced once per turn without stale metadata labels
    And recovery notifications remain available
