@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Preserve journal ownership while promoting conversation originals
  """
  Uses the single recovery owner from recoverable-conversation-persistence.feature.
  Journal identities remain physical original identities. Transcript records use
  their separate display-only lane and never become model dialogue.
  """

  Scenario: Journal identities remain physical assistant and result identities
    Given an acknowledged journal invocation and its captured result identities
    When assistant and result originals are promoted using those identities
    Then cold reopen contains each original once with its journal identity

  Scenario: Transcript recovery never creates a conversation original
    Given a retained transcript event for a real session
    When persistence recovery promotes the event
    Then the journal contains the event and the conversation archive remains empty

  Scenario: Transcript recovery rejects mismatched ownership
    Given a transcript event with a mismatched session or record identity
    When persistence recovery attempts to promote the event
    Then recovery fails without writing the event or a conversation original
