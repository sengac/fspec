@done
@CMPCT-067
@compaction
@context-management
@agent-core
@phase-1
@bug
Feature: Context fill uses one current request generation basis
  """
  A shared context snapshot identifies model, physical request and committed
  generation, inclusive occupancy, capacity, trigger threshold and provenance.
  Request occupancy is distinct from loop/session spend. Backend and TUI consume
  the snapshot, never recompute fill from cumulative display counters. Unknown
  or post-commit estimated context cannot certify measured fit. Older generation
  or reordered request events cannot overwrite a newer context snapshot.
  """

  Background: User Story
    As an operator observing multisegment inference
    I want live and final context fill to share a declared occupancy basis
    So that billing totals cannot masquerade as the current context

  Scenario: TUI reduction keeps cumulative spend separate from current request fill
    Given a current request snapshot and cumulative output from earlier segments
    When the TUI reduces a context update followed by cumulative token updates
    Then its threshold-relative current request fill remains unchanged
    And cumulative generated spend remains separately visible

  Scenario: Stale context events cannot overwrite a newer generation
    Given an estimated committed generation snapshot followed by a new measured request
    When an older generation or earlier request event arrives afterward
    Then the latest measured context remains unchanged on backend and TUI

  Scenario: Unknown occupancy and capacity cannot claim measured fit
    Given context usage or capacity is unavailable
    When backend and TUI consume the context snapshot
    Then provenance remains unknown and no guessed fit percentage authorizes inference
