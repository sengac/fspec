@phase2
@setup
@integration
@reverse-engineering
@acdd
@claude-code
@slash-command
@project-management
Feature: Reverse ACDD for Existing Codebases
  """
  Architecture notes:
  - This feature provides a Claude Code slash command (/rspec) for reverse engineering existing applications
  - Command installed by `fspec init` creates `.claude/commands/rspec.md`
  - rspec.md links to fspec.md and explains reverse engineering workflow
  - Analyzes codebase to infer user stories, personas, and acceptance criteria
  - Creates all fspec artifacts: work units, epics, prefixes, feature files, foundation.json updates
  - Generates skeleton tests (structure only, not implemented) with links to feature files
  - Uses Example Mapping when code is ambiguous
  - User story maps stored as Mermaid diagrams in foundation.json

  Critical implementation requirements:
  - MUST install .claude/commands/rspec.md via `fspec init`
  - MUST link to fspec.md as first instruction in rspec.md
  - MUST identify user-facing interactions (routes, commands, UI components)
  - MUST group interactions into epics
  - MUST create work units for each user story
  - MUST generate feature files following existing patterns
  - MUST create skeleton test files with proper header linking to feature files
  - MUST update foundation.json with user story maps (Mermaid diagrams)
  - MUST use fspec commands for all artifact creation
  - MUST handle ambiguous code by offering Example Mapping with human
  - Completion criteria: all user-facing interactions documented, all epics have work units

  Artifact creation order:
  1. Foundation.json updates (user story maps)
  2. Epics and prefixes registration
  3. Work units creation
  4. Feature files generation
  5. Skeleton test files creation

  References:
  - ACDD Workflow: spec/CLAUDE.md
  - Work Unit: RSPEC-001
  """

  Background: User Story
    As an AI agent working with an existing codebase without specifications
    I want to reverse engineer the application to discover user stories and acceptance criteria
    So that I can create proper Gherkin specifications and follow ACDD going forward

  @RSPEC-001
  Scenario: Install rspec.md via fspec init
    Given I run "fspec init" to set up a new project
    When the init command completes
    Then a file ".claude/commands/rspec.md" should be created
    And the first line should link to fspec.md: "fully read fspec.md"
    And the file should contain reverse engineering workflow instructions
    And the file should include examples of identifying user stories from code

  Scenario: Invoke rspec slash command in Claude Code
    Given I have an existing codebase without specifications
    And I have run "fspec init" to install rspec.md
    When I run "/rspec" in Claude Code
    Then Claude should read rspec.md
    And Claude should read fspec.md
    And Claude should begin analyzing the codebase for user interactions

  Scenario: Identify user-facing interactions from codebase
    Given I have an Express.js application with routes
    And the application has routes: POST /login, GET /dashboard, POST /checkout
    When I run "/rspec"
    Then Claude should identify user interactions:
      | Interaction       | Component Type | Inference Source     |
      | User Login        | Authentication | POST /login route    |
      | View Dashboard    | UI             | GET /dashboard route |
      | Complete Checkout | Payment        | POST /checkout route |

  Scenario: Group interactions into epics
    Given Claude has identified user interactions for authentication, payments, and dashboards
    When Claude groups interactions into epics
    Then the following epics should be created:
      | Epic Name          | Prefix | Description                           |
      | User Management    | AUTH   | Authentication and user sessions      |
      | Payment Processing | PAY    | Checkout and payment workflows        |
      | Dashboard Features | DASH   | User dashboard and data visualization |

  Scenario: Create work units for each user story
    Given Claude has created epics: AUTH, PAY, DASH
    And Claude has identified user stories within each epic
    When Claude creates work units
    Then the following work units should be created:
      | Work Unit | Epic | Title             | Status     |
      | AUTH-001  | AUTH | User Login        | specifying |
      | AUTH-002  | AUTH | User Logout       | backlog    |
      | PAY-001   | PAY  | Complete Checkout | specifying |
      | DASH-001  | DASH | View Dashboard    | specifying |

  Scenario: Generate feature files from inferred acceptance criteria
    Given Claude has created work unit AUTH-001 for "User Login"
    And Claude has analyzed the login route implementation
    When Claude generates the feature file
    Then a file "spec/features/user-login.feature" should be created
    And the feature should have proper tags: @phase1 @authentication @api
    And the feature should have a Background section with user story
    And the feature should have scenarios inferred from code:
      | Scenario                       | Inferred From                   |
      | Login with valid credentials   | Successful auth code path       |
      | Login with invalid credentials | Error handling for 401 response |
      | Login with missing credentials | Validation error handling       |
    And each scenario should have a comment: "Inferred from code - verify with human"

  Scenario: Create skeleton test files with feature links
    Given Claude has created feature file "spec/features/user-login.feature"
    When Claude creates the skeleton test file
    Then a file "src/routes/__tests__/login.test.ts" should be created
    And the test file should have a header:
      """
      /**
       * Feature: spec/features/user-login.feature
       *
       * This test file validates the acceptance criteria defined in the feature file.
       * Scenarios in this test map directly to scenarios in the Gherkin feature.
       *
       * NOTE: This is a skeleton test file generated by reverse ACDD.
       * Tests are NOT implemented - only structure is provided.
       */
      """
    And the test file should have describe blocks matching feature scenarios
    And each test should have a comment: "// TODO: Implement this test"

  Scenario: Update foundation.json with user story maps
    Given Claude has identified user stories and epics
    When Claude updates foundation.json
    Then foundation.json should contain a user story map section
    And the user story map should be a Mermaid diagram:
      """
      graph TB
        User[User] -->|Login| AUTH-001[User Login]
        User -->|Logout| AUTH-002[User Logout]
        User -->|View| DASH-001[View Dashboard]
        User -->|Purchase| PAY-001[Complete Checkout]

        AUTH-001 -->|Success| DASH-001
        DASH-001 -->|Action| PAY-001
      """
    And the Mermaid diagram should be validated with mermaid.parse()

  Scenario: Handle ambiguous code with Example Mapping
    Given Claude encounters ambiguous business logic in the checkout flow
    And the code contains magic numbers and unclear conditional branches
    When Claude attempts to reverse engineer the acceptance criteria
    Then Claude should document what is clear from the code
    And Claude should create a scenario with comment: "AMBIGUOUS: magic number 42 in discount logic - needs human clarification"
    And Claude should offer to run Example Mapping with the human
    And Claude should ask questions about the unclear business rules

  Scenario: Completion criteria - all user interactions documented
    Given Claude has analyzed the entire codebase
    When Claude determines if reverse engineering is complete
    Then all user-facing interactions should have feature files
    And all epics should have at least one work unit
    And foundation.json should contain a complete user story map
    And Claude should report:
      """
      Reverse ACDD complete:
      - 3 epics created (AUTH, PAY, DASH)
      - 4 work units created
      - 4 feature files generated
      - 4 skeleton test files created
      - foundation.json updated with user story map
      - 2 scenarios marked AMBIGUOUS for human review
      """

  Scenario: AI agent workflow - reverse then forward ACDD
    Given I have an existing Express.js application without specifications
    When I run "/rspec" in Claude Code
    Then Claude creates all specifications using reverse ACDD
    And when I later ask Claude to add a new feature
    Then Claude should follow forward ACDD (Discovery → Specify → Test → Implement)
    And Claude should use fspec commands to maintain specifications
    And Claude should update feature files as the codebase evolves
