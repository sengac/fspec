@validator
@schema-design
@foundation
@phase1
@FOUND-001
Feature: Design Generic Foundation Schema
  """
  TypeScript interfaces must map exactly to JSON Schema definitions
  Use Ajv with ajv-formats for validation (uri, email, date-time formats)
  Hierarchical foundations: parent foundation.json can reference child foundations in subFoundations array
  Migration path needed: old schema → new schema with backward compatibility flag
  Mermaid validation must use mermaid.parse() with jsdom (existing pattern)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Multiple problems supported. Complex products can have thousands of problems. Foundation.json can reference external sub-foundation documents for scalability (all built from single foundation.json). This creates a hierarchical PRD structure.
  #   2. Broad capabilities only (3-7 high-level abilities). Granular features belong in .feature files. Example: 'User Authentication' is a capability in foundation.json, 'Login with OAuth' is a feature in user-authentication.feature.
  #   3. REQUIRED: project identity, problem statement, solution overview. OPTIONAL: architecture diagrams, constraints, detailed personas. This ensures minimum viable PRD while allowing detailed documentation.
  #   4. Schema must focus ONLY on WHY (problem) and WHAT (solution), never HOW (implementation)
  #   5. Schema must work for ANY project type: web apps, CLI tools, libraries, services, mobile apps
  #   6. Mermaid diagram validation must be preserved from current implementation
  #   7. JSON Schema must use Ajv for validation with clear error messages
  #
  # EXAMPLES:
  #   1. Web app foundation includes personas: 'End User', 'Admin', 'API Consumer'
  #   2. CLI tool foundation includes persona: 'Developer using CLI in terminal'
  #   3. Library foundation includes persona: 'Developer integrating library into their codebase'
  #   4. Problem space includes multiple problems with impact ratings (high/medium/low)
  #   5. Solution includes 3-7 high-level capabilities like 'User Authentication', 'Data Visualization', 'API Integration'
  #   6. Foundation.json can reference sub-foundations: { 'subFoundations': ['spec/foundations/auth-subsystem.foundation.json'] }
  #
  # ASSUMPTIONS:
  #   1. Unlimited personas allowed, but suggest 3-7 in documentation as best practice. No hard limits enforced by schema.
  #
  # ========================================
  Background: User Story
    As a developer using fspec for any project type
    I want to create a generic foundation document that captures WHY and WHAT
    So that I have clear product requirements without mixing in implementation details

  Scenario: Define foundation schema for web application
    Given I am designing a foundation schema for any project type
    And the schema must support personas like "End User", "Admin", "API Consumer"
    When I validate a web app foundation.json with 3 personas
    Then the schema should accept unlimited personas
    And the schema should validate persona structure (name, description, goals)
    And the schema should suggest 3-7 personas as best practice in documentation

  Scenario: Define foundation schema for CLI tool
    Given I am designing a foundation schema for CLI applications
    And the schema must support persona "Developer using CLI in terminal"
    When I validate a CLI tool foundation.json
    Then the schema should accept CLI-specific personas
    And the schema should work for any project type (web, CLI, library, service)

  Scenario: Define foundation schema for library/SDK
    Given I am designing a foundation schema for libraries
    And the schema must support persona "Developer integrating library into their codebase"
    When I validate a library foundation.json
    Then the schema should accept library-specific personas
    And the schema should not require web-specific or CLI-specific fields

  Scenario: Support multiple problems with impact ratings
    Given I am defining the problem space structure
    And complex products can have thousands of problems
    When I validate a foundation.json with multiple problems
    Then the schema should accept an array of problems
    And each problem should support impact rating (high/medium/low)
    And each problem should include frequency and cost fields

  Scenario: Define solution space with high-level capabilities
    Given I am defining the solution space structure
    And capabilities should be broad (3-7 items), not granular features
    When I validate a foundation.json with capabilities like "User Authentication", "Data Visualization"
    Then the schema should accept 3-7 high-level capabilities
    And the schema should NOT include granular features (those belong in .feature files)
    And the schema should focus on WHAT the system does, not HOW

  Scenario: Support hierarchical foundation documents with sub-foundations
    Given I am designing support for complex products with subsystems
    And foundation.json can reference external sub-foundation documents
    When I validate a foundation.json with subFoundations array
    Then the schema should accept subFoundations field containing file paths
    And each sub-foundation path should point to another foundation.json
    And this creates a hierarchical PRD structure for scalability

  Scenario: Validate required vs optional sections
    Given the schema must define required and optional sections
    When I validate a foundation.json
    Then project identity section should be REQUIRED
    And problem statement section should be REQUIRED
    And solution overview section should be REQUIRED
    And architecture diagrams section should be OPTIONAL
    And constraints section should be OPTIONAL
    And detailed personas section should be OPTIONAL

  Scenario: Enforce WHY/WHAT boundary (no HOW)
    Given the schema must focus ONLY on WHY (problem) and WHAT (solution)
    And the schema must never include HOW (implementation details)
    When I validate a foundation.json containing implementation details
    Then the schema validation should reject HOW content
    And the schema should only accept problem statements and solution capabilities
    And implementation details should be flagged as invalid

  Scenario: Preserve Mermaid diagram validation
    Given the current implementation validates Mermaid diagrams
    And diagram validation uses mermaid.parse() with jsdom
    When I define the architecture diagrams section in new schema
    Then Mermaid diagram validation must be preserved
    And invalid Mermaid syntax should be rejected with clear error messages

  Scenario: Use Ajv for JSON Schema validation
    Given the schema must use Ajv validator
    And Ajv must include ajv-formats for uri, email, date-time validation
    When I validate a foundation.json against the schema
    Then Ajv should provide clear, actionable error messages
    And validation errors should include field path and reason
    And TypeScript interfaces must map exactly to JSON Schema definitions
