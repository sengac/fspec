@phase6
@cli
@foundation-management
@modification
@medium
@unit-test
Feature: Update Foundation Section Content
  """
  Architecture notes:
  - Updates or creates sections in FOUNDATION.md
  - Sections are top-level (## headers) in the markdown file
  - Replaces entire section content while preserving other sections
  - Creates FOUNDATION.md if it doesn't exist
  - Preserves diagrams and subsections within replaced section
  - Uses standard markdown formatting

  Critical implementation requirements:
  - MUST accept section name (e.g., "What We Are Building", "Why")
  - MUST accept section content (multi-line text)
  - MUST create section if it doesn't exist
  - MUST replace existing section content completely
  - MUST preserve other sections in the file
  - MUST preserve proper markdown structure
  - MUST handle multi-line content correctly
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer documenting project foundation
    I want to update section content in FOUNDATION.md
    So that I can maintain up-to-date project documentation

  Scenario: Update existing section content
    Given I have a FOUNDATION.md with a "What We Are Building" section
    When I run `fspec update-foundation "What We Are Building" "New content for this section"`
    Then the command should exit with code 0
    And the "What We Are Building" section should contain the new content
    And other sections should be preserved

  Scenario: Create new section if it doesn't exist
    Given I have a FOUNDATION.md without a "Technical Approach" section
    When I run `fspec update-foundation "Technical Approach" "Our technical approach details"`
    Then the command should exit with code 0
    And a new "Technical Approach" section should be created
    And it should contain the specified content

  Scenario: Replace entire section content
    Given I have a "Why" section with existing content
    When I run `fspec update-foundation Why "Completely new reasoning"`
    Then the command should exit with code 0
    And the old content should be completely replaced
    And only the new content should be present in the section

  Scenario: Preserve other sections when updating
    Given I have FOUNDATION.md with "What We Are Building", "Why", and "Architecture" sections
    When I run `fspec update-foundation Why "Updated why section"`
    Then the command should exit with code 0
    And the "What We Are Building" section should be unchanged
    And the "Architecture" section should be unchanged
    And only the "Why" section should have new content

  Scenario: Create FOUNDATION.md if it doesn't exist
    Given I have no FOUNDATION.md file
    When I run `fspec update-foundation "What We Are Building" "A new CLI tool for specifications"`
    Then the command should exit with code 0
    And a FOUNDATION.md file should be created
    And it should contain the "What We Are Building" section
    And the section should have the specified content

  Scenario: Handle multi-line section content
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation Why "Line 1\nLine 2\nLine 3"`
    Then the command should exit with code 0
    And the "Why" section should contain all three lines
    And the lines should be properly formatted

  Scenario: Preserve existing subsections in other sections
    Given I have an "Architecture" section with diagrams (### subsections)
    When I run `fspec update-foundation Why "New content"`
    Then the command should exit with code 0
    And the "Architecture" section diagrams should be preserved
    And only the "Why" section should be modified

  Scenario: Update section at the beginning of file
    Given I have FOUNDATION.md with "Overview" as the first section
    When I run `fspec update-foundation Overview "Updated overview"`
    Then the command should exit with code 0
    And the "Overview" section should have the new content
    And sections after it should be preserved

  Scenario: Update section at the end of file
    Given I have FOUNDATION.md with "Future Plans" as the last section
    When I run `fspec update-foundation "Future Plans" "Updated plans"`
    Then the command should exit with code 0
    And the "Future Plans" section should have the new content
    And sections before it should be preserved

  Scenario: Reject empty section name
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation "" "Some content"`
    Then the command should exit with code 1
    And the output should show "Section name cannot be empty"

  Scenario: Reject empty content
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation Why ""`
    Then the command should exit with code 1
    And the output should show "Section content cannot be empty"

  Scenario: Handle special characters in section names
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation "What We're Building" "Content with apostrophe"`
    Then the command should exit with code 0
    And the section "What We're Building" should be created
    And it should contain the specified content

  Scenario: Preserve markdown formatting in content
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation Features "- Feature 1\n- Feature 2\n- Feature 3"`
    Then the command should exit with code 0
    And the "Features" section should contain a markdown list
    And the list formatting should be preserved

  Scenario: Update section multiple times
    Given I have a "Why" section with content "Original"
    When I run `fspec update-foundation Why "First update"`
    And I run `fspec update-foundation Why "Second update"`
    Then the command should exit with code 0
    And the "Why" section should contain only "Second update"
    And previous content should not be present
