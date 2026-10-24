@agent-core
@context-management
@compaction
@CMPCT-059
Feature: Mandatory compaction reminder preservation
  """
  Preserves the acceptance scenarios from durable-versioned-dag-boundary.
  One real integration target owns this specification partition.
  """

  Scenario: Quoted markers cannot supersede authoritative mandatory reminders
    Given an authoritative mandatory reminder followed by ordinary text quoting reminder markers
    When the runtime admits and installs a new generation
    Then the exact authoritative reminder remains installed without treating the quote as authority

  Scenario: Mixed content mandatory reminders survive admission and installation intact
    Given a mandatory reminder after another content block containing a quoted DAG marker
    When the runtime admits and installs a new generation
    Then every block of the mandatory reminder message is preserved exactly

  Scenario: Persisted mandatory reminders survive an empty runtime reminder view
    Given a captured durable source containing a mandatory reminder absent from runtime memory
    When the selected runtime admits and installs a new generation
    Then the exact persisted mandatory reminder remains installed and admitted
