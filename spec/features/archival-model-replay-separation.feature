@done
@session
@rpc
@persistence
@agent-view
@BUG-245
Feature: archival-model-replay-separation
  """
  Model replay alone installs the validated committed summary and suffix. Archival display reads the authoritative original store in manifest order through bounded pages, shares Replay decoding, preserves persisted identities and timestamps, and excludes provider-only state. Display history never changes inference, counters, or manifests.
  """

  Background: User Story
    As a session user
    I want to browse the complete original conversation after compaction
    So that reduced model context does not hide my archived work

  Scenario: Compacted cold resume retains original transcript independently of model input
    Given a saved tool conversation with two committed compactions and an uncompacted suffix
    When a fresh process resumes the session and reads its transcript
    Then every original text tool argument and tool result appears once in order without synthetic user summaries
    And the next physical provider request contains only the committed summary and suffix

  Scenario: Missing compacted originals fail visibly without substituting model summaries
    Given a compacted session whose covered original record is unavailable
    When the archived transcript is requested
    Then a typed archive failure identifies the unavailable original instead of silently omitting it

  Scenario: Large archival hydration is bounded and preserves original identity
    Given a saved archive with many original records and a large tool result
    When its transcript is hydrated repeatedly through bounded cursor pages
    Then all records retain original positions identities timestamps and complete payloads
    And hydration retains at most one decoded original without filling the runtime output buffer or changing durable model state

  Scenario: Compaction metadata marks the covered original boundary without impersonating conversation
    Given a saved tool conversation with two committed compactions and an uncompacted suffix
    When its archive is restored repeatedly
    Then a single labelled compaction marker preserves the committed summary generation timestamp and covered boundary
    And the marker follows the covered originals before the suffix without changing durable state

  Scenario: Fully compacted archives retain their terminal metadata boundary
    Given a compacted conversation without an uncompacted suffix
    When its archive is cold restored
    Then one labelled compaction marker follows all covered originals without original message coordinates
