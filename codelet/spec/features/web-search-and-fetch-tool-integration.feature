@done
@tools
@bash-execution
@WEB-001
Feature: Web Search and Fetch Tool Integration

  """
  Copy tool registration: ToolSpec::WebSearch {} pattern from client_common.rs with native OpenAI tool type
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Web search must be implemented as native OpenAI tool type exactly like Codex
  #   2. Must copy WebSearchAction enum with Search/OpenPage/FindInPage variants exactly from Codex
  #   3. Must copy WebSearchBeginEvent and WebSearchEndEvent structures from Codex
  #   4. Must copy tool registration logic exactly - ToolSpec::WebSearch {} from client_common.rs
  #   5. Must copy configuration system - web_search_request feature flag from Codex
  #
  # EXAMPLES:
  #   1. Copy WebSearchAction enum from /tmp/codex/codex-rs/protocol/src/models.rs using astgrep
  #   2. Agent can perform web search by calling 'search for weather in Seattle' and get search results
  #   3. Agent can fetch page content by calling 'open page https://example.com' and get page content
  #   4. Agent can search within page content by calling 'find in page' with URL and pattern
  #
  # QUESTIONS (ANSWERED):
  #   Q: Do you want to implement custom web search logic (using search APIs like Bing/Google) or leverage OpenAI's native web_search tool like Codex does?
  #   A: Use OpenAI's native web_search tool exactly like Codex - copy their implementation using astgrep tool
  #
  #   Q: Should we implement just web search or also web fetch/scraping capabilities? Codex has both Search, OpenPage, and FindInPage actions.
  #   A: Copy ALL THREE web actions exactly as Codex has them: Search, OpenPage, FindInPage - 100% the same implementation
  #
  #   Q: What should be our approach for handling web content? Should we parse HTML, extract text, or return raw content?
  #   A: Copy Codex's exact approach - use their WebSearchAction enum, event handling, and tool registration exactly as they implemented it
  #
  #   Q: Should we implement all three web actions (Search, OpenPage, FindInPage) that Codex has, or start with just Search?
  #   A: Copy all three web actions exactly as Codex has them: Search, OpenPage, FindInPage
  #
  # ========================================

  Background: User Story
    As a AI agent user
    I want to use web search and content fetching capabilities
    So that I can access real-time web information during conversations

  Scenario: Agent performs web search
    Given the web search tool is available
    When the agent executes a search query "weather in Seattle"
    Then the agent receives search results
    And the web search event is logged

  Scenario: Agent fetches page content
    Given the web search tool is available
    When the agent opens a page "https://example.com"
    Then the agent receives the page content
    And the page fetch event is logged

  Scenario: Agent searches within page content
    Given the web search tool is available
    When the agent searches within a page with URL "https://example.com" and pattern "contact"
    Then the agent receives matching content from the page
    And the page search event is logged

  Scenario: Web search tool registration
    Given the codelet system is starting up
    When the web search configuration is enabled
    Then the WebSearch tool is registered as a native OpenAI tool
    And the tool appears in the available tools list
