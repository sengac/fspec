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

  @visualization
  @board
  Scenario: Display Kanban board view
    Given I have a project with spec directory
    When I run "fspec board"
    Then the output should show columns for each state
    And each column should list work units
    And work units should show ID, title, and estimate

  Scenario: Query by status and prefix
    Given I have a project with spec directory
    And work units exist with various statuses and prefixes
    And work unit "AUTH-001" has status "implementing" and prefix "AUTH"
    And work unit "AUTH-002" has status "implementing" and prefix "AUTH"
    And work unit "API-001" has status "implementing" and prefix "API"
    When I run "fspec query work-units --status=implementing --prefix=AUTH --output=json"
    Then the output should contain only work units matching both criteria
    And the output should include "AUTH-001" and "AUTH-002"
    And the output should not include "API-001"
    And the output should be valid JSON

  @export
  @integration
  Scenario: Export work units to JSON
    Given I have a project with spec directory
    And multiple work units exist with complete metadata
    When I run "fspec export work-units --format=json --output=work-units-export.json"
    Then the command should succeed
    And the file "work-units-export.json" should be created
    And the file should contain valid JSON array
    And each work unit should have all fields: id, title, status, createdAt, updatedAt
    And the export should be usable by external tools

  Scenario: Generate summary report with statistics
    Given I have a project with spec directory
    And work units exist across all Kanban states
    And 3 work units are in "backlog"
    And 2 work units are in "implementing"
    And 5 work units are in "done"
    When I run "fspec report summary"
    Then the output should show total work units: 10
    And the output should show breakdown by status
    And the output should show "backlog: 3"
    And the output should show "implementing: 2"
    And the output should show "done: 5"
    And the output should show total story points if estimates exist

  Scenario: Query with sorting by updated date
    Given I have a project with spec directory
    And work unit "AUTH-001" was updated at "2025-10-10T10:00:00Z"
    And work unit "AUTH-002" was updated at "2025-10-11T10:00:00Z"
    And work unit "AUTH-003" was updated at "2025-10-09T10:00:00Z"
    When I run "fspec query work-units --sort=updatedAt --order=desc --output=json"
    Then the output should list work units in descending order by updatedAt
    And the first result should be "AUTH-002"
    And the second result should be "AUTH-001"
    And the third result should be "AUTH-003"

  Scenario: Export filtered results to CSV
    Given I have a project with spec directory
    And work units exist with status "implementing"
    When I run "fspec query work-units --status=implementing --format=csv --output=implementing.csv"
    Then the command should succeed
    And the file "implementing.csv" should be created
    And the file should contain CSV headers: id,title,status,createdAt,updatedAt
    And the file should contain only work units with status "implementing"
    And the CSV should be valid and importable to spreadsheet applications

  Scenario: Summary report writes to file and returns output path
    Given I have a project with spec directory
    And work units exist across multiple states
    When I run "fspec generate-summary-report --format=markdown"
    Then the command should succeed
    And the file "spec/summary-report.md" should be created
    And the output should display "Report generated: spec/summary-report.md"
