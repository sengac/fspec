@LOCATE-005
Feature: Ref Resolution in Click and Fill Tools

  """
  Add resolveRef import from ./ref-state at top of browser-tools.ts. Ref resolution is a ~10-line prefix check at the top of each handler, after resolveTabId. Direct import is appropriate since ref-state is an internal extension module.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selectors starting with '@' are resolved from the ref map via resolveRef(tabId, refKey)
  #   2. Raw CSS selectors (not starting with '@') pass through unchanged — 100% backward compatible
  #   3. If ref is not found, return error suggesting to run browser_scan_page first
  #   4. The '@' prefix is stripped to get the ref key (e.g. '@e3' → 'e3')
  #   5. Only browser_click_element and browser_fill_form gain ref resolution (they are the only tools accepting selectors)
  #   6. '@' is not a valid start character for CSS selectors, so there is no ambiguity
  #
  # EXAMPLES:
  #   1. Click with @e1 after scan → resolves to the CSS selector stored for e1 and clicks it
  #   2. Fill form with @e3 after scan → resolves to the CSS selector stored for e3 and fills the value
  #   3. Click with @e99 (nonexistent ref) → returns error: 'Ref @e99 not found. Run browser_scan_page first to scan the page.'
  #   4. Click with @e1 on tab with no prior scan → returns error (no scan state exists)
  #   5. Click with raw CSS '#submit' → passes through unchanged, existing click logic executes
  #   6. Fill form with raw CSS 'input[name=email]' → passes through unchanged, existing fill logic executes
  #   7. Click with 'div@e1' (@ in middle) → treated as raw CSS selector, NOT as a ref
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want to use @ref shortcuts in browser_click_element and browser_fill_form
    So that I can interact with scanned elements without knowing their CSS selectors

  Scenario: Click element using ref after page scan
    Given a page has been scanned and ref "e1" maps to CSS selector "#submit-btn"
    When I call browser_click_element with selector "@e1"
    Then the handler should resolve "@e1" to "#submit-btn"
    And the element "#submit-btn" should be clicked

  Scenario: Fill form field using ref after page scan
    Given a page has been scanned and ref "e3" maps to CSS selector "input[name=email]"
    When I call browser_fill_form with selector "@e3" and value "user@example.com"
    Then the handler should resolve "@e3" to "input[name=email]"
    And the field "input[name=email]" should be filled with "user@example.com"

  Scenario: Click with nonexistent ref returns error
    Given a page has been scanned with refs "e1" through "e5"
    When I call browser_click_element with selector "@e99"
    Then the handler should return an error
    And the error message should contain "Ref @e99 not found"
    And the error message should suggest running browser_scan_page

  Scenario: Click with ref on tab with no prior scan returns error
    Given no page scan has been performed on the active tab
    When I call browser_click_element with selector "@e1"
    Then the handler should return an error
    And the error message should contain "Ref @e1 not found"

  Scenario: Click with raw CSS selector passes through unchanged
    Given a page has been scanned with some refs
    When I call browser_click_element with selector "#submit"
    Then the handler should use "#submit" as the CSS selector directly
    And the element "#submit" should be clicked

  Scenario: Fill form with raw CSS selector passes through unchanged
    Given a page has been scanned with some refs
    When I call browser_fill_form with selector "input[name=email]" and value "test@test.com"
    Then the handler should use "input[name=email]" as the CSS selector directly
    And the field should be filled with "test@test.com"

  Scenario: Selector with @ in the middle is not treated as a ref
    Given a page has been scanned with some refs
    When I call browser_click_element with selector "div@e1"
    Then the handler should use "div@e1" as the CSS selector directly
    And the selector should NOT be resolved through the ref map
