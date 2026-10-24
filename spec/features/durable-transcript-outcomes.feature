@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Archive terminal outcomes separately from model dialogue
  """
  Version 1 terminal records share the incremental transcript lane and bounded
  recovery admission mechanism. Public messages are selected from typed safe
  outcome codes, never provider bodies or arbitrary tool/error strings. Original
  timestamps and stable IDs survive retries and cold replay. Retention follows
  the owning session; export exposes the same scrubbed display-only records.
  Errors, user cancellation, loop interruption and failed compaction are durable.
  Token rate, waiting, spinners and progress are transient, never active on reopen.
  Terminal records are not MessageEnvelope dialogue and never affect provider
  history, original message coordinates, compaction or token accounting.
  """

  Scenario: Partial text and one original provider error survive reopen
    Given partial assistant text followed by a provider failure
    When terminal persistence is retried and the archive is reopened twice
    Then the text precedes exactly one error with its original timestamp

  Scenario: Cancellation and loop interruption remain distinct
    Given a genuine user cancellation and a genuine loop interruption
    When their terminal records are reopened
    Then the archive retains each original reason without inventing a tool cancellation

  Scenario: Failed compaction never becomes successful completion
    Given a failed compaction before DAG publication
    When its terminal record is reopened
    Then the transcript contains a compaction failure and no successful settlement

  Scenario: Terminal diagnostics cannot persist sensitive provider payloads
    Given a provider error containing credentials, a prompt and a tool payload
    When the terminal event is durably archived
    Then only the typed scrubbed public notice is stored
