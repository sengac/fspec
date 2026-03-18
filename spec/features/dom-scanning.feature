@LOCATE-004
Feature: DOM Scanning Core — browser_scan_page Tool
  """
  Two-file implementation: (1) scanPageDOM injected function lives in browser-tools.ts as the browser_scan_page handler — it uses chrome.scripting.executeScript() ISOLATED world to inject a TreeWalker-based scanning function. (2) Ref assignment and tree formatting happen in the service worker handler after the injected function returns raw element data. The handler stores refs via setTabScanState() from ref-state.ts (LOCATE-003). The injected function must be serializable (no closures over external state) since it runs in page context.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Scanning uses TreeWalker for O(n) DOM traversal, skipping SCRIPT/STYLE/NOSCRIPT nodes and their children via FILTER_REJECT
  #   2. Interactive elements detected via combined selector: a[href], button, input, textarea, select, ARIA role widgets, [contenteditable], [tabindex], summary, details — plus heuristic checks for cursor:pointer and onclick/onmousedown/onkeydown attributes
  #   3. Visibility filtering uses element.checkVisibility() for CSS display/visibility/opacity checks, plus getBoundingClientRect() for zero-size filtering
  #   4. Elements with aria-disabled=true, aria-hidden=true, or pointer-events:none are excluded from interactive elements
  #   5. CSS selector generation uses ranked strategy: data-testid > id (excluding dynamic IDs) > unique attribute combo > nth-child path
  #   6. Dynamic IDs are excluded from selector generation: IDs with >30% digits, >8 chars that look hex-like, or matching patterns like r-[hash], ember[digits], react-[hash]
  #   7. Output format is an indented accessibility tree: role "name" [ref=eN] [attr=value] — structural elements (headings, regions) included for context without refs, interactive elements get refs
  #   8. Refs are assigned by the service worker handler (e1, e2, ...) and stored via setTabScanState from ref-state.ts — NOT in the injected scanning function
  #   9. Accessible name extraction priority: aria-label > aria-labelledby > placeholder > value > alt > title > direct text content (trimmed, max 80 chars)
  #   10. Role extraction uses implicit role mapping (button→button, a[href]→link, input[type=text]→textbox, h1-h6→heading, etc.) with explicit role attribute always overriding
  #   11. Tool accepts optional parameters: tabId (defaults to active tab), interactive (boolean, default true — filters to only interactive elements), selector (CSS selector to scope scan to a DOM subtree)
  #   12. Metadata returned alongside tree text includes: url, title, viewport dimensions, total element count, and interactive ref count
  #   13. Validation-relevant attributes included in output when present: type, checked, selected, expanded, pressed, disabled, required, placeholder, min, max, minlength, maxlength, step, pattern, accept, multiple, inputmode, autocomplete, level (for headings)
  #   14. Interactive parent elements (links, buttons, role=button) that fully contain children should claim the children — only the parent gets a ref, not the contained spans/imgs/svgs
  #
  # EXAMPLES:
  #   1. Login page scan: page has h1 'Sign In', email input, password input, submit button, 'Forgot Password' link — returns tree with heading for context, 4 interactive elements with refs e1-e4, metadata shows 4 interactive elements
  #   2. Page with hidden elements: div with display:none containing a button, and a visible button — only the visible button gets a ref, hidden button is filtered out
  #   3. Dynamic ID filtering: button with id='btn-abc123def' (hex-like) uses fallback selector instead of #btn-abc123def; button with id='submit-btn' uses #submit-btn
  #   4. Interactive mode false: scan returns ALL elements including paragraphs, divs, spans — no interactive filtering, no refs assigned
  #   5. Scoped scan: selector='form.login' scopes TreeWalker to only traverse within that form element, returning only elements inside it
  #   6. Aria-label name extraction: input with aria-label='Search products' shows as 'textbox "Search products" [ref=e1]' instead of showing placeholder text
  #   7. Parent claiming children: <a href='/home'><span><svg>icon</svg></span><span>Home</span></a> — only the <a> gets a ref as 'link "Home" [ref=e1]', children are not separately listed
  #   8. Disabled/hidden exclusion: button with aria-disabled='true' and div with aria-hidden='true' containing a link — neither gets a ref
  #   9. Form validation attributes: input[type=email][required][pattern=.+@.+] shows as 'textbox "Email" [ref=e1] [type=email] [required] [pattern=.+@.+]'
  #   10. Cursor pointer heuristic: div with no role/onclick but cursor:pointer via CSS — detected as interactive and gets a ref
  #   11. Ref state integration: after scan, setTabScanState is called with refs map, tree text, and timestamp — resolveRef(tabId, 'e1') returns the correct RefEntry
  #   12. Error: invalid tabId returns MCP error result; restricted page (chrome://) returns error explaining page cannot be scanned
  #   13. Selector ranking: element with data-testid='email' gets selector [data-testid="email"]; element with only id='email-input' gets #email-input; element with no id gets attribute combo like input[type="email"][name="email"]
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to scan the active tab's DOM for interactive elements and structural context
    So that understand the page layout and interact with elements using ephemeral refs

  Scenario: Scan a login page and receive accessibility tree with refs
    Given a page with a heading "Sign In", an email input, a password input, a submit button, and a "Forgot Password" link
    When I call browser_scan_page with default parameters
    Then the result should contain a heading "Sign In" for structural context
    And the result should contain 4 interactive elements with refs e1 through e4
    And each interactive element should have a role, name, and ref annotation
    And the metadata should include url, title, viewport dimensions, and interactive element count

  Scenario: Hidden elements are filtered from scan results
    Given a page with a hidden div containing a button and a visible button
    When I call browser_scan_page
    Then only the visible button should appear in the tree with a ref
    And the hidden button should not appear in the results

  Scenario: Dynamic IDs are excluded from selector generation
    Given a page with a button having id "btn-abc123def" and another with id "submit-btn"
    When I call browser_scan_page
    Then the button with dynamic ID should use a fallback selector instead of the ID
    And the button with stable ID "submit-btn" should use selector "#submit-btn"

  Scenario: Non-interactive scan mode returns all elements without refs
    Given a page with interactive and non-interactive elements
    When I call browser_scan_page with interactive set to false
    Then the result should include all visible elements including paragraphs and divs
    And no elements should have ref annotations

  Scenario: Scoped scan via CSS selector parameter
    Given a page with a login form and a navigation bar both containing buttons
    When I call browser_scan_page with selector "form.login"
    Then only elements within the login form should appear in the results
    And navigation bar elements should not be included

  Scenario: Aria-label takes priority in accessible name extraction
    Given a page with an input having aria-label "Search products" and placeholder "Type here"
    When I call browser_scan_page
    Then the input should appear as textbox "Search products" using aria-label over placeholder

  Scenario: Interactive parent claims contained children
    Given a page with a link containing a span and an SVG icon
    When I call browser_scan_page
    Then only the parent link should receive a ref
    And the contained span and SVG should not appear as separate interactive elements

  Scenario: Aria-disabled and aria-hidden elements are excluded
    Given a page with a button having aria-disabled "true" and a div with aria-hidden "true" containing a link
    When I call browser_scan_page
    Then neither the disabled button nor the hidden link should have refs in the result

  Scenario: Form validation attributes are included in tree output
    Given a page with an email input having required, pattern, and type attributes
    When I call browser_scan_page
    Then the tree output should include the type, required, and pattern attributes for that element

  Scenario: Cursor pointer heuristic detects interactive divs
    Given a page with a plain div styled with cursor pointer via CSS
    When I call browser_scan_page
    Then the div should be detected as interactive and receive a ref

  Scenario: Scan results are stored in ref state for later resolution
    Given a page with interactive elements
    When I call browser_scan_page
    Then the scan state should be stored via setTabScanState
    And resolveRef with the tab ID and ref "e1" should return the correct RefEntry

  Scenario: Error handling for invalid tab ID
    Given an invalid tab ID that does not exist
    When I call browser_scan_page with that tab ID
    Then the result should be an MCP error indicating the tab was not found

  Scenario: CSS selector ranking by reliability
    Given a page with an element having data-testid "email", another with only id "email-input", and a third with no id
    When I call browser_scan_page
    Then the first element should use selector with data-testid attribute
    And the second element should use selector "#email-input"
    And the third element should use an attribute combination or nth-child selector
