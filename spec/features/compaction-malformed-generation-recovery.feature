@cli
@done
@BUG-232
@phase-1
@agent-core
@compaction
@validation
Feature: Compaction malformed generation recovery
  """
  Fresh and incremental builders use the same complete-candidate syntax contract.
  Correction is bounded to three whole-candidate attempts through the existing
  validator and production summary handoff. No parser salvage, clamping or invented
  evidence is permitted. Diagnostics correlate operation, target and attempt without
  logging full candidates. Incident bodies and executable revision remain unconfirmed.
  """

  Scenario: Fresh and incremental generation explain complete node syntax
    Given a live session with persisted originals
    When fresh and incremental compaction requests reach the provider
    Then both requests require complete closed nodes with exact attributes and nonempty bodies

  Scenario: Malformed generation receives actionable whole candidate correction
    Given generated prose unclosed nodes or malformed trailing structure
    When the production builder receives a valid replacement within its correction budget
    Then correction explains complete node syntax and identifies the operation and attempt
    And only the complete validated replacement becomes committed memory

  Scenario: Exhausted malformed generation keeps inference blocked
    Given every generated candidate has malformed node structure
    When the production summary handoff exhausts three candidate attempts
    Then failure identifies the target operation attempt and syntax remedy
    And durable originals and committed memory remain unchanged with ordinary inference blocked
