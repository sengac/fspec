@done
@CMPCT-067
@compaction
@context-management
@agent-core
@phase-1
@bug
Feature: Real multisegment requests preserve coherent backend and TUI fill
  """
  The shared context contract in coherent-request-context-fill.feature applies.
  Exercise selected physical requests through streaming, background publication
  and the TUI reducer. Current request occupancy is not cumulative billed spend.
  """

  Scenario: Multisegment spend cannot change current request fill
    Given a current request snapshot and cumulative output from earlier segments
    When live and final backend and TUI token updates arrive
    Then both surfaces retain the same threshold-relative current request fill
    And cumulative generated spend remains separately visible
