@LOCATE-006
Feature: Page Diff Tool — browser_diff_page

  """
  Myers diff algorithm implemented in separate module (myers-diff.ts) — ~80 lines, pure function, no Chrome dependencies
  Handler registered in browser-tools.ts alongside existing browser_scan_page — reuses resolveTabId, scripting.executeScript, scanPageDOM, formatAccessibilityTree
  Context lines use 1 unchanged line before/after each change group — separated by '...' ellipsis when gap >1 unchanged lines
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Diff uses simplified Myers algorithm on line-level tree text for minimum edit distance
  #   2. Reuses the same scanning function as browser_scan_page (scanPageDOM + formatAccessibilityTree)
  #   3. Previous scan state retrieved from ref-state.ts (getTabScanState); new scan updates stored state
  #   4. If no previous scan exists, return full current tree as all additions with explanatory note
  #   5. Output format uses unified diff: unchanged lines have no prefix, additions prefixed with '+ ', removals with '- '
  #   6. Returns DiffStats: { additions, removals, unchanged, changed: boolean }
  #   7. Only include changed regions with context lines (1-2 unchanged lines around changes) for token efficiency
  #   8. Tool accepts optional tabId parameter (defaults to active tab)
  #
  # EXAMPLES:
  #   1. Button text changes after click: 'Sign In' → 'Signing in...' with disabled added — shows 1 addition, 1 removal, N unchanged
  #   2. Identical pages scanned twice — diff returns 0 additions, 0 removals, changed: false
  #   3. New elements appear (e.g., form validation errors after submit) — multiple additions, 0 removals
  #   4. Elements removed (e.g., modal closed) — 0 additions, multiple removals
  #   5. No previous scan exists — returns full current tree as all additions with note 'No previous scan to compare against'
  #   6. Complete page navigation (all content changed) — all old lines removed, all new lines added
  #   7. Empty page (no elements) — handles gracefully with 0 changes
  #
  # ASSUMPTIONS:
  #   1. Diff always runs the scanner with the same parameters (interactive=true, no scope) as the default browser_scan_page to ensure comparable trees
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to see what changed on the page since the last scan
    So that I can verify my actions had the intended effect in the scan→interact→verify workflow

  Scenario: Detect single element change after interaction
    Given a page with a heading "Sign In", an email input, a password input, and a submit button
    And I have previously scanned the page with browser_scan_page
    When the button text changes from "Sign In" to "Signing in..." with disabled attribute added
    And I call browser_diff_page
    Then the diff output should show the old button line prefixed with "- "
    And the diff output should show the new button line prefixed with "+ "
    And the diff stats should show 1 addition and 1 removal
    And the changed flag should be true
    And the scan state should be updated with the new tree text

  Scenario: No changes detected between identical scans
    Given a page with a heading and two interactive elements
    And I have previously scanned the page with browser_scan_page
    When the page content has not changed
    And I call browser_diff_page
    Then the diff stats should show 0 additions and 0 removals
    And the changed flag should be false

  Scenario: New elements added to the page
    Given a page with a form containing an email input
    And I have previously scanned the page with browser_scan_page
    When new validation error elements appear on the page
    And I call browser_diff_page
    Then the diff output should show the new elements prefixed with "+ "
    And the diff stats should show additions greater than 0 and 0 removals

  Scenario: Elements removed from the page
    Given a page with a modal dialog containing interactive elements
    And I have previously scanned the page with browser_scan_page
    When the modal dialog is removed from the page
    And I call browser_diff_page
    Then the diff output should show the removed elements prefixed with "- "
    And the diff stats should show 0 additions and removals greater than 0

  Scenario: First diff without previous scan
    Given a page with interactive elements
    And no previous browser_scan_page has been called for this tab
    When I call browser_diff_page
    Then the output should contain "No previous scan to compare against"
    And all current tree lines should be shown as additions
    And the scan state should be stored for future diffs

  Scenario: Complete page change after navigation
    Given a page with a login form
    And I have previously scanned the page with browser_scan_page
    When the page content changes completely to a different page
    And I call browser_diff_page
    Then the diff should show all old lines as removals and all new lines as additions
    And the changed flag should be true

  Scenario: Empty page produces no diff changes
    Given a page with no visible elements
    And I have previously scanned the page with browser_scan_page producing an empty tree
    When I call browser_diff_page
    Then the diff stats should show 0 additions and 0 removals
    And the changed flag should be false

  Scenario: Context lines included around changes for readability
    Given a page with many elements where only one element in the middle changed
    And I have previously scanned the page with browser_scan_page
    When I call browser_diff_page
    Then unchanged lines adjacent to changes should be included for context
    And non-adjacent unchanged lines should be omitted with "..." separator
