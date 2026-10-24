@done
@CMPCT-065
@providers
@context-management
@compaction
@phase-1
Feature: Custom runtime usage retains the declared additive contract
  """
  Streaming live and final usage and nonstreaming response usage share the same
  declared disjoint-category normalization. Preserve all measured detail rather
  than replacing unavailable usage with an authoritative estimate or zero.
  """

  Scenario: Custom live and final usage share the declared contract
    Given a scripted custom provider reports disjoint cache and reasoning categories
    When its real streaming and nonstreaming adapters deliver usage
    Then live final and nonstreaming occupancy agree without dropping detail
