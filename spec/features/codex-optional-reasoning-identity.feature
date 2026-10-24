@provider-abstraction
@PROV-150
@providers
@streaming
@phase-1
Feature: Codex optional reasoning identity
  """
  Missing optional item IDs use output position until completed items establish
  stable IDs. Aliases preserve section identity without colliding with real IDs.
  """

  Scenario: Optional reasoning identities retain compatibility
    Given reasoning events without optional item IDs at distinct output positions
    When completed items identify those positions and repeat their readable text
    Then each section keeps a stable identity and only unseen text appears
