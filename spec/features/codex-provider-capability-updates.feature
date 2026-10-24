@PROV-152
@providers
@provider-abstraction
@integration
Feature: Codex provider capability updates
  """
  Provider-owned capability snapshots are shared with already-created agents.
  Clearing the snapshot restores conservative Codex-only validation.
  """

  Scenario: Provider capability updates reach existing agents
    Given a Codex provider has an agent with explicit experimental options
    When the trusted capability source updates the provider snapshot
    Then the existing agent uses the updated snapshot on its next request
    And clearing the snapshot restores conservative validation
