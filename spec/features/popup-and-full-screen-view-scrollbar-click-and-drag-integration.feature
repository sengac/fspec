@tui
@done
@TUI-103
Feature: Popup and full-screen view scrollbar click-and-drag integration

  """
  Each popup view (SlashCommandPopup, FileSearchPopup) adds scrollbar_drag: ScrollbarDrag and last_scrollbar_rect: Option<Rect> fields. SearchHistoryView adds the same. TurnContentModal scrollbar handling is wired through mouse_dispatch.rs handle_turn_modal_mouse. All follow the same pattern as ResumeSessionView (TUI-101) and AgentView scrollback (TUI-102).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each popup view (SlashCommandPopup, FileSearchPopup) adds a scrollbar_drag: ScrollbarDrag field and last_scrollbar_rect: Option<Rect> field
  #   2. SearchHistoryView adds a scrollbar_drag: ScrollbarDrag field and last_scrollbar_rect: Option<Rect> field
  #   3. TurnContentModal mouse handling in mouse_dispatch.rs routes left-button events through ScrollbarDrag when the click lands on the scrollbar gutter column
  #   4. Popup views render a scrollbar gutter (rightmost column of the dialog body) when match count exceeds visible rows
  #   5. ScrollbarGeometry for popup views uses area_height=visible_rows, total_items=matches.len(), visible_items=visible_rows, current_offset=scroll_offset
  #   6. Scrollbar interaction is ignored when content fits in viewport (total_items <= visible_items)
  #   7. Scrollbar drag state is reset when the popup/modal content changes (set_matches, set_query, etc.)
  #
  # EXAMPLES:
  #   1. SlashCommandPopup with 50 commands, 10 visible rows. User clicks on scrollbar track at row 5. Popup jumps to offset 5 and scrolls to show that section.
  #   2. User types @ to open file search popup with 100 matching files. User clicks and drags the scrollbar thumb downward. File list scrolls in real time following the mouse position.
  #   3. User opens search history with 50 matches. User quickly clicks the scrollbar thumb without dragging. The list scrolls down by one viewport height.
  #   4. User opens a turn content modal with 200 lines of text. User clicks on the scrollbar track near the bottom. The modal jumps to show content near the bottom of the turn.
  #
  # ========================================

  Background: User Story
    As a TUI user
    I want to click and drag scrollbars in popup and full-screen views
    So that I can navigate long lists faster without relying on mouse wheel

  Scenario: Click on scrollbar track in SlashCommandPopup jumps to that position
    Given the slash command popup is open with more commands than fit in the visible area
    When I click the left mouse button on the scrollbar track below the thumb
    Then the popup scroll offset jumps to the position corresponding to the click
    And the popup continues to display the newly visible commands

  Scenario: Drag scrollbar thumb in FileSearchPopup continuously scrolls content
    Given the file search popup is open with more files than fit in the visible area
    When I press the left mouse button on the scrollbar thumb
    And I drag the mouse downward
    Then the file list scrolls in real time following the mouse position
    And releasing the mouse button stops the drag

  Scenario: Quick click on scrollbar thumb in SearchHistoryView scrolls one viewport height
    Given the search history view is open with more matches than fit in the visible area
    When I quickly click and release the left mouse button on the scrollbar thumb
    Then the match list scrolls down by one viewport height

  Scenario: Click on scrollbar track in TurnContentModal jumps to that position
    Given a turn content modal is open with more lines than fit in the visible area
    When I click the left mouse button on the scrollbar track near the bottom
    Then the modal jumps to show content near the bottom of the turn

  Scenario: Scrollbar interaction is ignored when content fits in viewport
    Given a popup is open with fewer items than fit in the visible area
    When I click the left mouse button on the scrollbar area
    Then the scroll offset remains unchanged

  Scenario: Scrollbar drag state resets when popup content changes
    Given a popup is open and I am in the middle of a scrollbar drag
    When the popup match list is replaced with new content
    Then the drag state is reset to idle
    And subsequent mouse drag events are ignored
