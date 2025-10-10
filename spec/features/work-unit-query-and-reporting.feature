@wip
@phase7
@cli
@project-management
@querying
@reporting
Feature: Work Unit Query and Reporting
  """
  Architecture notes:
  - Powerful query interface for work units
  - Support filtering by status, epic, prefix, tags, dates
  - JSON and formatted output modes
  - Statistical reports and summaries
  - Export capabilities for external tools

  Critical implementation requirements:
  - MUST support compound filters (AND/OR logic)
  - MUST support JSON output for programmatic use
  - MUST support sorting by various fields
  - MUST calculate aggregate statistics
  - MUST support date range queries
  - MUST export to JSON, CSV, Markdown

  Query capabilities:
  - Filter by status, epic, prefix, estimate range
  - Search by title/description text
  - Find blocked work, work with questions
  - Show work by assignee (@mentions in questions)

  References:
  - Project Management Design: project-management.md (section 13: Querying)
  """

  Background: User Story
    As an AI agent analyzing project state
    I want to query and report on work units with flexible filters
    So that I can understand current status and make informed decisions

  @critical
  @happy-path
  Scenario: Query work units by status
    Given I have a project with spec directory
    And work units exist with various statuses
    When I run "fspec query work-units --status=implementing --output=json"
    Then the output should contain only implementing work units
    And the output should be valid JSON

  @query
  @filtering
  Scenario: Query work units by epic
    Given I have a project with spec directory
    And work units exist in multiple epics
    When I run "fspec query work-units --epic=epic-auth"
    Then the output should contain only work units in epic-auth

  @query
  @compound
  Scenario: Query with compound filters
    Given I have a project with spec directory
    When I run "fspec query work-units --status=implementing --epic=epic-auth --output=json"
    Then the output should contain work units matching ALL criteria

  @statistics
  @reporting
  Scenario: Generate project summary report
    Given I have a project with spec directory
    And work units exist across all states
    When I run "fspec report summary"
    Then the output should show total work units
    And the output should show breakdown by status
    And the output should show total story points
    And the output should show velocity metrics

  @export
  @integration
  Scenario: Export work units to JSON
    Given I have a project with spec directory
    When I run "fspec export work-units --format=json --output=work-units-export.json"
    Then the file should contain valid JSON array
    And each work unit should have all fields

  @visualization
  @board
  Scenario: Display Kanban board view
    Given I have a project with spec directory
    When I run "fspec board"
    Then the output should show columns for each state
    And each column should list work units
    And work units should show ID, title, and estimate
