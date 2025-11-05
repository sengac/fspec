@attachments
@tui
@dialog
@medium
@ui-enhancement
@TUI-019
Feature: Attachment selection dialog with keyboard navigation
  """
  Create new AttachmentDialog component using Ink's Box and Text components. Use virtual list pattern for scrollable attachment display. Reuse openInBrowser utility from src/utils/openBrowser.ts. Remove old 'a' key handler from UnifiedBoardLayout that opened first attachment directly. Dialog should be modal overlay rendered on top of board view.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing 'a' key opens attachment selection dialog (only if attachments exist)
  #   2. If no attachments exist, pressing 'a' does nothing (no dialog opens)
  #   3. Dialog displays all attachments in a virtual list (scrollable if more than viewport)
  #   4. Arrow up/down keys navigate through attachment list
  #   5. Pressing Enter opens the currently selected attachment in default browser
  #   6. Pressing Esc closes the dialog without opening any attachment
  #   7. Must reuse existing openInBrowser utility function
  #   8. Remove old TUI-013 single-attachment opening code from UnifiedBoardLayout
  #
  # EXAMPLES:
  #   1. User presses 'a' on work unit with 3 attachments, dialog opens showing list of all 3 files
  #   2. User navigates with arrow keys, presses Enter on 'diagram.png', file opens in browser, dialog closes
  #   3. User presses 'a', dialog opens, user presses Esc, dialog closes without opening anything
  #   4. User presses 'a' on work unit with no attachments, nothing happens (no dialog)
  #   5. Work unit has 10 attachments, dialog shows scrollable list with scroll indicators
  #
  # ========================================
  Background: User Story
    As a developer viewing work unit attachments in fspec TUI
    I want to select which attachment to open from a list
    So that I can choose the specific file I want to view when there are multiple attachments

  Scenario: Open dialog showing all attachments
    Given I am viewing a work unit with 3 attachments in the TUI
    When I press the 'a' key
    Then an attachment selection dialog should open
    And the dialog should display all 3 attachment filenames
    And the first attachment should be selected by default

  Scenario: Navigate and open selected attachment
    Given the attachment selection dialog is open with multiple attachments
    And 'diagram.png' is in the list
    When I navigate to 'diagram.png' using arrow keys
    And I press Enter
    Then 'diagram.png' should open in the default browser
    And the dialog should close

  Scenario: Close dialog with Esc key
    Given the attachment selection dialog is open
    When I press the Esc key
    Then the dialog should close
    And no attachment should be opened

  Scenario: No dialog when work unit has no attachments
    Given I am viewing a work unit with no attachments
    When I press the 'a' key
    Then no dialog should open
    And nothing should happen

  Scenario: Scrollable list for many attachments
    Given I am viewing a work unit with 10 attachments
    When I press the 'a' key
    Then an attachment selection dialog should open
    And the dialog should display a scrollable list
    And scroll indicators should be visible when scrolled
