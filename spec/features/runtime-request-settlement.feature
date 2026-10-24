@done
@CMPCT-069
@providers
@context-management
@compaction
@phase-1
Feature: Runtime finish hooks settle actual measured provider requests
  """
  Ordinary provider finish hooks persist measured physical request usage before
  successful completion. JSON display counters do not authorize settlement.
  Preserve request identity and cache detail through cold manifest reload.
  """

  Scenario: Runtime finish hooks settle every physical request
    Given a scripted provider stream with two measured physical requests
    When real request finish hooks complete through the selected adapter
    Then both requests are durably settled once without reading display counters
    And cache and reasoning evidence survives a cold store reload

  Scenario: Ordinary provider finish hooks settle measured cache usage
    Given a scripted Anthropic stream with measured fresh input cache and output usage
    When its real adapter completes through the runtime finish hook
    Then a cold manifest reload retains exactly one settlement with complete cache evidence
    And current occupancy agrees with the measured physical request
