@cli
@guidance
@reverse-acdd
@discovery
@phase1
@FOUND-002
Feature: Implement Automated Discovery - Code Analysis

  """
  Integration: Output from code analysis becomes INPUT to interactive questionnaire (FOUND-003). AI shows [DETECTED] values that human can confirm/edit/skip. This supports reverse ACDD for existing codebases.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Code analysis must detect project type (web app, CLI tool, library, service, mobile app, desktop app, API)
  #   2. Code analysis must infer personas from user-facing interactions (routes, commands, UI components, API endpoints)
  #   3. Code analysis must infer capabilities from code structure (high-level features, not implementation details)
  #   4. Analysis must be fast enough for interactive use (< 5 seconds for typical projects)
  #   5. fspec provides GUIDANCE to AI, not implementation - AI decides how to analyze code
  #   6. Discovery must infer: project type, personas, capabilities, AND problems/pain points
  #   7. Support monorepos - AI analyzes all packages that are part of the same project
  #
  # EXAMPLES:
  #   1. CLI tool with commander.js commands → project type: cli-tool, persona: Developer using CLI
  #   2. Express.js with /api routes → project type: web-app, personas: API Consumer, End User
  #   3. Package with exports in package.json → project type: library, persona: Developer integrating library
  #   4. React components in src/ → capability: User Interface, not implementation detail like 'Uses React hooks'
  #   5. AI discovers React app → infers problem: 'Users need interactive web UI' not 'Code needs React'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should code analysis support monorepos with multiple package.json files, or just single-project repositories?
  #   A: true
  #
  #   Q: What file patterns should we analyze to detect project type? (package.json, tsconfig.json, Cargo.toml, go.mod, etc.)
  #   A: true
  #
  #   Q: Should we infer problems/pain points from code, or only project type and personas?
  #   A: true
  #
  #   Q: How deep should directory traversal go? Should we limit to specific directories (src/, lib/, cmd/) or scan entire project?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer using fspec to document an existing codebase
    I want to automatically discover project structure and infer personas/capabilities
    So that I can quickly bootstrap a foundation document without manual analysis

  Scenario: Discover CLI tool project type and developer persona
    Given I have guidance documentation for discovering CLI tools
    When AI analyzes codebase with commander.js command structure
    Then AI should infer project type as 'cli-tool'
    And AI should infer persona 'Developer using CLI in terminal'


  Scenario: Discover web app project type with multiple personas
    Given I have guidance documentation for discovering web applications
    When AI analyzes codebase with Express routes and React components
    Then AI should infer project type as 'web-app'
    And AI should infer persona 'End User' from UI components
    And AI should infer persona 'API Consumer' from API routes


  Scenario: Discover library project type from package exports
    Given I have guidance documentation for discovering libraries
    When AI analyzes codebase with package.json exports field
    Then AI should infer project type as 'library'
    And AI should infer persona 'Developer integrating library into their codebase'


  Scenario: Infer capabilities focusing on WHAT not HOW
    Given I have guidance documentation for capability inference
    When AI analyzes React components for user features
    Then AI should infer capability 'User Interface' not 'Uses React hooks'
    And AI should focus on high-level features not implementation details


  Scenario: Infer problems as WHY not implementation details
    Given I have guidance documentation for problem inference
    When AI discovers a React app codebase
    Then AI should infer problem 'Users need interactive web UI' not 'Code needs React'
    And AI should focus on user needs not technical implementation choices

