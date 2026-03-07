@LOCATE-003
Feature: Ref State Management Module

  """
  Pure TypeScript module at extension/src/background/ref-state.ts. Uses Map<number, TabScanState> as internal state. No chrome.storage — in-memory only for sub-millisecond reads. Service worker restart clears state naturally (correct behavior — stale refs should require re-scan). Wire into existing browser-events.ts via import + clearTabScanState calls in onUpdated and onRemoved handlers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. State is stored in-memory in the service worker as a Map<number, TabScanState> keyed by tabId
  #   2. setTabScanState stores refs map, treeText, and timestamp for a given tabId
  #   3. getTabScanState returns the stored state or undefined if no scan exists for that tabId
  #   4. clearTabScanState removes the state for a given tabId without affecting other tabs
  #   5. resolveRef is a convenience wrapper that looks up a ref key (e.g. 'e1') from the stored state for a tab and returns the RefEntry or undefined
  #   6. Tab navigation (changeInfo.url in tabs.onUpdated) must trigger clearTabScanState to invalidate stale refs
  #   7. Tab close (tabs.onRemoved) must trigger clearTabScanState to clean up memory
  #   8. Each tab maintains independent state — operations on one tab never affect another tab's scan state
  #
  # EXAMPLES:
  #   1. Set scan state for tab 42 with 3 refs (e1→button, e2→textbox, e3→link), then getTabScanState(42) returns the exact same refs, treeText, and timestamp
  #   2. resolveRef(42, 'e2') returns { selector: '#email', role: 'textbox', name: 'Email' } after a scan, but returns undefined for unknown ref 'e99'
  #   3. Tab 42 has scan state, tab 99 has different scan state — clearTabScanState(42) removes only tab 42's state, getTabScanState(99) still returns its data
  #   4. Tab 42 navigates to a new URL (changeInfo.url fires) → clearTabScanState(42) is called → getTabScanState(42) returns undefined, forcing re-scan
  #   5. Tab 42 is closed (onRemoved fires) → clearTabScanState(42) is called → no memory leak from accumulated scan states of closed tabs
  #   6. getTabScanState(999) for a tab that was never scanned returns undefined (not an error)
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to have scan state persisted per-tab in the service worker
    So that the scan→interact→verify workflow has consistent ref resolution and automatic invalidation on navigation

  @unit
  Scenario: Store and retrieve scan state for a tab
    Given no scan state exists for tab 42
    When I store scan state for tab 42 with 3 refs and tree text "- button Login [ref=e1]"
    Then getTabScanState for tab 42 should return the stored refs map with 3 entries
    And the returned state should include the exact tree text and a valid timestamp

  @unit
  Scenario: Resolve a known ref to its entry
    Given tab 42 has scan state with ref "e2" mapped to selector "#email" role "textbox" name "Email"
    When I resolve ref "e2" for tab 42
    Then I should receive the RefEntry with selector "#email" role "textbox" name "Email"

  @unit
  Scenario: Resolve an unknown ref returns undefined
    Given tab 42 has scan state with ref "e2" mapped to selector "#email" role "textbox" name "Email"
    When I resolve ref "e99" for tab 42
    Then I should receive undefined

  @unit
  Scenario: Resolve ref for unknown tab returns undefined
    Given no scan state exists for tab 999
    When I resolve ref "e1" for tab 999
    Then I should receive undefined

  @unit
  Scenario: Clear scan state for one tab without affecting others
    Given tab 42 has scan state with 3 refs
    And tab 99 has scan state with 2 refs
    When I clear scan state for tab 42
    Then getTabScanState for tab 42 should return undefined
    And getTabScanState for tab 99 should still return its 2 refs

  @unit
  Scenario: Get scan state for never-scanned tab returns undefined
    Given no scan state exists for tab 999
    When I get scan state for tab 999
    Then I should receive undefined without any error

  @integration
  Scenario: Navigation event invalidates scan state
    Given tab 42 has scan state with 3 refs
    And browser event listeners are registered with ref state invalidation
    When the tab 42 onUpdated event fires with a new URL
    Then getTabScanState for tab 42 should return undefined

  @integration
  Scenario: Tab close event cleans up scan state
    Given tab 42 has scan state with 3 refs
    And browser event listeners are registered with ref state invalidation
    When the tab 42 onRemoved event fires
    Then getTabScanState for tab 42 should return undefined
