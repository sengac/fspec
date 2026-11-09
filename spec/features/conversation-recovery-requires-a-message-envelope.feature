@done
@BUG-332
@persistence
@session-resume
Feature: Conversation recovery commits only records with a message envelope
  """
  The agent loop persists conversation rows through recovery records committed by
  persistence_retry_commit::commit (rust/agent-loop/src/persistence_retry_commit.rs).
  Every production record carries a serialized MessageEnvelope; a conversation record
  without one is refused instead of being stored as a bare row that replay could not
  reconstruct (session-replay-keeps-stored-envelopes).
  """

  Background: User Story
    As a client resuming a persisted session
    I want every committed conversation row to carry its message envelope
    So that replay never meets a row it would have to reconstruct

  Scenario: A recovery record without a message envelope is not committed
    Given a session whose recovery journal holds a conversation record without a message envelope
    When the journal is committed
    Then the commit fails because the record carries no message envelope
    And the session gains no conversation row
