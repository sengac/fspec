@done
@screenshot
@browser-control
@EXT-015
Feature: Element-targeted screenshots via selector or @ref
  """
  Uses client-side OffscreenCanvas crop (not Chrome's undocumented ImageDetails.rect parameter) for stability
  Reuses resolveRefSelector() for @ref→CSS+frameId, executeScript for scrollIntoView+getBoundingClientRect, drawImage crop pattern from tiling code
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When selector is omitted, browser_screenshot must behave identically to today (full viewport capture)
  #   2. When selector is provided, the element must be scrolled into view before capture
  #   3. The crop must account for device pixel ratio (CSS pixels from getBoundingClientRect × DPR = device pixels in captured PNG)
  #   4. Both CSS selectors and @ref identifiers (e.g., @e5 from browser_scan_page) must be accepted
  #   5. Elements inside iframes must be supported via existing frame-aware ref resolution
  #   6. If the element has zero visible dimensions, return an error rather than an empty image
  #   7. The cropped image must pass through the existing resize/JPEG/tile pipeline (1568px max, 80% JPEG, 800KB tile limit)
  #
  # EXAMPLES:
  #   1. Agent calls browser_screenshot() with no selector → full viewport JPEG returned (backward compatible)
  #   2. Agent calls browser_screenshot({ selector: '@e5' }) after scan → element scrolled into view, viewport captured, cropped to element rect, JPEG returned
  #   3. Agent calls browser_screenshot({ selector: '#hero-image' }) with CSS selector → element found, scrolled, captured, cropped
  #   4. Agent calls browser_screenshot({ selector: '@e3' }) but @e3 not found → error: 'Ref @e3 not found. Run browser_scan_page first'
  #   5. Agent calls browser_screenshot({ selector: '.hidden-el' }) where element has display:none (0×0 rect) → error: 'Element has no visible dimensions'
  #   6. Agent screenshots @f2e1 (element in iframe) → executeScript runs in correct frameId, captures viewport, crops to element position within viewport
  #   7. On 2x Retina display, element at CSS rect (100,200,300,150) → crop uses device pixels (200,400,600,300) from the captured PNG
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to screenshot a specific element on a page by selector or @ref
    So that I get a focused image of just the element I care about instead of the full viewport

  # Rule: When selector is omitted, browser_screenshot must behave identically to today (full viewport capture)
  Scenario: Full viewport screenshot when selector is omitted (backward compatible)
    Given the agent has an active MCP connection to the extension
    And the active tab displays a page with viewport 640x480
    When the agent calls browser_screenshot with no selector
    Then the result contains a JPEG image of the full viewport
    And the behaviour is identical to the pre-selector implementation

  # Rule: When selector is provided, the element must be scrolled into view before capture
  Scenario: Element screenshot via @ref scrolls into view and crops
    Given the agent has an active MCP connection to the extension
    And the agent has run browser_scan_page to populate refs
    And @e5 maps to a CSS selector for an element below the fold
    When the agent calls browser_screenshot with selector "@e5"
    Then the element is scrolled into view
    And the viewport is captured
    And the image is cropped to the element bounding rect
    And the result contains a JPEG image of the cropped element

  # Rule: Both CSS selectors and @ref identifiers must be accepted
  Scenario: Element screenshot via CSS selector
    Given the agent has an active MCP connection to the extension
    And the active tab has an element matching "#hero-image"
    When the agent calls browser_screenshot with selector "#hero-image"
    Then the element is scrolled into view
    And the image is cropped to the element bounding rect
    And the result contains a JPEG image of the cropped element

  # Rule: @ref not found returns descriptive error
  Scenario: Error when @ref is not found
    Given the agent has an active MCP connection to the extension
    And no scan state exists for the active tab
    When the agent calls browser_screenshot with selector "@e3"
    Then the result is an error with message "Ref @e3 not found. Run browser_scan_page first to scan the page."

  # Rule: If the element has zero visible dimensions, return an error rather than an empty image
  Scenario: Error when element has zero visible dimensions
    Given the agent has an active MCP connection to the extension
    And the active tab has an element matching ".hidden-el" with display:none
    When the agent calls browser_screenshot with selector ".hidden-el"
    Then the result is an error with message "Element has no visible dimensions"

  # Rule: Elements inside iframes must be supported via existing frame-aware ref resolution
  Scenario: Element screenshot in iframe via frame-aware @ref
    Given the agent has an active MCP connection to the extension
    And the agent has run browser_scan_page which scanned iframes
    And @f2e1 maps to an element inside iframe frame 2
    When the agent calls browser_screenshot with selector "@f2e1"
    Then executeScript runs in the correct frameId to get the bounding rect
    And the viewport is captured
    And the image is cropped to the element position within the viewport
    And the result contains a JPEG image of the cropped element

  # Rule: The crop must account for device pixel ratio
  Scenario: DPR scaling applies device pixel ratio to crop coordinates
    Given the agent has an active MCP connection to the extension
    And the device has a pixel ratio of 2 (Retina display)
    And the active tab has an element with CSS bounding rect (100, 200, 300, 150)
    When the agent calls browser_screenshot with selector for that element
    Then the crop uses device pixel coordinates (200, 400, 600, 300) from the captured PNG
    And the result contains a JPEG image of the correctly scaled crop

  # Rule: The cropped image must pass through the existing resize/JPEG/tile pipeline
  Scenario: Cropped element image passes through resize and tile pipeline
    Given the agent has an active MCP connection to the extension
    And the active tab has a very large element producing an image exceeding 1568px on the long edge
    When the agent calls browser_screenshot with selector for that element
    Then the cropped image is resized so the long edge is at most 1568px
    And the image is encoded as JPEG at 80% quality
    And if the result exceeds 800KB it is sliced into vertical tiles
