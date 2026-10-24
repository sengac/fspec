@done
@persistence
@context-management
@compaction
@phase-1
@CMPCT-064
Feature: Session manifest delete ownership
  """
  Both session deletion front doors share the manifest writer lease and reject
  competing processes without removing authoritative session bytes.
  """

  Scenario: Competing processes cannot delete an owned session
    Given a process owns a committed session writer lock
    When another process deletes through the store or public facade
    Then deletion reports an ownership failure and preserves the committed bytes
