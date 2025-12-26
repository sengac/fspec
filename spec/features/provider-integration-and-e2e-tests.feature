@TOOL-007 @high @tools @facade-pattern @provider-abstraction @e2e
Feature: Provider Integration and E2E Tests

  """
  ClaudeProvider needs FacadeToolWrapper(ClaudeWebSearchFacade) in create_rig_agent(). GeminiProvider already uses all facades. E2E tests in codelet/tests/*_facade_e2e_test.rs with #[ignore] for CI. Claude supports oneOf schemas so only WebSearch needs facade; other tools use raw implementations.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ClaudeProvider.create_rig_agent() MUST use FacadeToolWrapper with ClaudeWebSearchFacade instead of raw WebSearchTool
  #   2. GeminiProvider MUST continue using all existing facade wrappers (verified by E2E test)
  #   3. E2E tests MUST verify facade tool definitions are correctly sent to provider APIs
  #   4. E2E tests MUST be marked with #[ignore] attribute and require real API keys to run
  #
  # EXAMPLES:
  #   1. ClaudeProvider.create_rig_agent() creates FacadeToolWrapper with ClaudeWebSearchFacade → agent receives tool 'web_search' with flat schema {action_type: enum, query, url, pattern}
  #   2. GeminiProvider.create_rig_agent() includes LsToolFacadeWrapper → agent receives tool 'list_directory' with flat schema {path: string}
  #   3. E2E test calls gemini_provider.create_rig_agent() and verifies 10 tools registered: read_file, write_file, replace, run_shell_command, search_file_content, find_files, list_directory, ast_grep, google_web_search, web_fetch
  #
  # ========================================

  Background: User Story
    As a developer integrating LLM providers
    I want to have all providers use the facade pattern consistently and verify with E2E tests
    So that providers have consistent tool interfaces and I can confidently deploy knowing tools work with real APIs

  # Example 1: Claude uses web search facade
  Scenario: ClaudeProvider uses ClaudeWebSearchFacade for web search tool
    Given a ClaudeProvider with valid API credentials
    When I call create_rig_agent()
    Then the agent includes FacadeToolWrapper wrapping ClaudeWebSearchFacade
    And the web_search tool has flat schema with action_type enum containing search, open_page, and find_in_page

  # Example 2: Gemini uses all facade wrappers
  Scenario: GeminiProvider includes all facade-wrapped tools
    Given a GeminiProvider with valid API credentials
    When I call create_rig_agent()
    Then the agent includes LsToolFacadeWrapper with tool name 'list_directory'
    And the list_directory tool has flat schema with path parameter
    And all other facade wrappers are registered

  # Example 3: E2E test verifies tool registration
  @e2e @integration
  Scenario: E2E test verifies all Gemini tools are registered with correct schemas
    Given a configured GeminiProvider with real API key
    When I create a rig agent and inspect tool definitions
    Then I find 10 tools registered with Gemini-native names
    And each tool has the expected flat schema without oneOf

  # Rule 4: E2E tests marked with ignore
  @e2e @integration
  Scenario: E2E tests are marked with ignore attribute for CI
    Given E2E test files in codelet/tests/
    When the tests are compiled
    Then tests requiring real API keys have #[ignore] attribute
    And tests can be run explicitly with --ignored flag
