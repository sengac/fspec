@done
@LOCATE-007
Feature: Advanced Interactivity Heuristics

  """
  Heuristics are implemented in two places: (1) the testable helper module dom-scanner-helpers.ts exports named functions for unit testing, (2) the scanPageDOM function in scan-page-dom.ts has inlined copies of the same logic since it runs in page context via chrome.scripting.executeScript(). A new file dom-scanner-heuristics.ts holds the exported helper functions to keep dom-scanner-helpers.ts under 300 lines. The bounding box propagation (heuristic 2) replaces the existing shouldClaimChildren with a post-processing pass using getBoundingClientRect containment. The filterDynamicClasses function is used during selector generation for stable hashing, not during interactivity detection.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Label wrapper detection: labels wrapping form controls (max_depth=2) get the ref, not the inner input. Labels with 'for' attribute are skipped (they proxy to the external input). Prevents double-counting in Ant Design patterns like <label><span><input></span></label>
  #   2. Bounding box propagation: PROPAGATING_ELEMENTS (a, button, div[role=button], div[role=combobox], span[role=button], span[role=combobox], input[role=combobox]) — when a propagating parent fully contains a child (99% area containment), the child is excluded. Replaces the simpler shouldClaimChildren from LOCATE-004
  #   3. Compound control collapsing: HTML5 date, time, datetime-local, month, week, range, number, color, file inputs are represented as single interactive elements. Their internal shadow DOM sub-components (spinners, calendar dropdowns) are NOT exposed as separate refs
  #   4. Search element heuristics: divs/spans with class/id containing 'search', 'magnify', 'glass', 'lookup', 'find', 'query' or data-* search attributes are marked interactive even without ARIA roles
  #   5. Icon-size heuristics: 10-50px elements with class, role, data-action, or aria-label are likely interactive icons and should be detected as interactive
  #   6. Early exit for inert elements: aria-disabled=true, aria-hidden=true, and inert attribute should cause immediate skip in interactivity check (enhancing existing LOCATE-004 checks with 'inert' support)
  #   7. Dynamic class filtering for stable hashing: state-related CSS classes (focus, hover, active, loading, expanded, collapsed, highlighted, entering, leaving, selected, disabled, animation, transition, open, closed, visible, hidden, pressed, checked, current) are filtered before hashing, and remaining classes are sorted for stability
  #   8. All heuristics must be inlined in the scanPageDOM function (no imports) since it runs via chrome.scripting.executeScript() in ISOLATED world. The testable helper module (dom-scanner-helpers.ts) gets matching exported functions for unit testing
  #
  # EXAMPLES:
  #   1. Ant Design checkbox pattern: <label class='ant-checkbox-wrapper'><span class='ant-checkbox'><input type='checkbox'></span><span>Remember me</span></label> → single ref on the label, no ref on the inner input
  #   2. Label with for attribute: <label for='email'>Email</label><input id='email'> → label is skipped (no ref), only the input gets a ref
  #   3. Bounding box dedup: <a href='/home'><span><img src='icon.png'></span><span>Home</span></a> where children are 99%+ contained → only the <a> gets a ref, children excluded by containment check
  #   4. Compound date input: <input type='date' min='2024-01-01' max='2025-12-31'> → single ref with [type=date] [min=2024-01-01] [max=2025-12-31], shadow DOM spinners/calendar are NOT separate refs
  #   5. Search div detection: <div class='search-icon magnify' data-action='toggle-search'>🔍</div> → detected as interactive and gets a ref despite having no ARIA role, href, or onclick
  #   6. Icon button detection: 24x24px <span class='close-btn' aria-label='Close'>✕</span> → detected as interactive icon and gets a ref
  #   7. Inert element: <div inert><button>Disabled by inert</button></div> → button is skipped immediately, not included in results
  #   8. Dynamic class filtering: element with class='btn active focus hover-highlight' → filtered to stable classes 'btn' only for hashing, removing state classes
  #   9. 60px element not an icon: <span class='close-btn' style='width:60px;height:60px'>✕</span> → NOT detected as icon-size (exceeds 50px max), falls through to normal heuristics
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to scan pages with advanced interactivity heuristics that handle edge cases in modern web UIs
    So that I get accurate interactive element detection without double-counting, missed controls, or unstable element identification

  Scenario: Label wrapping a form control gets the ref instead of the inner input
    Given a page with an Ant Design checkbox pattern where a label wraps a span wrapping an input
    When I call browser_scan_page
    Then the label element should receive a ref as the interactive element
    And the inner input should not receive a separate ref

  Scenario: Label with for attribute is skipped
    Given a page with a label having a for attribute pointing to an input
    When I call browser_scan_page
    Then the label should not receive a ref
    And only the input should receive a ref

  Scenario: Bounding box propagation excludes fully contained children
    Given a page with a link containing spans and images that are fully contained within its bounding box
    When I call browser_scan_page
    Then only the parent link should receive a ref
    And the contained children should be excluded by the 99% area containment check

  Scenario: Compound HTML5 inputs are represented as single elements
    Given a page with a date input having min and max attributes
    When I call browser_scan_page
    Then the date input should receive exactly one ref
    And the tree output should include the type, min, and max attributes

  Scenario: Non-semantic search elements are detected as interactive
    Given a page with a div having search-related class names and a data-action attribute
    When I call browser_scan_page
    Then the search div should be detected as interactive and receive a ref
    And the detection should work even without ARIA roles or onclick handlers

  Scenario: Small icon-sized elements with metadata are detected as interactive
    Given a page with a 24x24 pixel span having a class name and aria-label
    When I call browser_scan_page
    Then the icon-sized element should be detected as interactive and receive a ref

  Scenario: Elements beyond icon-size range are not detected by icon heuristic
    Given a page with a 60x60 pixel span having a class name
    When I call browser_scan_page
    Then the oversized element should not be detected by the icon-size heuristic

  Scenario: Inert elements are skipped immediately
    Given a page with a div having the inert attribute containing a button
    When I call browser_scan_page
    Then the button inside the inert container should not appear in the results

  Scenario: Dynamic state classes are filtered for stable hashing
    Given an element with classes including state-related classes like active, focus, and hover
    When the dynamic class filter is applied
    Then only stable non-state classes should remain
    And the remaining classes should be sorted alphabetically
