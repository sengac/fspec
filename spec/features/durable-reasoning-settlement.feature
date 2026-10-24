@done
@CMPCT-069
@agent-core
@context-management
@compaction
@phase-1
Feature: Measured reasoning settlement survives compaction and cold replay
  """
  Physical settlement atomically persists cumulative reasoning alongside billed
  input and output. Duplicate physical identities do not add spend or reasoning.
  Context replacement and generation promotion preserve these cumulative fields.
  """

  Scenario: Measured terminal reasoning survives a compacted cold resume
    Given a persisted session with earlier cumulative reasoning and measured terminal requests
    When duplicate terminal delivery context replacement and compaction are completed
    Then a fresh process restores every measured reasoning token and bills each request once

  Scenario: Reasoning overflow rejects the complete settlement atomically
    Given a persisted session whose cumulative reasoning cannot accept another measured token
    When a new physical request attempts settlement
    Then settlement fails without changing durable bytes billing or physical identities

  Scenario: Unknown reasoning does not erase earlier measured reasoning
    Given a persisted session with measured cumulative reasoning
    When a new physical request settles without measured reasoning detail
    Then cold reload preserves earlier reasoning and settles only the measured spend
