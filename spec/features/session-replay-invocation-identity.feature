@done
@CMPCT-061
@phase-1
@agent-core
@compaction
@context-management
@bug
Feature: Session replay preserves distinct tool item and invocation identities
  """
  The session replay converter preserves provider invocation identity separately
  from the persisted tool item identity. The next physical Responses request
  must pair the original invocation with its result, not substitute the item ID.
  Explicit malformed invocation identities are rejected before replay mutation.
  """

  Scenario: Distinct invocation identities survive session replay onto the next request
    Given structured tool call envelopes with distinct item and invocation identities
    When the session restores the structured envelopes and sends its next request
    Then the physical request preserves the item identity and pairs the original invocation

  Scenario: Invalid invocation identity fails without installing partial replay
    Given a valid replay prefix followed by a tool call with invalid invocation identity
    When the session restores the structured envelopes
    Then restoration fails without changing history or publishing replay output
