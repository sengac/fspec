@EXT-012
Feature: AI-Optimized DOM Element Location for WebMCP Browser Control
  """
  Scanning script runs via chrome.scripting.executeScript in ISOLATED world (shares DOM, not JS). Uses TreeWalker for O(n) traversal. Generates unique CSS selectors using id, data attributes, nth-child fallback. Results returned as JSON via InjectionResult.
  Changes required in 3 files: (1) browser-tools.ts — add browser_scan_page handler, (2) mcp-server.mjs — add tool definition to NATIVE_TOOLS, (3) webmcp-skill.md — document the new tool for AI agents.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The tool must use chrome.scripting.executeScript() in the ISOLATED world to access the page DOM — this shares DOM but not JS context with the page
  #   2. Elements are considered interactable if they are: buttons, links, inputs, textareas, selects, contenteditable elements, or elements with click/keyboard event listeners or ARIA roles implying interactivity
  #   3. Each interactable element gets a stable numeric index (ref) starting from 1, used for subsequent browser_click_element and browser_fill_form calls
  #   4. Visibility filtering uses element.checkVisibility({opacityProperty:true, visibilityProperty:true}) + getBoundingClientRect() for zero-size checks + viewport intersection check
  #   5. Each element in the scan result includes: ref (index), tag, type (button/link/input/etc), text (visible label), selector (CSS selector for interaction), rect (bounding box), and attributes (aria-label, placeholder, name, value, href, role)
  #   6. The tool must use TreeWalker (document.createTreeWalker with NodeFilter.SHOW_ELEMENT) for efficient DOM traversal instead of querySelectorAll('*') which is slower on large DOMs
  #   7. The scan result includes page metadata: url, title, viewport dimensions, and scroll position
  #   8. The ref-based selector approach must work with existing browser_click_element and browser_fill_form tools — the scan stores a selector mapping that these tools can resolve
  #
  # EXAMPLES:
  #   1. Agent calls browser_scan_page on a login form page and receives indexed elements: [1] input[email], [2] input[password], [3] button[Login] with CSS selectors for each
  #   2. Agent calls browser_scan_page on a complex SPA (React/Lexical editor) with dynamic class names and gets reliable element refs instead of fragile CSS selectors
  #   3. Hidden elements (display:none, visibility:hidden, opacity:0, off-screen) are excluded from scan results
  #   4. Agent scans a page, gets ref 5 for a submit button, then calls browser_click_element with selector from the scan to click it
  #   5. Scan on a page with 500+ DOM elements completes within 100ms and returns only the ~30 interactable ones
  #   6. Scan result includes ARIA information: a div with role=button and aria-label=Close is detected as an interactable button
  #   7. Elements with contenteditable=true are detected as text input areas
  #   8. Scan on a new tab with no URL (about:blank) returns an empty elements array with page metadata only
  #
  # ========================================
  Background: User Story
    As a AI assistant
    I want to scan a web page for interactive elements and receive a structured, indexed representation
    So that reliably locate, understand, and interact with page elements without guessing CSS selectors or parsing raw HTML
