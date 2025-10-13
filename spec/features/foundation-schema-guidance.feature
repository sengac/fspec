@validator
@partial
@phase2
@setup
@foundation
@json-schema
@validation
@critical
Feature: Foundation Schema Guidance for AI Agents
  """
  Architecture notes:
  - This feature provides JSON Schema for foundation.json to guide AI agents in writing structured, meaningful project documentation
  - Schema is separate file (spec/foundation.schema.json) with rich descriptions and semantic guidance
  - Accepts both camelCase and snake_case field names for flexibility
  - Uses additionalProperties: true to allow custom sections
  - Validation checks semantic content (e.g., "projectOverview should focus on WHAT/HOW, not WHY")
  - Mermaid syntax validation delegated to mermaid.parse(), not JSON Schema
  - Two CLI commands: show-foundation-schema (display) and validate-foundation-schema (validate)
  - No backward compatibility - existing foundation.json may need manual updates

  Critical implementation requirements:
  - MUST create spec/foundation.schema.json with JSON Schema format
  - MUST include rich descriptions for each field explaining intent and what to avoid
  - MUST include examples of good vs. bad content in schema
  - MUST support both camelCase and snake_case field names (e.g., projectOverview OR project_overview)
  - MUST set additionalProperties: true for extensibility
  - MUST provide semantic validation guidance in descriptions
  - MUST delegate Mermaid syntax validation to mermaid.parse()
  - MUST implement fspec show-foundation-schema command
  - MUST implement fspec validate-foundation-schema command
  - Error messages MUST be clear enough for AI to self-correct

  References:
  - JSON Schema Spec: https://json-schema.org/
  - Work Unit: DOC-002
  """

  Background: User Story
    As an AI agent writing project foundation documentation
    I want JSON Schema guidance that explains the intent and format of each section
    So that I write documentation that preserves meaning, stays on point, and follows the expected structure

  @DOC-002
  Scenario: Display foundation schema with rich descriptions
    Given I am an AI agent working on a project using fspec
    When I run "fspec show-foundation-schema"
    Then I should receive the complete JSON Schema for foundation.json
    And the schema should include rich descriptions explaining the intent behind each section
    And the schema should include examples of good vs. bad content for key fields
    And the schema should include format instructions for structured fields
    And the output should be valid JSON Schema format

  Scenario: Schema accepts flexible field naming (camelCase and snake_case)
    Given I have foundation.json with snake_case field names
    And the file uses "architecture_diagrams" instead of "architectureDiagrams"
    And the file uses "technical_requirements" instead of "technicalRequirements"
    And the file uses "project_overview" instead of "projectOverview"
    When I run "fspec validate-foundation-schema"
    Then the validation should pass
    And the output should confirm all fields are valid

  Scenario: Schema provides semantic guidance for projectOverview
    Given I have foundation.json with projectOverview field
    And the schema description states: "Focus on WHAT and HOW, not WHY. Business justification belongs in whyWeAreBuildingIt"
    When I read the schema using "fspec show-foundation-schema"
    Then I should see detailed guidance on projectOverview content
    And the description should specify it should be 2-4 sentences
    And the description should specify to describe: what systems are built, who uses them, how they integrate
    And the description should warn against including business justification

  Scenario: Validate foundation.json with minimum array length requirements
    Given I have foundation.json with problemDefinition.primary.coreProblems
    And the array contains only 2 items
    And the schema requires minimum 3 items for coreProblems
    When I run "fspec validate-foundation-schema"
    Then the validation should fail
    And the error message should state: "Field problemDefinition.primary.coreProblems must have at least 3 items (found 2)"
    And the error message should be clear enough for AI to self-correct

  Scenario: Schema allows additional custom sections for extensibility
    Given I have foundation.json with standard sections (project, whatWeAreBuilding, whyWeAreBuildingIt)
    And I add a custom section called "deploymentStrategy"
    And the schema has additionalProperties: true
    When I run "fspec validate-foundation-schema"
    Then the validation should pass
    And the custom section should be allowed
    And the output should confirm the file is valid

  Scenario: Mermaid syntax validation delegated to mermaid library
    Given I have foundation.json with architectureDiagrams array
    And one diagram contains invalid Mermaid syntax
    And the JSON Schema only validates that mermaidCode is a string
    When I run "fspec validate-foundation-schema"
    Then the JSON Schema validation should pass
    And Mermaid syntax validation should be handled separately by mermaid.parse()

  Scenario: AI agent workflow - read schema, update foundation, validate
    Given I am an AI agent working on updating foundation.json
    When I run "fspec show-foundation-schema"
    Then I receive the schema with rich descriptions and examples
    When I use the schema guidance to update foundation.json with "fspec update-foundation"
    And I run "fspec validate-foundation-schema"
    Then the validation should pass
    And I should have confidence the documentation follows the expected format
