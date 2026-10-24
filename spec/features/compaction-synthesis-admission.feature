@done
@BUG-211
@phase-1
@agent-core
@context-management
@compaction
Feature: Compaction synthesis admission
  """
  Final synthesis preserves the selected provider's historical tool evidence
  requirements while preventing further tool execution. Its complete serialized
  request must fit the selected context and output reservation before dispatch.
  """

  Scenario: Synthesis respects provider requirements for historical tool evidence
    Given a provider requires tool declarations alongside historical tool evidence
    When an exhausted compactor makes its final synthesis request
    Then historical evidence remains valid and further tool use is disabled
    And the summary is returned without another exploration round

  Scenario: Oversized synthesis is rejected before provider dispatch
    Given gathered evidence exceeds the selected synthesis request budget
    When the compactor finishes its exploration budget
    Then synthesis fails admission without sending an oversized request
