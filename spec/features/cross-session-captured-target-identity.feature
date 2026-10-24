@agent-core
@phase-1
@context-management
@compaction
@CMPCT-056
Feature: Cross-session compaction rejects a changed captured target identity
  """
  The cross-session-compaction-ownership contract applies. Capture binds the
  selected model and durable generation; identity changes cannot promote stale
  candidates, replace committed context or publish successful completion.
  """

  Scenario: Changed target identity rejects the captured candidate
    Given an Idle target captured by a DAG builder waiting at a barrier
    When the selected target model changes before the builder returns
    Then the captured candidate is rejected without changing durable context
    And the unchanged target returns to Idle without completion
