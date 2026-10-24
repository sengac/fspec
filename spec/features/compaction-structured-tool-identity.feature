@cli
@context-management
@compaction
@CMPCT-059
Feature: Compaction structured tool identity
  """
  Durable conversation projection preserves tool invocation identity for selected
  provider serializers. Explicit call identifiers remain distinct from item IDs.
  Legacy tool-use identifiers are the invocation identifiers when no separate
  call identifier was stored. Immutable envelopes remain unchanged.
  """

  Scenario: Explicit tool call identifiers survive durable compaction projection
    Given a captured source with a structured suffix containing distinct item and call identifiers
    When the runtime admits and installs the durable generation
    Then selected preparation and installed messages retain both tool invocation identifiers
    And immutable suffix envelopes remain unchanged

  Scenario: Legacy tool use identifiers authorize matching structured results
    Given a captured source with a structured suffix containing legacy tool use identifiers
    When the runtime admits and installs the durable generation
    Then selected preparation and installed messages retain matching tool invocation identifiers
    And immutable suffix envelopes remain unchanged
