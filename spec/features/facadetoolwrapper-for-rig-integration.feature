@TOOL-002 @high @tools @facade @rig
Feature: FacadeToolWrapper for Rig Integration

  """
  FacadeToolWrapper in codelet/tools/src/facade/wrapper.rs implements rig::tool::Tool trait. Overrides name() and definition() to return facade-specific values. call() maps params via facade then executes base tool.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. FacadeToolWrapper MUST implement rig::tool::Tool trait
  #   2. FacadeToolWrapper MUST delegate name() to facade.tool_name()
  #   3. FacadeToolWrapper MUST delegate definition() to facade.definition()
  #
  # EXAMPLES:
  #   1. Wrapping GeminiGoogleWebSearchFacade returns tool name 'google_web_search'
  #   2. Wrapping GeminiWebFetchFacade returns tool name 'web_fetch'
  #
  # ========================================

  Background: User Story
    As a developer using the facade system
    I want to use FacadeToolWrapper to integrate facades with rig's agent builder
    So that facades can be added to agents using .tool() method

  # Rule 1: FacadeToolWrapper MUST implement rig::tool::Tool trait
  # Rule 2: FacadeToolWrapper MUST delegate name() to facade.tool_name()
  Scenario: FacadeToolWrapper returns facade tool name for web search
    Given a FacadeToolWrapper wrapping GeminiGoogleWebSearchFacade
    When I call name() on the wrapper
    Then it returns "google_web_search"

  # Example 2: Wrapping GeminiWebFetchFacade returns tool name 'web_fetch'
  Scenario: FacadeToolWrapper returns facade tool name for web fetch
    Given a FacadeToolWrapper wrapping GeminiWebFetchFacade
    When I call name() on the wrapper
    Then it returns "web_fetch"

  # Rule 3: FacadeToolWrapper MUST delegate definition() to facade.definition()
  Scenario: FacadeToolWrapper returns facade definition with flat schema
    Given a FacadeToolWrapper wrapping GeminiGoogleWebSearchFacade
    When I call definition() on the wrapper
    Then it returns a flat schema without oneOf
    And the schema has name "google_web_search"
