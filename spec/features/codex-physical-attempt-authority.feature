@done
@CMPCT-078
@compaction
@agent-core
@context-management
@phase-1
@bug
Feature: Codex physical attempts retain current generation authority
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

  Scenario: Reset during transient rejection prevents a stale physical retry
    Given a Codex request receives a transient rejection
    When its generation is reset before the retry is released
    Then the stream fails without sending another physical request
    And the reset checkpoint remains unchanged

  Scenario: Server side truncation is rejected before transport
    Given a full DAG Codex request configured for automatic truncation
    When the request is prepared for streaming or completion
    Then a typed policy failure occurs without a physical request
    And the committed checkpoint remains unchanged

  Scenario: Under reserved wire output is rejected before transport
    Given a Codex request whose wire output allowance exceeds its reservation
    When the physical stream is consumed
    Then a typed admission failure occurs without a physical request
    And the committed checkpoint remains unchanged

  Scenario: Selected request certification rejects under reserved output
    Given a selected provider request whose wire output allowance exceeds its reservation
    When its next generation certificate is prepared
    Then typed admission rejects the unchanged request before any physical send
