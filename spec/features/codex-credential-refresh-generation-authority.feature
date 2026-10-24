@done
@CMPCT-078
@compaction
@agent-core
@context-management
@phase-1
@bug
Feature: Codex credential refresh generation authority
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

  Scenario: Reset during credential refresh prevents the authorized retry
    Given a Codex request is rejected and waits for refreshed credentials
    When its generation is reset before credential refresh is released
    Then the stream fails without another physical inference request
    And the reset checkpoint remains unchanged
