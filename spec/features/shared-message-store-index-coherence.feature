@phase-1
@persistence
@session-management
@bug-fix
Feature: Shared message store index coherence
  """
  Independent MessageStore instances share an append-only message history and
  a binary index. Index refresh, message mutation, and index publication must
  form one locked transaction. Refreshed offsets and metadata must come from
  the current history, including after another store compacts that history.
  Live reads share the message sidecar lock and invalidate cached index and
  message snapshots when the files change. Cleanup discovers references from
  committed manifests on disk, not a process-local session cache. It uses only
  nonblocking session lease probes and conservatively defers deletion while a
  writer could still be committing a message reference.
  """

  Scenario: Independent stores retain every committed append after reopening
    Given two independent stores opened before either writes a message
    When the stores take turns appending distinct messages
    Then a cold reopened store reads every committed message with its original content
    And reopening uses the saved index without rebuilding it

  Scenario: Live readers observe another store's committed append
    Given a reader opened before another store appends a message
    When the reader fetches that message without writing or reopening
    Then it reads the committed message with its original identity and content

  Scenario: Live readers invalidate cached metadata after a peer update
    Given a reader has cached a message with metadata
    When another store replaces that metadata with an equally sized value
    Then the reader sees the updated metadata without writing or reopening

  Scenario: Live readers use current offsets after another store compacts history
    Given a reader has cached retained and orphaned messages and an uncached retained message
    When another store removes the orphaned messages
    Then the reader no longer returns the orphaned message
    And it reads both retained messages with their original identities and content

  Scenario: Cleanup discovers committed references missing from the session cache
    Given the process session cache was loaded before a peer committed a new session
    And message history contains an unreferenced message
    When cleanup discovers orphaned messages
    Then it removes only the unreferenced message
    And the peer session's original messages remain available after cold reopening

  Scenario: Cleanup preserves messages awaiting a writer's manifest commit
    Given a writer has appended a message but has not committed its session reference
    When cleanup runs while the writer retains its session lease
    Then cleanup defers deletion without waiting for the writer
    And the writer can commit its reference and cold reopen the original message

  Scenario: Cleanup preserves messages when committed session evidence is unavailable
    Given stored messages exist but the session directory is absent
    When cleanup discovers orphaned messages
    Then cleanup defers deletion without treating missing evidence as an empty reference set
    And the original messages remain available after cold reopening

  Scenario: Metadata updates preserve another store's messages and metadata
    Given two independent stores with a cached copy of the same message
    When one store appends a message and updates the shared message metadata
    And the other store adds different metadata to the shared message
    Then a cold reopened store reads both messages and both metadata updates

  Scenario: Appending after another store compacts history uses current offsets
    Given two independent stores opened with retained and orphaned messages
    When one store removes the orphaned messages
    And the other store appends a new message
    Then a cold reopened store reads the retained and new messages correctly
    And the orphaned messages stay absent from the index

  Scenario: Cleanup retains referenced messages appended by another store
    Given two independent stores opened before either writes a message
    When one store appends a referenced message and an orphaned message
    And the other store cleans up history retaining the referenced message
    Then cleanup removes exactly the orphaned message
    And a cold reopened store reads the referenced message

  Scenario: Concurrent independent writers retain every message
    Given independent stores opened before any writer appends
    When the stores append distinct messages concurrently
    Then a cold reopened store reads every committed message with its original content
    And reopening uses the saved index without rebuilding it
