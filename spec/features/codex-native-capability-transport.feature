@PROV-158
@providers
@provider-abstraction
@integration
Feature: Codex native capability transport
  """
  Native hosted search and compaction are explicitly opted in per Codex model.
  Default models advertise neither capability; enabled requests preserve typed
  output before newly supplied user input.
  """

  Scenario: Explicit native capabilities reach the production transport
    Given a Codex model explicitly enabling hosted search and native compaction
    When successful native output is followed by a new user turn
    Then the wire request advertises the enabled capability and replays its typed output
    And default models advertise neither capability
