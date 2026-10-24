@CMPCT-063
@done
@phase-1
@persistence
@context-management
@compaction
Feature: Lossless original envelope retrieval
  """
  Architecture notes:
  - Add an explicit SessionSearch originals action; keep search/show preview semantics unchanged.
  - Read immutable structured metadata through core persistence readers, retaining unknown fields and original envelope identities.
  - Page forward over persisted manifest positions with explicit session/range and record/serialized-byte bounds.
  - Return stable persisted message IDs separately from original envelope UUIDs; never synthesize missing envelopes.
  - Resolve stored blob references without lossy schema conversion; missing sources reject the entire page.
  - Only persisted _blobRefs entries authorize blob expansion; literal reference-like content is evidence, not a storage marker.
  - Original pages bypass compaction trimming and identify themselves as original, untruncated evidence.
  """

  Background: User Story
    As an agent reconstructing a session DAG
    I want bounded pages of original persisted envelopes
    So that I can recover exact evidence beyond search previews

  Scenario: Retrieve exact structured evidence beyond previews
    Given a persisted tool result with large UTF-8 content and a sentinel beyond its preview
    And original envelope metadata includes provider-specific nested fields
    When original retrieval is requested while compaction trimming is enabled
    Then the exact structured envelope and sentinel are returned without truncation
    And the persisted identity is separate from the unchanged envelope identity
    And ordinary show still returns its existing preview

  Scenario: Traverse bounded original pages without mixing sessions
    Given two persisted sessions containing multiple structured envelopes
    When bounded forward pages are retrieved for one session and inclusive turn range
    Then each selected original appears exactly once in persisted order
    And continuation identifies the next unread position within that range
    And the final page has no continuation

  Scenario: Reject missing original messages without partial success
    Given a session references a missing original after a valid envelope
    When original retrieval includes both references
    Then a typed missing-message failure is returned without any partial page
    And persisted source state remains unchanged

  Scenario: Resolve original blobs and reject unavailable blob evidence
    Given structured originals contain stored tool result and tool input blobs
    When original retrieval is requested
    Then blob content is restored exactly while unknown nested fields remain unchanged
    But a missing or invalid blob returns a typed failure without a partial page

  Scenario: Reject preview-only sources instead of fabricating envelopes
    Given a persisted message has only searchable preview content
    When original retrieval is requested
    Then a typed missing-envelope failure is returned instead of a fabricated original

  Scenario: Enforce original page bounds without truncating evidence
    Given persisted originals and explicit page size and byte limits
    When invalid ranges or limits are requested
    Then a typed invalid-request failure is returned
    But valid byte-limited pages stop before the next complete envelope
    And an envelope exceeding the byte budget returns a typed oversized-envelope failure

  Scenario: Advertise original retrieval and source selection to DAG builders
    Given the SessionSearch tool definition and DAG builder instructions
    When the original retrieval contract is inspected
    Then explicit session-scoped original retrieval and its bounds are discoverable
    And builders are instructed to select originals rather than treating previews as exact evidence

  Scenario: Preserve literal blob-like content without recorded storage references
    Given original messages contain literal blob-like text and tool input fields
    And those fields have no recorded storage references
    When original retrieval is requested
    Then every literal value and unknown input field is returned unchanged

  Scenario: Validate recorded blob references before returning original evidence
    Given originals externalized by the persistence writer with recorded storage references
    When original retrieval is requested
    Then all recorded content is restored without changing other envelope fields
    But inconsistent or unsupported recorded references return a typed failure without partial evidence
