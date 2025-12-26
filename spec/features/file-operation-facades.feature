@TOOL-003 @high @tools @facade @gemini
Feature: File Operation Facades

  """
  Create GeminiReadFileFacade, GeminiWriteFileFacade, GeminiReplaceFacade in codelet/tools/src/facade/file_ops.rs. Each implements ToolFacade trait. Add InternalFileParams enum to traits.rs. Wrap with FacadeToolWrapper and add to GeminiProvider.create_rig_agent(). Follow pattern from web_search.rs facades.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GeminiReadFileFacade MUST use tool name 'read_file' and map to internal ReadTool
  #   2. GeminiWriteFileFacade MUST use tool name 'write_file' and map to internal WriteTool
  #   3. GeminiReplaceFacade MUST use tool name 'replace' and map to internal EditTool
  #   4. All file operation facades MUST use flat JSON schemas without nested objects
  #   5. All file operation facades MUST be wrapped with FileToolFacadeWrapper and added to GeminiProvider.create_rig_agent()
  #
  # EXAMPLES:
  #   1. Gemini sends {path: '/tmp/file.txt'} to read_file → facade maps to ReadArgs{file_path: '/tmp/file.txt'} → ReadTool executes
  #   2. Gemini sends {path: '/tmp/file.txt', content: 'hello'} to write_file → facade maps to WriteArgs → WriteTool executes
  #   3. Gemini sends {path: '/tmp/file.txt', old_text: 'foo', new_text: 'bar'} to replace → facade maps to EditArgs → EditTool executes
  #   4. GeminiProvider.create_rig_agent() adds read_file, write_file, replace tools using FileToolFacadeWrapper instead of raw ReadTool, WriteTool, EditTool
  #
  # ========================================

  Background: User Story
    As a LLM agent using Gemini provider
    I want to use file operations with Gemini-native tool names and flat schemas
    So that Gemini understands my tool calls without schema compatibility issues

  # Example 1: Gemini sends {path: '/tmp/file.txt'} to read_file → facade maps to ReadArgs
  Scenario: Map Gemini read_file parameters to internal ReadTool format
    Given a GeminiReadFileFacade is registered
    When Gemini sends parameters {path: '/tmp/file.txt'} to tool 'read_file'
    Then the facade maps to InternalFileParams::Read with file_path '/tmp/file.txt'
    And the same base ReadTool executes with the mapped parameters

  # Example 2: Gemini sends {path: '/tmp/file.txt', content: 'hello'} to write_file → facade maps to WriteArgs
  Scenario: Map Gemini write_file parameters to internal WriteTool format
    Given a GeminiWriteFileFacade is registered
    When Gemini sends parameters {path: '/tmp/file.txt', content: 'hello'} to tool 'write_file'
    Then the facade maps to InternalFileParams::Write with file_path '/tmp/file.txt' and content 'hello'
    And the same base WriteTool executes with the mapped parameters

  # Example 3: Gemini sends {path: '/tmp/file.txt', old_text: 'foo', new_text: 'bar'} to replace → facade maps to EditArgs
  Scenario: Map Gemini replace parameters to internal EditTool format
    Given a GeminiReplaceFacade is registered
    When Gemini sends parameters {path: '/tmp/file.txt', old_text: 'foo', new_text: 'bar'} to tool 'replace'
    Then the facade maps to InternalFileParams::Edit with file_path '/tmp/file.txt', old_string 'foo', new_string 'bar'
    And the same base EditTool executes with the mapped parameters

  # Example 4: GeminiProvider.create_rig_agent() adds file operation tools using FacadeToolWrapper
  Scenario: GeminiProvider registers file operation facades with rig agent builder
    Given a GeminiProvider is configured
    When create_rig_agent() is called
    Then the agent has tool 'read_file' backed by GeminiReadFileFacade
    And the agent has tool 'write_file' backed by GeminiWriteFileFacade
    And the agent has tool 'replace' backed by GeminiReplaceFacade
    And all file operation tools use FileToolFacadeWrapper instead of raw tools

  # Rule 4: All file operation facades MUST use flat JSON schemas
  Scenario: GeminiReadFileFacade provides flat JSON schema without nested objects
    Given a GeminiReadFileFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing only {path: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  Scenario: GeminiWriteFileFacade provides flat JSON schema without nested objects
    Given a GeminiWriteFileFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing {path: {type: 'string'}, content: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  Scenario: GeminiReplaceFacade provides flat JSON schema without nested objects
    Given a GeminiReplaceFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing {path: {type: 'string'}, old_text: {type: 'string'}, new_text: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  # Rule 5: FileToolFacadeWrapper integration with rig::tool::Tool trait
  Scenario: FileToolFacadeWrapper for read_file integrates with rig Tool trait
    Given a FileToolFacadeWrapper wrapping GeminiReadFileFacade
    When I call name() on the wrapper
    Then it returns "read_file"
    And when I call definition() it returns a flat schema with path parameter