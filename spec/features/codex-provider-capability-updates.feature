@BUG-472
@PROV-152
@providers
@provider-abstraction
@integration
Feature: Codex provider capability updates
  """
  Provider-owned capability snapshots are shared with already-created agents.
  Without a snapshot no Codex capability is invented: explicit options are
  sent unchanged (codex-reasoning-for-every-codex-model.feature, BUG-441).
  """

  Scenario: Provider capability updates reach existing agents
    Given a Codex provider whose capability snapshot does not advertise xhigh has an agent with explicit experimental options
    When the trusted capability source updates the provider snapshot
    Then the existing agent uses the updated snapshot on its next request
    And clearing the snapshot leaves explicit options unvalidated, with no capability invented
