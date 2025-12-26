@TOOL-006 @high @tools @facade-pattern @provider-abstraction
Feature: Directory Listing Facade

  """
  GeminiListDirectoryFacade in codelet/tools/src/facade/ls.rs implements LsToolFacade trait. Uses tool name 'list_directory'. Maps {path} to InternalLsParams::List. Wrapped with LsToolFacadeWrapper and replaces raw LsTool in GeminiProvider.create_rig_agent().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GeminiListDirectoryFacade MUST use tool name 'list_directory'
  #   2. GeminiListDirectoryFacade MUST use flat JSON schema with {path} parameter (no oneOf or nested action objects)
  #   3. GeminiListDirectoryFacade MUST map Gemini params to internal LsArgs format
  #   4. Directory listing facade MUST be wrapped with LsToolFacadeWrapper for rig integration
  #   5. LsToolFacadeWrapper MUST replace raw LsTool usage in GeminiProvider.create_rig_agent()
  #
  # EXAMPLES:
  #   1. Gemini sends {path: 'src'} to tool 'list_directory' → facade maps to LsArgs{path: Some('src')} → LsTool executes and returns directory listing
  #   2. Gemini sends {} (empty) to tool 'list_directory' → facade maps to LsArgs{path: None} → LsTool lists current directory
  #   3. GeminiListDirectoryFacade provides flat schema {type: 'object', properties: {path: {type: 'string'}}, required: []} - no oneOf, path is optional
  #
  # ========================================

  Background: User Story
    As a LLM agent using Gemini provider
    I want to execute directory listing operations with Gemini-native tool name and flat schema
    So that Gemini understands my tool calls without schema compatibility issues

  # Example 1: Gemini sends {path} → facade maps to LsArgs
  Scenario: Map Gemini list_directory parameters with path to internal LsArgs format
    Given a GeminiListDirectoryFacade is registered
    When Gemini sends parameters {path: 'src'} to tool 'list_directory'
    Then the facade maps to InternalLsParams::List with path 'src'
    And the base LsTool executes with the mapped parameters

  # Example 2: Gemini sends empty params → facade maps to LsArgs with None
  Scenario: Map Gemini list_directory with empty parameters to current directory
    Given a GeminiListDirectoryFacade is registered
    When Gemini sends empty parameters {} to tool 'list_directory'
    Then the facade maps to InternalLsParams::List with path None
    And the base LsTool lists the current directory

  # Example 3: Flat schema without oneOf
  Scenario: GeminiListDirectoryFacade provides flat JSON schema
    Given a GeminiListDirectoryFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing {path: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects
    And the 'path' parameter is optional (not in required array)

  # Rule 4 & 5: Wrapper integration with rig
  Scenario: LsToolFacadeWrapper integrates with rig Tool trait
    Given a LsToolFacadeWrapper wrapping GeminiListDirectoryFacade
    When I call name() on the wrapper
    Then it returns "list_directory"
    And when I call definition() it returns a flat schema with path parameter
