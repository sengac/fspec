@done
@phase-1
@persistence
@session-management
@bug-fix
@BUG-248
Feature: Message JSONL tail recovery
  """
  Records are physical JSONL lines containing complete hash-valid StoredMessage
  values. Indexes are derived and cover only validated boundaries. Under the
  stable sidecar lock, a valid unterminated record is delimited and synchronized.
  Invalid unframed tail bytes are quarantined durably before truncation only when
  committed manifests prove no referenced original is lost. Missing or malformed
  evidence and malformed delimited records fail closed. No arbitrary rescanning
  for UUID-looking strings is permitted. Legacy String APIs bridge typed framing
  failures. Reopen consistency tests are not power-loss experiments.
  """

  Scenario: Torn tails remain recoverable after append and full rebuild
    Given valid history followed by an uncommitted partial JSON tail
    When the store reopens and appends a complete Unicode record
    Then a full index rebuild preserves exact valid IDs and content
    And the torn bytes remain quarantined across repeated recovery

  Scenario: Valid unterminated records are retained and delimited
    Given valid history ending with complete JSON without a newline
    When another record is appended and the index is rebuilt
    Then both original and new records remain separately readable

  Scenario: Referenced corruption refuses destructive recovery
    Given a manifest referencing a record truncated inside its UUID or Unicode content
    When the store reopens or attempts another append
    Then corruption is reported without changing the message history

  Scenario: Malformed interior records never invent message identities
    Given valid records separated by a malformed complete physical line
    When the index is rebuilt
    Then corruption is reported without indexing UUID text inside malformed content

  Scenario: Every Unicode byte truncation has controlled recovery
    Given serialized messages containing arbitrary Unicode content
    When each serialized byte boundary is truncated before a subsequent append
    Then recovery and full rebuild retain the valid prefix and exact appended content

  Scenario: Interrupted quarantine recovery is retryable
    Given a valid prefix and an uncommitted torn tail
    When each quarantine and repair synchronization boundary fails
    Then retry preserves the prefix and durably quarantines the torn bytes before removal

  Scenario: Concurrent recovery and append preserve exact record identities
    Given independent writers and an uncommitted torn tail
    When the writers reopen and append concurrently
    Then repeated full rebuilds retrieve each acknowledged record exactly once
