@screenshot
@web-tools
@browser
@BROWSE-002
Feature: Full-page scrollable screenshot capture

  """
  
  Integration Points:
  - WebSearchAction enum (codelet_common/src/web_search.rs): Add CaptureScreenshot variant with url, output_path, full_page fields
  - WebSearchTool (codelet_tools/src/web_search.rs): Handle CaptureScreenshot action, update tool description
  - ChromeBrowser (codelet_tools/src/chrome_browser.rs): Add capture_screenshot method
  
  CDP Method: Tab::capture_screenshot
  - capture_beyond_viewport: false (default) = viewport only
  - capture_beyond_viewport: true = entire scrollable page
  - format: PNG (lossless quality)
  - from_surface: true (capture rendered content)
  
  File Handling:
  - Default: Generate unique path in system temp directory (e.g., /tmp/screenshot-{uuid}.png)
  - Custom: Use output_path if provided by caller
  - Always PNG format for consistency with Read tool
  
  Return Format:
  - WebSearchResult with success=true and message containing file path
  - Path can be passed directly to Read tool for viewing
  
  Tool Description Update Required:
  - Current: 'Perform web search, open web pages, or find content within pages...'
  - Updated: Add 'capture screenshots' to the description
  
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Screenshot action must be added to the existing WebSearchAction enum as CaptureScreenshot variant
  #   2. Screenshots must be saved to a file path and return the path so the AI can use Read tool to view
  #   3. Full-page screenshots require setting capture_beyond_viewport: true in the CDP call
  #   4. Screenshots default to PNG format for lossless quality
  #   5. Default save location is temp directory if no output_path specified
  #
  # EXAMPLES:
  #   1. AI uses web_search tool with action: {type: capture_screenshot, url: 'https://example.com'}, gets back file path /tmp/screenshot-abc123.png
  #   2. AI captures full-page screenshot with action: {type: capture_screenshot, url: 'https://example.com', full_page: true}, captures entire scrollable content
  #   3. AI captures screenshot to specific path with action: {type: capture_screenshot, url: 'https://example.com', output_path: '/tmp/my-screenshot.png'}
  #   4. AI captures screenshot then uses Read tool on the returned path to view the image
  #   (Example 5 removed: JavaScript content waiting is implicit in all screenshot scenarios via ChromeBrowser.navigate_and_wait)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the screenshot action return the image data directly (like Read tool does for images) or save to a file and return the path? The Read tool approach would be more direct but potentially consume more tokens for large screenshots.
  #   A: Option A: Save to file and return path. This is better for token efficiency (full-page screenshots can be huge), reuses existing Read tool multimodal support, simpler implementation, and matches industry patterns like Playwright MCP.
  #
  # ========================================

  Background: User Story
    As a AI agent using codelet web tools
    I want to capture screenshots of web pages and save them to files
    So that I can visually analyze web content using the Read tool's multimodal support

  Scenario: Capture viewport screenshot with default settings
    Given a web page at "https://example.com"
    When I capture a screenshot with action type "capture_screenshot" and url "https://example.com"
    Then the tool should return a file path to a PNG screenshot
    And the screenshot file should exist at the returned path
    And the screenshot should capture the visible viewport


  Scenario: Capture full-page screenshot of scrollable content
    Given a web page with scrollable content taller than the viewport
    When I capture a screenshot with url "https://example.com" and full_page set to true
    Then the screenshot should capture the entire scrollable page content
    And the screenshot height should exceed the viewport height


  Scenario: Capture screenshot to custom output path
    Given a web page at "https://example.com"
    When I capture a screenshot with url "https://example.com" and output_path "/tmp/my-custom-screenshot.png"
    Then the screenshot should be saved to "/tmp/my-custom-screenshot.png"
    And the returned path should match the specified output_path


  Scenario: View captured screenshot using Read tool
    Given I have captured a screenshot that returned path "/tmp/screenshot.png"
    When I use the Read tool with file_path "/tmp/screenshot.png"
    Then the Read tool should return image data with type "image"
    And the media_type should be "image/png"

