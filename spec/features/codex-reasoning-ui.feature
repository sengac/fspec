@provider-abstraction
@PROV-150
@cli
@integration
@streaming
@phase-1
Feature: Codex reasoning UI delivery
  """
  The production CLI stream loop receives Codex thinking deltas. Completed-only
  presentation is selectable without altering readable history or answer events.
  """

  Scenario: CLI reasoning presentation respects completed delivery selection
    Given a live Codex agent with incremental and completed-only reasoning
    When the CLI stream loop runs with completed delivery enabled or disabled
    Then thinking events honor the selection and ordinary answer events remain visible
    And encrypted continuation content never reaches the UI
