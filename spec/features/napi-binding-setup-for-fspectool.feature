@done
@feature-management
@cli
@tools
@integration
@CODE-003
Feature: NAPI Binding Setup for FspecTool

  """
  Implement real FspecTool in codelet/tools/src/fspec.rs and expose via NAPI callback pattern
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT - FRESH IMPLEMENTATION
  # ========================================
  #
  # BUSINESS RULES:
  #   1. FspecTool struct must be implemented in codelet/tools/src/fspec.rs as a Rust tool
  #   2. NAPI function must expose callFspecCommand with callback pattern to invoke TypeScript command modules
  #   3. TypeScript callback receives command name, JSON args, and project root path and returns JSON result with system reminders
  #
  # EXAMPLES:
  #   1. Rust agent calls callFspecCommand('list-work-units', '{}', '/project/root', callback) where callback imports listWorkUnits from src/commands/ and returns JSON with work units data
  #   2. Rust agent calls callFspecCommand('create-story', '{"prefix":"TEST","title":"New Feature"}', '/project/root', callback) and receives JSON response with new work unit ID and system reminders about Example Mapping
  #   3. Rust agent calls callFspecCommand with invalid command name and receives error result with clear error message instead of system crash
  #   4. Rust agent calls callFspecCommand('add-rule', '{"workUnitId":"AUTH-001","rule":"Passwords must be 8+ characters"}', '/project', callback) and receives JSON with success status and system reminder about Example Mapping completion
  #   5. Rust agent calls callFspecCommand('board', '{}', '/project', callback) and receives JSON with current Kanban board state showing work units organized by status columns
  #
  # ANSWERED QUESTIONS:
  #   Q1: Should FspecTool implement Debug, Clone, or Default traits?
  #   A1: No traits needed. Use 'pub struct FspecTool;' like BashTool and other existing tools
  #   
  #   Q2: How should callback errors be handled?  
  #   A2: Capture in JSON response with error types and suggestions for rich agent context
  #   
  #   Q3: Which commands to support initially?
  #   A3: All ACDD workflow commands (work units, example mapping, features, board) but exclude setup commands
  #
  # PATTERN:
  #   Rust agent: callFspecCommand(command, args_json, project_root, callback)
  #   NAPI: calls callback(command, args_json, project_root) -> json_result
  #   TypeScript callback: imports src/commands/, executes, returns structured JSON
  #
  # ========================================

  Background: User Story
    As a AI agent using codelet tools
    I want to call fspec commands via NAPI callback pattern
    So that I can execute fspec commands directly from Rust without process spawning or complex JavaScript execution

  Scenario: Implement FspecTool struct in codelet/tools/src/fspec.rs
    Given I need to create a new FspecTool for NAPI integration
    And I have existing tool patterns like BashTool and GrepTool
    When I implement FspecTool as a simple unit struct
    And I follow the pattern 'pub struct FspecTool;' without derive macros
    And I add it to codelet/tools/src/lib.rs exports
    Then FspecTool should be available for NAPI binding
    And it should follow existing tool implementation patterns

  Scenario: Expose callFspecCommand via NAPI callback pattern  
    Given FspecTool is implemented in codelet/tools/src/fspec.rs
    And I have the NAPI binding infrastructure in codelet/napi/src/lib.rs
    When I create callFspecCommand NAPI function with callback pattern
    And I use signature: callFspecCommand(command: String, args: String, project_root: String, callback: Function)
    Then Rust agents should be able to call fspec commands via callbacks
    And TypeScript definitions should be generated automatically
    And the pattern should avoid complex JavaScript execution

  Scenario: Execute ACDD workflow commands via callback
    Given callFspecCommand NAPI function is available
    And TypeScript callback can import src/commands/ modules
    When Rust agent calls callFspecCommand('list-work-units', '{}', '/project', callback)
    And callback imports listWorkUnits from src/commands/list-work-units
    And callback executes the command and returns JSON
    Then agent receives JSON with work units data and system reminders
    And the response includes structured success/error information
    And system reminders guide ACDD workflow progression

  Scenario: Handle command execution errors gracefully
    Given callFspecCommand is available with error handling
    When Rust agent calls callFspecCommand with invalid command name
    And callback cannot find the requested command module
    Then agent receives JSON error response with clear message
    And error includes type, suggestions, and context information
    And system does not crash or throw unhandled exceptions
    And agent can adapt behavior based on error details

  Scenario: Support all ACDD workflow commands
    Given callFspecCommand supports comprehensive command execution
    When Rust agent calls commands for work units (create-story, show-work-unit, update-work-unit-status)
    And agent calls example mapping commands (add-rule, add-example, answer-question) 
    And agent calls feature commands (create-feature, generate-scenarios, validate)
    And agent calls board and dependency management commands
    Then all ACDD workflow commands execute successfully
    And each returns appropriate JSON responses with system reminders
    And setup commands like bootstrap and init are not included