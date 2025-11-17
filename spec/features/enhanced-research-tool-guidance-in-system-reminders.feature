@done
@cli
@high
@system-reminders
@research-tools
@REMIND-013
Feature: Enhanced research tool guidance in system reminders
  """
  Uses RES-018's getToolConfigurationStatus() from research-tools/registry.ts for configuration validation. Integrates with existing system-reminder.ts infrastructure. Makes getStatusChangeReminder() async to support tool configuration checking. Adds workUnitCreatedReminder() function for creation-time guidance. Implements detectWorkUnitIntent() for keyword-based AST/Perplexity emphasis. All reminder functions use RES-018's config loading (spec/fspec-config.json, ~/.fspec/fspec-config.json).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System reminders must show all 5 bundled research tools (ast, perplexity, jira, confluence, stakeholder) with their configuration status
  #   2. Configuration status must be determined using RES-018's getToolConfigurationStatus() function, not environment variables
  #   3. When work unit title contains code-related keywords (refactor, implement, create, add, update), emphasize AST tool in reminder
  #   4. When work unit title contains research keywords (research, explore, investigate), emphasize Perplexity tool with natural language guidance
  #   5. Reminders must be emitted at TWO points: work unit creation AND when entering specifying state
  #   6. For unconfigured tools, reminder must show JSON config example from RES-018's getConfigExample()
  #   7. Yes, reminders should be tailored by type. Stories: emphasize new functionality/refactoring scenarios. Bugs: emphasize different strategies like asking Perplexity for solutions and using AST to check linkage/structure. Tasks: emphasize code analysis and infrastructure work.
  #   8. Make getStatusChangeReminder() fully async (Option B). Only one call site to update, already have async specifyingStateReminder(), and it's future-proof for adding tool guidance to other states if needed. Clean architecture with await handling both Promise and string returns.
  #   9. Tool emphasis should show in BOTH creation and specifying reminders. Creation (backlog): AI needs immediate guidance for initial discovery/research. Specifying: AI needs tool guidance for Example Mapping to answer questions and gather rules/examples. Both states benefit from context-aware AST/Perplexity emphasis based on work unit type and title.
  #
  # EXAMPLES:
  #   1. AI creates task 'Refactor authentication module', reminder shows 'CODE-RELATED task' and emphasizes AST tool with example commands
  #   2. AI creates story 'Research OAuth2 implementation patterns', reminder shows 'RESEARCH-HEAVY story' and emphasizes Perplexity with 'USE NATURAL LANGUAGE' guidance
  #   3. AI moves work unit to specifying state, reminder lists all 5 tools with ✓ for configured and ✗ for unconfigured, showing JSON config examples for unconfigured tools
  #   4. Perplexity is not configured, reminder shows '✗ perplexity (not configured)' with JSON example showing {"research": {"perplexity": {"apiKey": "..."}}}
  #   5. AST tool is always shown as '✓ ast (configured)' with reason 'No configuration required'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should reminders be different for bugs vs stories vs tasks, or only based on title keywords?
  #   A: true
  #
  #   Q: The research document says reminder functions must be async. Should we add async support to ALL state reminders or just specifying and creation?
  #   A: true
  #
  #   Q: Should tool emphasis (AST vs Perplexity) only show in creation reminders, only in specifying reminders, or both?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to receive context-aware research tool guidance in system reminders
    So that I use the right research tool at the right time with proper natural language queries

  Scenario: Creation reminder for code-related task emphasizes AST
    Given I am creating a task with title "Refactor authentication module"
    When the work unit is created
    Then the system reminder should contain "CODE-RELATED task"
    And the reminder should emphasize the AST tool
    And the reminder should show example AST commands
    And the reminder should list all 5 research tools with configuration status

  Scenario: Creation reminder for research-heavy story emphasizes Perplexity
    Given I am creating a story with title "Research OAuth2 implementation patterns"
    When the work unit is created
    Then the system reminder should contain "RESEARCH-HEAVY story"
    And the reminder should emphasize the Perplexity tool
    And the reminder should contain "USE NATURAL LANGUAGE" guidance
    And the reminder should list all 5 research tools with configuration status

  Scenario: Specifying state reminder shows all tools with configuration status
    Given I have a work unit in backlog status
    And Perplexity is not configured
    And JIRA is configured
    When I move the work unit to specifying state
    Then the system reminder should list all 5 tools
    And AST should show as "✓ ast (configured)"
    And Perplexity should show as "✗ perplexity (not configured)"
    And unconfigured tools should show JSON config examples
    And the reminder should reference spec/fspec-config.json

  Scenario: Unconfigured tool shows JSON configuration example
    Given Perplexity is not configured
    When a system reminder is displayed
    Then the reminder should show "✗ perplexity (not configured)"
    And the reminder should include JSON config example from RES-018
    And the config example should show the research.perplexity.apiKey structure

  Scenario: AST tool always shows as configured
    Given no research tools are configured except AST
    When a system reminder is displayed
    Then AST should show as "✓ ast (configured)"
    And the status reason should be "No configuration required"

  Scenario: Bug creation reminder emphasizes Perplexity and AST for diagnostics
    Given I am creating a bug with title "Fix login crash on invalid credentials"
    When the work unit is created
    Then the reminder should emphasize using Perplexity for solution research
    And the reminder should emphasize using AST to check code linkage
    And the reminder should suggest AST commands for finding related code

  Scenario: getStatusChangeReminder is fully async
    Given the getStatusChangeReminder function exists
    When it checks tool configuration status
    Then it should use async/await to call RES-018's getToolConfigurationStatus
    And it should return a Promise
    And the caller in update-work-unit-status should await the result
