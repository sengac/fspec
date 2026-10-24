@done
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Separate explicit conversation reset from committed continuation boundaries
  """
  Explicit conversation replacement removes continuation, not the generation
  watermark. Provider metadata records a reset alongside null continuation so a
  cold replay can distinguish an authentic reset from missing required state.
  Explicit non-Codex replay authority can invalidate a dormant extension only
  when no Codex metadata exists; typed replay remains authoritative regardless
  of the currently selected provider. Preflight must use the same authority.
  """

  Background: User Story
    As a session owner
    I want explicit history replacement to remove obsolete provider continuation
    So that reset history cannot resurrect stale reasoning or accept stale responses

  Scenario: Explicit reset removes continuation and cold restores its generation watermark
    Given a durable Codex continuation and an in-flight request from its generation
    When I explicitly reset the provider session twice
    Then live and durable continuation are absent with an advancing reset watermark
    And a cold restore rejects older checkpoints and committed boundaries and accepts new work at the reset generation
    And committed generation boundaries remain present checkpoints

  Scenario: Bare or malformed reset metadata cannot authorize missing continuation
    Given an existing Codex continuation
    When I restore null continuation without a valid advancing reset marker
    Then restore fails without replacing the authoritative checkpoint
    And inference remains blocked rather than becoming a fresh session

  Scenario: Explicit non-Codex authority invalidates only untyped dormant continuation
    Given a dormant Codex extension and ordinary non-Codex replay including a committed summary without Codex metadata
    When I validate and install that replay with explicit non-Codex authority
    Then the replay remains unchanged and dormant continuation is absent
    And typed Codex replay still restores under non-Codex selection
    And active Codex replay still rejects missing or null checkpoints under bare or qualified selection

  Scenario: An in-flight response cannot publish after explicit reset
    Given a nonstreaming Codex request awaiting its physical response
    When an explicit reset removes continuation before that response arrives
    Then the old response fails without publishing completion or restoring continuation
    And the reset generation remains authoritative

  Scenario: A newer committed boundary supersedes retained older reset records
    Given a durable reset record retained behind a newer committed Codex boundary
    When I validate and cold restore the complete retained replay
    Then the newer committed checkpoint remains present and authoritative
    And older reset records cannot remove the newer continuation
