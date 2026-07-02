@done
@navigation
@dialog
@tui
@RPC-396
Feature: Scrollable, space-filling help dialog with scrollbar
  """
  Add scroll_offset: usize + visible_rows: usize + wheel: WheelVelocity fields to HelpDialog. render() takes &mut self so store measured body height directly (unlike SlashCommandPopup which uses Cell). Slice content by [scroll_offset..scroll_offset+visible_rows].
  Reuse scroll_viewport::ensure_visible + WheelVelocity/WheelDirection and list_scrollbar::render_list_scrollbar (both already pub in src/components/). For content-scroll (no selection cursor) clamp scroll_offset against total-visible_rows directly, mirroring TurnContentModal scroll. Mouse wheel: match Event::Mouse(m) => m.kind ScrollUp/ScrollDown in handle_event (dialog is centered/topmost Critical, no hit-test needed, per thinking_level_dialog.rs pattern).
  Sizing: add a space-filling render path for HelpDialog rather than dialog_theme dialog_rect shrink-to-content. dialog_theme already has render_dialog_at for explicit rects (used by TurnContentModal); compute a rect filling area minus a small margin. Keep help_dialog.rs under 300 LoC; extract a help_dialog_scroll.rs submodule if needed. Existing snapshot help_dialog centered_popup_80x24 must be updated for the new size.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The help dialog expands to fill the available terminal area (leaving a small margin), not shrink-to-content
  #   2. When content exceeds the visible body height, the dialog scrolls instead of truncating
  #   3. Arrow Up/Down scroll by one line; PageUp/PageDown scroll by one visible page; Home jumps to top; End jumps to bottom
  #   4. Mouse wheel up/down scrolls the dialog using the shared WheelVelocity acceleration
  #   5. When content overflows, a proportional scrollbar is rendered in a reserved 1-column gutter using the shared render_list_scrollbar helper
  #   6. Scroll offset is clamped so it never scrolls past the last line and never above the first line; ESC still dismisses the dialog
  #
  # EXAMPLES:
  #   1. A user opens a help dialog with 40 lines of content on a 24-row terminal, sees the first page with a scrollbar, presses PageDown, and sees the next page
  #   2. A user presses the Down arrow repeatedly to the bottom, then presses End — the last line is visible and the scrollbar thumb sits at the bottom
  #   3. A user scrolls the mouse wheel down over the help dialog and the content scrolls down; scrolling up past the top does nothing
  #   4. A user opens the help dialog on a large terminal where all content fits — no scrollbar is shown and paging keys are no-ops
  #
  # ========================================
  Background: User Story
    As a fspec TUI user viewing the help dialog
    I want to scroll the help dialog with arrow keys, PageUp/PageDown, Home/End, and the mouse wheel, and see a scrollbar while the dialog fills the available space
    So that I can read all the help content even when it is longer than the screen, consistent with every other scroll view in the TUI

  Scenario: PageDown advances a page when content overflows the terminal
    Given a HelpDialog whose content has 40 lines rendered against an 80x24 TestBackend
    And the rendered dialog shows the first page with a scrollbar in a reserved gutter
    When the user presses PageDown
    Then the scroll offset advances by one visible page
    And the rendered buffer shows lines from further down the content

  Scenario: End jumps to the bottom and the thumb sits at the bottom
    Given a HelpDialog whose content has 40 lines rendered against an 80x24 TestBackend
    When the user presses End
    Then the last content line is visible
    And the scrollbar thumb is drawn at the bottom of the gutter

  Scenario: Mouse wheel scrolls the content and is clamped at the top
    Given a HelpDialog whose content has 40 lines rendered against an 80x24 TestBackend
    When the user scrolls the mouse wheel down
    Then the content scrolls down
    When the user scrolls the mouse wheel up past the top
    Then the scroll offset stays at zero

  Scenario: No scrollbar and inert paging when all content fits
    Given a HelpDialog whose content fits within a 200x60 TestBackend
    When the rendered dialog is drawn
    Then no scrollbar gutter is rendered
    And pressing PageDown leaves the scroll offset at zero
