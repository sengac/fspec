@done
@CONT-011
@cli
@completion
@integration
Feature: Completion registration layout guards
  """
  Preserve the existing armed registry, seven provider builders and DeepSearch
  exclusion checks. Follow Codex mod.rs delegation into agent.rs rather than
  mistaking a moved implementation for missing registration. These guards are
  supplemental; real provider-stream acceptance remains in the child features.
  """

  Scenario: Armed Done registration stays guarded across provider builders
    Given a provider agent is built for a session marked armed
    Then the agent tool set includes the done tool
    And an agent built for an unarmed session does not include the done tool
    And all 7 provider builder chains register the done tool conditionally on the armed state
    And the DeepSearch sub-agent toolset never includes the done tool
