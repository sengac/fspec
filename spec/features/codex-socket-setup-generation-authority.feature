@done
@CMPCT-078
@compaction
@agent-core
@context-management
@phase-1
@bug
Feature: Codex socket setup generation authority
  """
  Every physical attempt, including retries after rejection or credential setup,
  must revalidate the generation captured by the originating request. Server-side
  truncation cannot substitute for the complete selected DAG. Admission must
  reserve at least the actual serialized output allowance before any send.
  """

  Background: User Story
    As an agent using full-current-DAG Codex requests
    I want every physical attempt authorized against its complete request
    So that retry and request options cannot bypass committed context authority

  Scenario: Reset during socket setup prevents the response create message
    Given a Codex stream is waiting for its socket handshake
    When its generation is reset before the handshake is released
    Then the stream fails without a response create message
    And the reset checkpoint remains unchanged
