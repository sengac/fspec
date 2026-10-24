@agent-core
@context-management
@compaction
@CMPCT-059
Feature: Runtime durable DAG installation
  """
  Preserves the acceptance scenarios from durable-versioned-dag-boundary.
  One real integration target owns this specification partition.
  """

  Scenario: Runtime installation reconciles only the durable promoted generation
    Given a captured durable source and a real selected request serializer
    And additional structured tool work arrives after capture
    When the runtime commits and publishes the candidate
    Then its installed DAG suffix provider state and token count match the durable generation
    And ordinary inference opens only after exact identity reconciliation

  Scenario: Runtime publication failure cannot authorize ordinary inference
    Given a captured durable source and a real selected request serializer
    When runtime publication fails after durable promotion
    Then the new identity remains authoritative and ordinary inference stays blocked
    And an incorrect reconciliation identity cannot clear that block

  Scenario: Runtime rejects a candidate that drops authoritative file identity
    Given a captured durable source with an authoritative file modification
    When a candidate explicitly supplies an empty file ledger
    Then the runtime rejects it before promotion without completion

  Scenario: Runtime assembles missing file identity and retains mandatory reminders
    Given a captured durable source with an authoritative file modification and mandatory reminder
    When a candidate omits its file ledger
    Then the admitted generation contains the authoritative file identity
    And the exact mandatory reminder remains installed
