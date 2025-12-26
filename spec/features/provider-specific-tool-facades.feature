@provider-abstraction
@facade-pattern
@tools
@TOOL-001
Feature: Provider-Specific Tool Facades
  """
  Three-layer architecture: Provider Facades (Claude/Gemini/OpenAI) → Tool Adapter Layer (maps params) → Base Tool Implementation. Uses ToolFacade trait with provider(), tool_name(), definition(), map_params() methods. ProviderToolRegistry manages facade lookup by provider name. Gemini requires flat schemas (no oneOf), Claude supports complex nested schemas.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each tool facade MUST implement the ToolFacade trait with provider(), tool_name(), definition(), and map_params() methods
  #   2. Provider-specific parameters MUST be mapped to internal parameters through the facade before tool execution
  #   3. A single internal tool MAY have multiple facades for a provider (e.g., Gemini has google_web_search AND web_fetch for our web_search)
  #   4. Tool definitions MUST use JSON Schema format that the target provider understands (no oneOf for Gemini)
  #   5. Tool definitions sent to the provider API MUST use the provider's expected schema format, tool names, and parameter structures (e.g., Gemini gets 'google_web_search' with flat {query} schema, not 'web_search' with nested action objects)
  #   6. Base tool implementations MUST remain unchanged - only facades adapt the interface
  #   7. ProviderToolRegistry MUST return only facades registered for the requested provider
  #
  # EXAMPLES:
  #   1. Claude web search: provider sends {action: {type: 'search', query: 'rust async'}} → facade maps to InternalParams::Search{query: 'rust async'} → base tool executes search
  #   2. Gemini web search: provider sends {query: 'rust async'} to 'google_web_search' tool → facade maps to InternalParams::Search{query: 'rust async'} → same base tool executes
  #   3. Gemini URL fetch: provider sends {prompt: 'https://example.com summarize'} to 'web_fetch' tool → facade extracts URL and maps to InternalParams::OpenPage{url: 'https://example.com'}
  #   4. Registry lookup: tools_for_provider('gemini') returns [GeminiGoogleWebSearchFacade, GeminiWebFetchFacade, GeminiReadFileFacade, ...] but NOT Claude facades
  #   5. Tool definition for Gemini: google_web_search sends schema {type: 'object', properties: {query: {type: 'string'}}, required: ['query']} - flat structure, no oneOf
  #   6. Tool definition for Claude: web_search sends schema with oneOf containing search/open_page/find_in_page action variants - Claude handles complex schemas well
  #
  # ========================================
  Background: User Story
    As a developer building multi-provider LLM agents
    I want to use a consistent internal tool interface while each provider receives tool schemas in their native format
    So that I can add new providers without duplicating tool logic and each provider gets optimal tool definitions

  Scenario: Map Claude web search parameters to internal format
    Given a ClaudeWebSearchFacade is registered
    When Claude sends parameters {action: {type: 'search', query: 'rust async'}}
    Then the facade maps to InternalParams::Search with query 'rust async'
    And the base web search tool executes with the mapped parameters

  Scenario: Map Gemini google_web_search parameters to internal format
    Given a GeminiGoogleWebSearchFacade is registered
    When Gemini sends parameters {query: 'rust async'} to tool 'google_web_search'
    Then the facade maps to InternalParams::Search with query 'rust async'
    And the same base web search tool executes as with Claude

  Scenario: Map Gemini web_fetch URL to internal open_page format
    Given a GeminiWebFetchFacade is registered
    When Gemini sends parameters {prompt: 'https://example.com summarize this'} to tool 'web_fetch'
    Then the facade extracts the URL and maps to InternalParams::OpenPage with url 'https://example.com'
    And the base web search tool executes the open_page action

  Scenario: Registry returns only facades for requested provider
    Given facades are registered for both Claude and Gemini providers
    When I request tools_for_provider('gemini')
    Then the registry returns GeminiGoogleWebSearchFacade and GeminiWebFetchFacade
    And the registry does not return any Claude facades

  Scenario: Gemini facade provides flat JSON schema without oneOf
    Given a GeminiGoogleWebSearchFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing only {query: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  Scenario: Claude facade provides complex schema with action variants
    Given a ClaudeWebSearchFacade is created
    When I request the tool definition
    Then the schema contains an 'action' property with 'oneOf' variants
    And the variants include search, open_page, and find_in_page action types
