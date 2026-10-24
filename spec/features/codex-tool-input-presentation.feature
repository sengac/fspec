@PROV-151
@providers
@provider-abstraction
@streaming
@integration
Feature: Codex tool input presentation
  """
  The Codex-only presentation hook forwards fragments without bypassing existing
  permission hooks or executing incomplete arguments.
  """

  Scenario: Input fragments reach presentation before execution
    Given interleaved Codex names and fragmented arguments on an open stream
    When a presentation consumer observes the agent
    Then each input update retains its item identity and no partial call executes
