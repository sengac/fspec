@BOARD-024
@tui
@dialog
@board
@integration
@rust
@phase-1
Feature: Board card details planning sections
  """
  Full card details present all relevant stored information, not only the
  description and metadata. The complete Example Mapping (user story, rules,
  examples, questions with their answers, assumptions and architecture notes)
  and the complete Event Storm are rendered as named sections. Event Storm
  items are grouped under their type headings (domain events, commands,
  policies, hotspots, aggregates, bounded contexts and external systems) and
  keep their type-specific fields such as policy when/then, hotspot concerns,
  aggregate responsibilities, bounded context descriptions and external
  integration types. Relationships, epic, estimate, history, attachments,
  timestamps and unknown fields remain visible beside these sections.
  """

  Background: User Story
    As a developer using the kanban board
    I want card details to show the example mapping and event storm
    So that I can review all planning context without leaving the board

  Scenario: Details show complete example mapping and event storm sections
    Given a canonical card with example mapping and every event storm item type
    When the user opens full card details
    Then an Example Mapping section shows the user story rules examples answered questions assumptions and architecture notes
    And an Event Storm section lists every item under its type heading with its type-specific fields
