@done
@TOOL-005 @high @tools @facade-pattern @provider-abstraction
Feature: Search Facades

  """
  GeminiSearchFileContentFacade in codelet/tools/src/facade/search.rs implements SearchToolFacade trait. Uses tool name 'search_file_content'. Maps {pattern, path} to InternalSearchParams::Grep. GeminiGlobFacade uses tool name 'find_files'. Maps {pattern, path} to InternalSearchParams::Glob. Both wrapped with SearchToolFacadeWrapper and added to GeminiProvider.create_rig_agent().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GeminiSearchFileContentFacade MUST use tool name 'search_file_content'
  #   2. GeminiSearchFileContentFacade MUST use flat JSON schema with {pattern, path} parameters
  #   3. GeminiSearchFileContentFacade MUST map Gemini params to internal GrepArgs
  #   4. GeminiGlobFacade MUST use tool name 'find_files'
  #   5. GeminiGlobFacade MUST use flat JSON schema with {pattern, path} parameters
  #   6. GeminiGlobFacade MUST map Gemini params to internal GlobArgs
  #   7. Search facades MUST be wrapped with SearchToolFacadeWrapper for rig integration
  #
  # EXAMPLES:
  #   1. Gemini sends {pattern: 'TODO', path: 'src'} to tool 'search_file_content' → facade maps to GrepArgs{pattern: 'TODO', path: Some('src')} → GrepTool executes
  #   2. GeminiSearchFileContentFacade provides flat schema {type: 'object', properties: {pattern: {type: 'string'}, path: {type: 'string'}}, required: ['pattern']} - no oneOf
  #   3. Gemini sends {pattern: '**/*.rs', path: 'src'} to tool 'find_files' → facade maps to GlobArgs{pattern: '**/*.rs', path: Some('src')} → GlobTool executes
  #   4. GeminiGlobFacade provides flat schema {type: 'object', properties: {pattern: {type: 'string'}, path: {type: 'string'}}, required: ['pattern']} - no oneOf
  #
  # ========================================

  Background: User Story
    As a LLM agent using Gemini provider
    I want to execute search operations with Gemini-native tool names and flat schemas
    So that Gemini understands my tool calls without schema compatibility issues

  # Example 1: Gemini sends {pattern, path} → facade maps to GrepArgs
  Scenario: Map Gemini search_file_content parameters to internal GrepArgs format
    Given a GeminiSearchFileContentFacade is registered
    When Gemini sends parameters {pattern: 'TODO', path: 'src'} to tool 'search_file_content'
    Then the facade maps to InternalSearchParams::Grep with pattern 'TODO' and path 'src'
    And the base GrepTool executes with the mapped parameters

  # Example 2: Flat schema without oneOf for search_file_content
  Scenario: GeminiSearchFileContentFacade provides flat JSON schema
    Given a GeminiSearchFileContentFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing {pattern: {type: 'string'}, path: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  # Example 3: Gemini sends {pattern, path} → facade maps to GlobArgs
  Scenario: Map Gemini find_files parameters to internal GlobArgs format
    Given a GeminiGlobFacade is registered
    When Gemini sends parameters {pattern: '**/*.rs', path: 'src'} to tool 'find_files'
    Then the facade maps to InternalSearchParams::Glob with pattern '**/*.rs' and path 'src'
    And the base GlobTool executes with the mapped parameters

  # Example 4: Flat schema without oneOf for find_files
  Scenario: GeminiGlobFacade provides flat JSON schema
    Given a GeminiGlobFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing {pattern: {type: 'string'}, path: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  # Rule 7: Wrapper integration with rig for search_file_content
  Scenario: SearchToolFacadeWrapper integrates with rig Tool trait for search_file_content
    Given a SearchToolFacadeWrapper wrapping GeminiSearchFileContentFacade
    When I call name() on the wrapper
    Then it returns "search_file_content"
    And when I call definition() it returns a flat schema with pattern and path parameters

  # Rule 7: Wrapper integration with rig for find_files
  Scenario: SearchToolFacadeWrapper integrates with rig Tool trait for find_files
    Given a SearchToolFacadeWrapper wrapping GeminiGlobFacade
    When I call name() on the wrapper
    Then it returns "find_files"
    And when I call definition() it returns a flat schema with pattern and path parameters
