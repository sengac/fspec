@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Gate the real executor on durable transcript admission
  """
  The actual streaming consumer and output adapters must propagate failed
  admission before polling the tool executor. Recovery never runs the tool.
  """

  Scenario: The real executor cannot bypass failed transcript admission
    Given a provider requesting a sentinel-writing tool and transcript storage failing at dispatch
    When the actual streaming executor consumes that provider request
    Then the executor returns a recovery error without creating the sentinel
