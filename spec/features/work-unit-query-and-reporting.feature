@COV-051
@COV-050
@COV-049
@wip
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
