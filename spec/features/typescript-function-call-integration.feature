@feature-management
@cli
@integration
@CODE-005
Feature: TypeScript Function Call Integration

  """
  Architecture notes:
  - Replace TODO in Tool::call() method with callFspecCommand NAPI integration
  - Connects FspecTool struct (CODE-004) with NAPI bridge (CODE-003) via TypeScript callback pattern
  - TypeScript callback dynamically imports src/commands/ modules like listWorkUnits(), createStory()
  - System reminders parsed from console.error() output and included in tool response for LLM workflow orchestration
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool::call() method must replace TODO placeholder by calling callFspecCommand NAPI function with a TypeScript callback
  #   2. Support ALL fspec commands EXCEPT bootstrap and init commands via direct TypeScript function calls
  #   3. Parse and include system reminder tags from console/stderr output in tool response for LLM workflow orchestration
  #   4. TypeScript callback must dynamically import and call fspec command modules from src/commands/ directory
  #   5. Convert TypeScript function errors to ToolError for consistency while preserving original fspec error messages and context
  #
  # EXAMPLES:
  #   1. Agent calls FspecTool.call(FspecArgs{command:'list-work-units', args:'{}', project_root:'.'}) and receives JSON with work units data plus system reminder
  #   2. Agent calls FspecTool.call() for 'create-story' command and TypeScript callback dynamically imports createStory() function and returns JSON result
  #   3. Agent calls FspecTool.call() for 'create-story' command with story details and receives JSON with new work unit ID plus system reminder about Example Mapping next steps
  #   4. Agent calls FspecTool.call() for unsupported 'bootstrap' command and receives ToolError with clear error message about using CLI directly for setup commands
  #   5. Agent executes multiple fspec commands rapidly (list-work-units, show-work-unit, update-work-unit-status) without 100-500ms CLI process spawning delays between calls
  #
  # QUESTIONS (ANSWERED):
  #   Q: What specific fspec commands need to be supported through the direct TypeScript integration? Should we support all commands or just the most commonly used ones like list-work-units, create-story, update-work-unit-status?
  #   A: FspecTool should support ALL fspec commands for complete ACDD workflow functionality EXCEPT bootstrap and init commands. Bootstrap is for other AI agents and isn't needed since codelet has fspec context in the system prompt. Init is for project setup which happens outside the FspecTool context.
  #
  #   Q: How should the Tool::call() method map fspec command names (like 'list-work-units') to TypeScript function imports? Should it use dynamic imports or a pre-built mapping?
  #   A: The architecture is already established: Tool::call() should replace the TODO placeholder by creating a TypeScript callback that dynamically imports fspec command modules from src/commands/ and calls the appropriate function (like listWorkUnits(), createStory(), etc.). The callback gets passed to callFspecCommand via the existing NAPI infrastructure.
  #
  #   Q: The fspec commands output system reminders to stderr via console.error() - how should these be captured and returned to the agent when called via NAPI? Should they be included in the tool response JSON?
  #   A: FspecTool should parse <system-reminder> tags from console/stderr output during TypeScript function execution and include them in the tool response. These provide timely workflow orchestration context that tells the LLM what needs to be done next, chaining the entire ACDD process together. The system reminders implement an anti-drift pattern that makes fspec more than just a CLI tool - it becomes an AI agent workflow orchestrator.
  #
  #   Q: The callFspecCommand NAPI function expects a callback that takes (command, args_json, project_root) and returns a string. Should this callback be implemented as part of the Rust Tool::call() method, or should it be a separate TypeScript function that imports and executes the fspec commands?
  #   A: The Tool::call() method should provide its own callback function that imports and executes the real fspec command modules from src/commands/. This avoids needing external callbacks. The Rust code creates the callback and passes it to callFspecCommand.
  #
  #   Q: For performance optimization, should we pre-import all fspec command modules when FspecTool is initialized, or dynamically import them on each call? What about caching project state between calls?
  #   A: No specific performance measurements required. The goal is simply to eliminate CLI process spawning overhead completely. As long as FspecTool calls TypeScript functions directly via NAPI without spawning processes, the performance improvement is sufficient. No complex caching strategies needed.
  #
  # ========================================

  Background: User Story
    As a Rust agent using FspecTool
    I want to execute fspec commands directly via TypeScript functions
    So that eliminate CLI process spawning overhead and get 10-100x performance improvement

  Scenario: Execute list-work-units command via direct TypeScript function call
    Given I have FspecTool with Tool trait implementation connected to NAPI
    And callFspecCommand NAPI function is available with TypeScript callback
    When I call FspecTool.call(FspecArgs{command:'list-work-units', args:'{}', project_root:'.'})
    Then I receive JSON response with work units data
    And the response includes system reminder with workflow orchestration guidance
    And no CLI process spawning occurs during command execution

  Scenario: Execute create-story command with system reminder workflow guidance
    Given I have FspecTool connected to NAPI infrastructure 
    And TypeScript callback can dynamically import fspec command modules
    When I call FspecTool.call() for 'create-story' command with story details
    Then TypeScript callback imports createStory() function from src/commands/
    And I receive JSON response with new work unit ID
    And the response includes system reminder about Example Mapping next steps
    And the command executes without CLI process spawning

  Scenario: Handle unsupported bootstrap command with clear error
    Given I have FspecTool with command validation rules
    And bootstrap command is excluded from NAPI integration
    When I call FspecTool.call() for unsupported 'bootstrap' command
    Then I receive ToolError with clear error message
    And the error message suggests using CLI directly for setup commands
    And the error maintains consistent ToolError format with other codelet tools

  Scenario: Execute multiple commands rapidly without CLI spawning delays
    Given I have FspecTool optimized for performance via NAPI
    And I need to execute a sequence of related fspec commands
    When I execute multiple commands (list-work-units, show-work-unit, update-work-unit-status) rapidly
    Then each command executes via direct TypeScript function calls
    And no 100-500ms CLI process spawning delays occur between calls
    And total execution time is significantly faster than CLI approach
    And each command returns proper JSON responses with system reminders

  Scenario: Handle TypeScript function errors with consistent error format
    Given I have FspecTool with error handling integrated
    And a fspec command execution fails in TypeScript
    When TypeScript function throws an error during command execution
    Then the error is converted to ToolError for consistency with other tools
    And original fspec domain error messages and context are preserved
    And the agent receives structured error information for proper handling
