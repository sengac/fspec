@agent-core
@phase-1
@context-management
@compaction
@CMPCT-054
Feature: Aborted turn compaction dependency cleanup
  """
  A real background turn owns builder, generation and candidate submission
  registrations. Aborting its task after a physical provider request begins
  must release every compaction dependency without replacing committed history
  or publishing completion. The durable baseline includes legitimate incoming
  input persistence before task cancellation.
  """

  Scenario: Aborting a real turn releases every compaction dependency
    Given the real outer loop waiting for a provider with registered compaction dependencies
    When the owning agent task is aborted after the provider request starts
    Then the builder and candidate submission registrations are removed
    And committed history remains unchanged without completion
