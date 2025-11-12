@done
@research-tools
@formatter
@high
@research
@jira
@bug-fix
@markdown-formatting
@BUG-076
Feature: JIRA research tool outputs [object Object] in markdown format for description field

  """
  Bug in formatIssueMarkdown() function at src/research-tools/jira.ts:146 - directly outputs ADF object instead of parsing it. ADF (Atlassian Document Format) is a nested JSON structure with type/version/content fields. Parser must recursively traverse content array, extract text nodes, and convert to markdown. JSON format unaffected because it preserves object structure.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. JIRA API returns description field as Atlassian Document Format (ADF) object, not plain string
  #   2. ADF object has nested content structure with type/version/content fields
  #   3. Markdown formatter must recursively parse ADF content array to extract text nodes
  #   4. JSON format works correctly because it preserves ADF object structure
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=jira --issue CCS-6' and sees description text instead of [object Object]
  #   2. Issue CCS-6 description 'The frontend interface should be user-friendly...' displays correctly in markdown format
  #   3. Empty or missing description field shows 'No description provided' instead of [object Object]
  #
  # ========================================

  Background: User Story
    As a developer using fspec research tool
    I want to view JIRA issue descriptions in markdown format
    So that I can read issue details without switching to JSON format

  Scenario: Parse ADF description and display as markdown text
    Given a JIRA issue has a description in Atlassian Document Format
    When I run "fspec research --tool=jira --issue CCS-6"
    Then the output should contain the description text
    And the output should not contain "[object Object]"

  Scenario: Display specific issue description correctly
    Given JIRA issue CCS-6 has description "The frontend interface should be user-friendly and responsive"
    When I run "fspec research --tool=jira --issue CCS-6"
    Then the markdown output should contain "The frontend interface should be user-friendly and responsive"
    And the description section should be human-readable

  Scenario: Handle empty or missing description gracefully
    Given a JIRA issue has no description field
    When I run "fspec research --tool=jira --issue" with that issue key
    Then the output should contain "No description provided"
    And the output should not contain "[object Object]"
