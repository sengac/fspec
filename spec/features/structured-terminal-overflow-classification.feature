@cli
@context-management
@compaction
@CMPCT-054
Feature: Structured terminal overflow classification
  """
  Runtime overflow recovery uses machine-readable rejection codes carried by
  the selected provider error chain. Human-readable text alone never authorizes
  compaction or substitutes another successful request.
  """

  Scenario: Structured HTTP overflow survives streaming wrappers
    Given a provider HTTP rejection with a context overflow code
    When streaming and agent context wrap the original rejection
    Then terminal recovery recognizes the overflow structurally

  Scenario: HTTP diagnostic wording does not authorize compaction
    Given a provider HTTP rejection without a context overflow code
    When its diagnostic text mentions context overflow
    Then terminal recovery does not classify it as overflow

  Scenario: Bare text cannot impersonate a provider rejection
    Given an untyped error containing a serialized overflow rejection
    When terminal recovery classifies the error
    Then terminal recovery does not classify it as overflow

  Scenario: Provider overflow code survives agent context
    Given a provider completion rejection with a context overflow code
    When agent context wraps the original rejection
    Then terminal recovery recognizes the overflow structurally
