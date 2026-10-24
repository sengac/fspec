@done
@persistence
@context-management
@compaction
@phase-1
@CMPCT-064
Feature: Durable session manifest transactions
  Session updates preserve unrelated committed fields and immutable message references.
  Failed writes never masquerade as committed state.

  """
  One process owns each session writer lock. Supported Unix platforms promote a
  synced staging file by atomic rename and sync the containing directory. Windows
  must reject unsupported promotion rather than claiming equivalent durability.
  Revision checked snapshots reject stale writers. Field updates serialize against
  the current committed revision. Staged files are not authoritative. Promotion is
  irreversible even when acknowledgement is lost. Generic and provider history
  must share the same durable generation before resume permits inference.
  """

  Scenario: Concurrent field writers retain all committed updates
    Given token provider and message writers share one persisted session
    When barrier synchronized writers commit their owned fields
    Then token usage provider metadata and every message reference survive cold open

  Scenario: Stale whole snapshots cannot overwrite a newer commit
    Given two snapshots of the same committed session
    When one snapshot commits and the other attempts a whole snapshot save
    Then the stale save reports a revision conflict and preserves the newer commit

  Scenario: Provider session creation publishes the committed revision
    Given a fresh session store
    When a session is created with an explicit provider
    Then the returned cached and durable snapshots share the committed revision

  Scenario: Failed promotion cannot publish a changed cache
    Given a readable committed session and an unwritable promotion directory
    When a rename attempts to commit a new session name
    Then the write fails and both cached and durable state remain unchanged before permissions are restored

  Scenario: Interrupted staging preserves the committed revision
    Given an unpromoted staging file beside a committed session
    When a fresh store opens the session
    Then only the old committed revision is visible

  Scenario: Competing processes cannot mutate an owned session
    Given a process owns the session writer lock
    When another process attempts to save that session
    Then the competing process receives an explicit ownership failure

  Scenario: Unreadable committed manifests are not absent sessions
    Given an incompatible committed manifest on disk
    When a fresh store opens the sessions directory
    Then opening fails explicitly instead of silently omitting the session

  Scenario: Missing committed message references block replay
    Given a committed session referencing an unavailable original message
    When a cold reader loads the session replay envelopes
    Then replay fails explicitly instead of omitting the committed message
