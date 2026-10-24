@done
@CMPCT-074
@compaction
@agent-core
@context-management
@phase-1
@bug
Feature: Codex latest context occupancy remains separate from spend
  """
  Each physical Responses terminal contributes measured spend exactly once.
  Aggregate loop finals preserve that spend but expose latest-request occupancy
  separately to the compaction hook. Inclusive cache/reasoning subsets count
  once. Missing latest usage fails rather than publishing an older request or aggregate.
  Generation changes invalidate old occupancy without deleting measured spend.
  """

  Background: User Story
    As an agent using Codex internal continuations
    I want admission to use the latest physical context rather than aggregate spend
    So that fitting requests are not blocked by unrelated earlier generation

  Scenario: Aggregate continuation spend cannot veto a fitting next request
    Given two physical Codex responses individually below the threshold
    And their combined measured usage exceeds that threshold
    When the Responses stream finishes and the compaction hook checks a small next request
    Then admission remains open and all measured spend is retained

  Scenario: Actual latest occupancy blocks the next request
    Given a Codex continuation whose latest measured context exceeds the threshold
    When the Responses stream finishes and the compaction hook checks a small next request
    Then admission is cancelled and all measured spend is retained

  Scenario: Missing latest usage does not reuse earlier measured context
    Given a measured Codex response followed by a terminal without usage
    When the Responses stream is consumed
    Then missing latest usage fails without publishing completion while earlier measured spend is retained
    And the committed checkpoint remains unchanged
