@FEAT-012
@cli
@generator
@example-mapping
Feature: Capture architecture notes during Example Mapping
  """
  Adds architectureNotes array field to WorkUnit interface in src/types/index.ts. Creates new CLI commands: add-architecture-note, remove-architecture-note. Enhances generate-scenarios to populate docstring from captured notes instead of TODO placeholders. Integrates architecture questions into Example Mapping Step 3 (Ask Questions phase).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a AI agent doing Example Mapping
  #   I want to capture architecture notes and non-functional requirements
  #   So that generated feature files have complete technical context in docstrings
  #
  # BUSINESS RULES:
  #   1. Architecture notes must be stored in WorkUnit during Example Mapping
  #   2. Architecture notes must populate docstring in generate-scenarios output
  #   3. Must support adding/removing/viewing architecture notes via CLI
  #   4. Architecture notes should include: dependencies, performance reqs, security considerations, implementation constraints
  #
  # EXAMPLES:
  #   1. User runs 'fspec add-architecture-note WORK-001 "Uses @cucumber/gherkin parser"' during Example Mapping
  #   2. User runs 'fspec generate-scenarios WORK-001' and docstring contains captured architecture notes instead of TODO placeholders
  #   3. User runs 'fspec show-work-unit WORK-001' and sees architecture notes section with all captured notes
  #   4. Generated docstring includes sections for dependencies, performance, security based on captured notes
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should architectureNotes be a separate array field in WorkUnit, or reuse assumptions field?
  #   A: true
  #
  #   Q: Should we ask about architecture notes as a separate Example Mapping step (purple cards), or integrate with existing steps?
  #   A: true
  #
  #   Q: What specific NFR questions should AI ask during discovery? (performance, security, dependencies, scalability, etc.)
  #   A: true
  #
  #   Q: Should architecture notes have categories/types, or just be free-form strings?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent doing Example Mapping
    I want to capture architecture notes and non-functional requirements
    So that generated feature files have complete technical context in docstrings

  Scenario: Add architecture note during Example Mapping
    Given I have a work unit WORK-001 in specifying status
    When I run "fspec add-architecture-note WORK-001 'Uses @cucumber/gherkin parser'"
    Then the architecture note should be added to the work unit
    And when I run "fspec show-work-unit WORK-001"
    Then I should see an "Architecture Notes:" section
    And the section should contain "Uses @cucumber/gherkin parser"

  Scenario: Generate scenarios populates docstring with architecture notes
    Given I have a work unit with architecture notes captured
    And the work unit has the note "Uses @cucumber/gherkin parser"
    And the work unit has the note "Must complete validation within 2 seconds"
    When I run "fspec generate-scenarios WORK-001"
    Then the generated feature file should have a docstring
    And the docstring should contain "Uses @cucumber/gherkin parser"
    And the docstring should contain "Must complete validation within 2 seconds"
    And the docstring should NOT contain placeholder text

  Scenario: View architecture notes in work unit
    Given I have added architecture notes to WORK-001
    When I run "fspec show-work-unit WORK-001"
    Then I should see an "Architecture Notes:" section
    And the section should list all captured notes with indices
    And the notes should be displayed in the order they were added

  Scenario: Remove architecture note from work unit
    Given I have a work unit with 3 architecture notes
    When I run "fspec remove-architecture-note WORK-001 1"
    Then the architecture note at index 1 should be removed
    And when I run "fspec show-work-unit WORK-001"
    Then I should see 2 remaining architecture notes

  Scenario: Generated docstring organizes notes by category
    Given I have architecture notes with natural prefixes
    And I have note "Dependency: @cucumber/gherkin parser"
    And I have note "Performance: Must complete within 2 seconds"
    And I have note "Refactoring: Share validation logic with formatter"
    When I run "fspec generate-scenarios WORK-001"
    Then the docstring should group notes by detected prefix
    And dependency notes should appear first
    And performance notes should appear in their own section
