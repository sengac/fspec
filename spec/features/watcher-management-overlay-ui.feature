@watcher
@tui
@WATCH-008
Feature: Watcher Management Overlay UI

  """
  Add isWatcherMode useState and watcherList useState to AgentView.tsx state section (around line 950)
  Add /watcher command handler in handleSubmit() near other slash commands (around line 1696)
  Add watcher overlay render block following isResumeMode pattern (around line 5776)
  Import sessionGetWatchers, sessionGetRole, sessionManagerDestroy from codelet-napi
  Add watcher mode keyboard handling in useInput hook (near line 4930) for ↑↓Nav, Enter, D, N, Esc
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /watcher command opens Watcher Management overlay displaying all watchers for the current session
  #   2. Overlay lists watchers with their name, role, authority level, and status (idle/running)
  #   3. N key opens Watcher Creation dialog (WATCH-009 dependency)
  #   4. D key deletes the selected watcher (with confirmation dialog)
  #   5. Enter key opens/switches to the selected watcher session
  #   6. ↑↓ arrow keys navigate the watcher list with visual selection highlight
  #   7. Esc key closes the overlay and returns to the main agent view
  #   8. Watcher list is populated via sessionGetWatchers(currentSessionId) NAPI call
  #   9. Each watcher entry displays role info from sessionGetRole(watcherId) NAPI call
  #   10. If no watchers exist, overlay shows 'No watchers. Press N to create one.' message
  #   11. E key opens edit mode for selected watcher's name
  #   12. When navigating with arrow keys, scroll offset adjusts to keep selected watcher visible
  #   13. Edit mode persists changes to backend via sessionSetRole NAPI call
  #
  # EXAMPLES:
  #   1. User types /watcher → overlay opens showing list of 2 watchers: 'Code Reviewer (Peer, idle)' and 'Security Auditor (Supervisor, running)'
  #   2. User types /watcher with no watchers → overlay shows 'No watchers. Press N to create one.'
  #   3. User selects 'Code Reviewer' with ↓ key and presses Enter → overlay closes, session switches to Code Reviewer watcher
  #   4. User presses D on selected watcher → confirmation dialog 'Delete watcher Code Reviewer?' with Yes/No options
  #   5. User presses Esc in overlay → overlay closes, returns to main agent view with conversation visible
  #   6. User has 10 watchers, overlay shows scrollable list with scroll position indicator
  #   7. User presses E on selected watcher → inline edit mode activates, user can modify name and press Enter to save
  #   8. User has 10 watchers (visible=5), navigates to index 6 → scroll offset adjusts to keep selection visible
  #   9. User edits watcher name from 'Code Reviewer' to 'Senior Reviewer' and presses Enter → change persists
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to open a Watcher Management overlay via /watcher command
    So that I can create, view, edit, delete and open watcher sessions for the current parent session

  Scenario: Open watcher overlay with existing watchers
    Given a session with two watchers: "Code Reviewer" (Peer, idle) and "Security Auditor" (Supervisor, running)
    When the user types "/watcher" and presses Enter
    Then the Watcher Management overlay should open
    And the overlay should display "Code Reviewer (Peer, idle)"
    And the overlay should display "Security Auditor (Supervisor, running)"
    And the first watcher should be highlighted

  Scenario: Open watcher overlay with no watchers
    Given a session with no watchers
    When the user types "/watcher" and presses Enter
    Then the Watcher Management overlay should open
    And the overlay should display "No watchers. Press N to create one."

  Scenario: Navigate watcher list with arrow keys
    Given the Watcher Management overlay is open with 3 watchers
    And the first watcher is selected
    When the user presses the down arrow key
    Then the second watcher should be highlighted
    When the user presses the down arrow key
    Then the third watcher should be highlighted
    When the user presses the up arrow key
    Then the second watcher should be highlighted

  Scenario: Open selected watcher with Enter key
    Given the Watcher Management overlay is open with "Code Reviewer" selected
    When the user presses Enter
    Then the overlay should close
    And the session should switch to the "Code Reviewer" watcher

  Scenario: Delete watcher with confirmation
    Given the Watcher Management overlay is open with "Code Reviewer" selected
    When the user presses the D key
    Then a confirmation dialog should appear with message "Delete watcher Code Reviewer?"
    And the dialog should have Delete and Cancel options

  Scenario: Close overlay with Escape key
    Given the Watcher Management overlay is open
    When the user presses the Escape key
    Then the overlay should close
    And the main agent view should be visible with conversation intact

  Scenario: Scrollable list for many watchers
    Given a session with 10 watchers
    When the user types "/watcher" and presses Enter
    Then the Watcher Management overlay should show a scrollable list
    And a scroll position indicator should be visible

  Scenario: Scroll follows selection when navigating
    Given the Watcher Management overlay is open with 10 watchers
    And only 5 watchers are visible at a time
    And the first watcher is selected with scroll offset 0
    When the user presses the down arrow key 6 times
    Then the 7th watcher should be highlighted
    And the scroll offset should adjust to keep the selection visible

  Scenario: Edit watcher with E key
    Given the Watcher Management overlay is open with "Code Reviewer" selected
    When the user presses the E key
    Then inline edit mode should activate for the watcher name
    And the user can modify the name and press Enter to save

  Scenario: Edit watcher persists changes to backend
    Given the Watcher Management overlay is open with "Code Reviewer" selected
    And the user is in edit mode with value "Senior Reviewer"
    When the user presses Enter to save
    Then sessionSetRole should be called with the new name
    And the watcher list should show "Senior Reviewer"
    And reopening the overlay should still show "Senior Reviewer"
