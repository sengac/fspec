@tools-development
@codelet
@tools
@CODE-004
Feature: Core FspecTool Implementation
  """
  FspecArgs struct contains command, args, and project_root to match callFspecCommand NAPI signature
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. FspecTool must implement rig::tool::Tool trait with NAME, Error, Args, and Output associated types
  #   2. FspecArgs struct must derive Debug, Deserialize, Serialize, and JsonSchema traits for tool parameter validation
  #   3. Tool trait requires async definition() method that returns ToolDefinition and async call() method that executes the command
  #   4. Existing execute_via_callback functionality must be preserved and used by the Tool trait implementation
  #
  # EXAMPLES:
  #   1. Developer can use FspecTool.call(FspecArgs { command: 'list-work-units', args: '{}', project_root: '.' }) alongside BashTool.call() in the same agent session
  #   2. When agent system initializes tools, FspecTool.definition() returns name 'Fspec' and description explaining fspec command capabilities
  #   3. When fspec command fails, developer receives consistent ToolError format like other tools instead of custom error formats
  #
  # QUESTIONS (ANSWERED):
  #   Q: What specific fspec commands need to be supported through the Tool interface?
  #   A: The Tool trait call() method should use the NAPI callFspecCommand function, providing its own callback that maps command names to the real imported fspec command functions from src/commands/
  #
  #   Q: Should FspecTool support all fspec commands or just a subset needed for AI workflow orchestration?
  #   A: FspecTool should support ALL fspec commands for complete ACDD workflow functionality EXCEPT bootstrap and init commands. Bootstrap is for other AI agents and isn't needed since codelet has fspec context in the system prompt. Init is for project setup which happens outside the FspecTool context.
  #
  #   Q: How should the Tool handle errors from fspec commands - return ToolError or pass through fspec's error format?
  #   A: FspecTool should parse <system-reminder> tags from console/stderr output during TypeScript function execution and include them in the tool response. These provide timely workflow orchestration context that tells the LLM what needs to be done next, chaining the entire ACDD process together.
  #
  #   Q: How should FspecTool call fspec TypeScript functions directly? Should it import and call them or use a different mechanism?
  #   A: Convert TypeScript function errors to ToolError for consistency with other codelet tools, but MUST preserve the original fspec domain error messages and context within the ToolError. This maintains uniform error handling while keeping fspec's rich error information.
  #
  #   Q: How exactly should the Rust FspecTool call TypeScript functions? Should it use NAPI to import and execute the actual TypeScript modules, or call them via some other mechanism?
  #   A: No specific performance measurements required. The goal is simply to eliminate CLI process spawning overhead completely. As long as FspecTool calls TypeScript functions directly via NAPI without spawning processes, the performance improvement is sufficient.
  #
  #   Q: Which specific fspec TypeScript functions need to be called? Should this integrate with the existing commands in src/commands/ like list-work-units.ts, create-story.ts, etc.?
  #   A: Yes, FspecTool should follow exactly the same architectural patterns as other tools in codelet/tools/src/ for uniformity. Use the same Tool trait implementation pattern, error handling, and code organization as WebSearchTool, BashTool, etc.
  #
  #   Q: How are system reminders captured from TypeScript function calls? Do the TypeScript functions return them directly, or do they need to be extracted from stderr/output like in CLI?
  #   A: Use NAPI to directly import the TypeScript modules and call the exported functions. Everything should be as direct as possible - no wrapper layers or adapters. FspecTool should directly import and call listWorkUnits(), createStory(), etc. from src/commands/ modules.
  #
  #   Q: How should errors from TypeScript function calls be handled? Should they be converted to ToolError, or passed through in their original format?
  #   A: TypeScript function errors should be converted to ToolError for consistency with other codelet tools. However, structured error responses (like validation failures) should preserve their original message format while wrapping them in ToolError::Execution.
  #
  #   Q: What is the exact technical architecture for how FspecTool should call TypeScript functions? Should it use the existing NAPI bindings from CODE-003, or a different mechanism?
  #   A: Use the existing NAPI bridge architecture: FspecTool in Rust should call actual TypeScript functions from src/commands/ directly via the NAPI bindings that were set up in CODE-003. Replace the simulation functions with real function imports and calls.
  #
  #   Q: Which specific fspec commands must be supported through FspecTool? Should it support ALL fspec commands, or just a subset needed for AI workflow orchestration (like work unit management, scenario generation, etc.)?
  #   A: FspecTool should support ALL fspec commands for complete ACDD workflow functionality EXCEPT bootstrap and init commands. Bootstrap is for other AI agents and isn't needed since codelet has fspec context in the system prompt. Init is for project setup which happens outside the FspecTool context.
  #
  #   Q: How exactly should system reminders be captured and returned? The TypeScript functions output them to console/stderr wrapped in <system-reminder> tags - should FspecTool parse these during execution and include them in the response?
  #   A: FspecTool should parse <system-reminder> tags from console/stderr output during TypeScript function execution and include them in the tool response. These provide timely workflow orchestration context that tells the LLM what needs to be done next, chaining the entire ACDD process together.
  #
  #   Q: How should errors from TypeScript function calls be handled? Should they be converted to ToolError for consistency with other codelet tools, or returned in the original TypeScript error format?
  #   A: Convert TypeScript function errors to ToolError for consistency with other codelet tools, but MUST preserve the original fspec domain error messages and context within the ToolError. This maintains uniform error handling while keeping fspec's rich error information.
  #
  #   Q: What are the specific performance requirements? You mentioned eliminating 100-500ms CLI process spawning delays - what should the target execution time be for individual fspec commands?
  #   A: No specific performance measurements required. The goal is simply to eliminate CLI process spawning overhead completely. As long as FspecTool calls TypeScript functions directly via NAPI without spawning processes, the performance improvement is sufficient.
  #
  #   Q: How should FspecTool integrate with the existing codelet tools architecture? Should it follow the same patterns as other tools in codelet/tools/src/ (like WebSearchTool, etc.)?
  #   A: Yes, FspecTool should follow exactly the same architectural patterns as other tools in codelet/tools/src/ for uniformity. Use the same Tool trait implementation pattern, error handling, and code organization as WebSearchTool, BashTool, etc.
  #
  #   Q: The TypeScript commands export functions like 'listWorkUnits()' - how should the Rust FspecTool import and call these functions? Should it use NAPI to directly import the TypeScript modules, or call wrapper functions in the NAPI bridge?
  #   A: Use NAPI to directly import the TypeScript modules and call the exported functions. Everything should be as direct as possible - no wrapper layers or adapters. FspecTool should directly import and call listWorkUnits(), createStory(), etc. from src/commands/ modules.
  #
  #   Q: What specific methods and types does the rig::tool::Tool trait require that are not already implemented in FspecTool?
  #   A: The Tool trait requires: const NAME, type Error/Args/Output, async fn definition() returning ToolDefinition, and async fn call() taking Args and returning Result. The current FspecTool has none of these - only execute_via_callback.
  #
  #   Q: The current FspecTool already has execute_via_callback - should the Tool trait call() method use this existing functionality, or does it need something different?
  #   A: The Tool trait call() method should use callFspecCommand from NAPI, providing its own callback function that imports and executes the real fspec command modules from src/commands/. This avoids needing external callbacks.
  #
  #   Q: Looking at other tools like BashTool, they have Args structs (like BashArgs). What should FspecArgs contain - just command, or also args and projectRoot parameters?
  #   A: FspecArgs should contain all three fields: command (String), args (String), and project_root (String) to match the callFspecCommand NAPI signature and give users full control over structured fspec command execution
  #
  #   Q: Does FspecTool need to be integrated into the NAPI bindings so it's available to TypeScript, or is this just for Rust-side agent usage?
  #   A: FspecTool should be Rust-only for agent usage. TypeScript already has direct access to fspec functions via imports and doesn't need the Tool trait interface. The Tool trait is specifically for Rust agents to eliminate CLI spawning.
  #
  # ========================================
  Background: User Story
    As a developer using codelet agent system
    I want to use FspecTool alongside other codelet tools (BashTool, ReadTool, WriteTool)
    So that I can execute fspec commands seamlessly in the same agent session

  Scenario: Developer uses FspecTool alongside other codelet tools in same session
    Given I have a codelet agent system with multiple tools available
    And FspecTool implements rig::tool::Tool trait alongside BashTool and ReadTool
    When I use FspecTool.call() to execute fspec commands
    And I also use BashTool.call() and ReadTool.call() in the same session
    Then all tools work seamlessly together without conflicts
    And FspecTool eliminates CLI process spawning for fspec commands
    And I can execute fspec commands like list-work-units with structured arguments

  Scenario: Agent system tool initialization provides FspecTool definition
    Given I have FspecTool implementing rig::tool::Tool trait
    And the agent system is initializing available tools
    When the system calls FspecTool.definition()
    Then it returns name "Fspec" and comprehensive description
    And the description explains fspec ACDD workflow capabilities
    And tool parameters schema includes command, args, and project_root fields

  Scenario: Fspec command failures return consistent ToolError format
    Given I have FspecTool integrated with other codelet tools
    And all tools return consistent ToolError format for failures
    When I execute an fspec command that fails
    Then FspecTool returns ToolError instead of custom error formats
    And the error maintains the same structure as BashTool and ReadTool errors
    And the error preserves original fspec error context and messages
