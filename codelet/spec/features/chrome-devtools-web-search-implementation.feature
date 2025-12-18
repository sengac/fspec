@chrome
@web-search
@tools
@WEB-002
Feature: Chrome DevTools Web Search Implementation

  """
  Uses rust-headless-chrome crate to control Chrome/Chromium via DevTools Protocol. Supports three connection modes: connect to existing Chrome (Browser::connect), launch with custom path (LaunchOptions::path), or auto-detect installed browser. Content extraction uses injected JavaScript with Readability-like algorithm to filter navigation, ads, and non-content elements. Search scrapes DuckDuckGo HTML for actual results. Browser is lazily initialized on first web search request.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must use rust-headless-chrome crate WITHOUT the 'fetch' feature to avoid auto-downloading Chrome
  #   2. Must support connecting to existing Chrome via Browser::connect(ws_url) for users with Chrome already running
  #   3. Must support auto-detecting locally installed Chrome/Chromium via LaunchOptions::path
  #   4. Content extraction must filter out nav, header, footer, sidebar, ads, and script/style elements
  #   5. Search must return actual result URLs with titles and snippets, not just instant answers
  #   6. Browser must be lazily initialized - don't spawn Chrome until first web search request
  #
  # EXAMPLES:
  #   1. Agent searches for 'rust async programming' and gets 10 results with titles, URLs, and snippets from DuckDuckGo HTML
  #   2. Agent opens a React SPA page and gets the rendered content (not the empty HTML shell)
  #   3. Agent opens a news article and gets only the article content, not navigation/sidebar/footer
  #   4. Agent finds pattern 'installation' in documentation page and gets 5 matches with surrounding context
  #   5. Agent connects to existing Chrome on ws://127.0.0.1:9222 instead of launching new browser
  #   6. Agent uses locally installed Chrome at /Applications/Google Chrome.app without downloading anything
  #
  # ========================================

  Background: User Story
    As a AI agent using codelet
    I want to perform web searches and fetch page content with full JavaScript rendering
    So that I can access modern web content including SPAs, dynamic pages, and get clean extracted content without navigation/ads

  Scenario: Agent performs web search and gets results with URLs
    Given the Chrome browser is available
    And the web search tool is configured
    When the agent executes a search for "rust async programming"
    Then the agent receives at least 5 search results
    And each result contains a title, URL, and snippet
    And the results are from DuckDuckGo HTML scraping

  Scenario: Agent opens JavaScript-rendered SPA page
    Given the Chrome browser is available
    And the web search tool is configured
    When the agent opens a React SPA page
    Then the agent receives the fully rendered content
    And the content is not an empty HTML shell
    And JavaScript-generated content is included

  Scenario: Agent extracts clean article content
    Given the Chrome browser is available
    And the web search tool is configured
    When the agent opens a news article page
    Then the agent receives only the article content
    And navigation elements are filtered out
    And sidebar content is filtered out
    And footer content is filtered out
    And advertisement content is filtered out

  Scenario: Agent finds pattern in page with context
    Given the Chrome browser is available
    And the web search tool is configured
    When the agent searches for pattern "installation" in a documentation page
    Then the agent receives matching content with surrounding context
    And the search is case-insensitive
    And multiple matches are returned if present

  Scenario: Agent connects to existing Chrome instance
    Given Chrome is running with remote debugging on port 9222
    And the CODELET_CHROME_WS_URL environment variable is set
    When the agent performs a web search
    Then the agent connects to the existing Chrome instance
    And no new Chrome process is spawned

  Scenario: Agent uses locally installed Chrome
    Given Chrome is installed at a standard system location
    And the CODELET_CHROME_PATH environment variable is not set
    When the agent performs a web search
    Then the agent auto-detects the installed Chrome
    And no Chrome binary is downloaded
    And the local Chrome installation is used

  Scenario: Browser is lazily initialized
    Given the web search tool is loaded
    When no web search has been performed yet
    Then no Chrome process is running
    When the agent performs the first web search
    Then Chrome is initialized on demand
    And subsequent searches reuse the same browser instance
