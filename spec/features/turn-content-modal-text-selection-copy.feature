@done
@integration
@rust
@mouse-events
@clipboard
@agent-view
@text-selection
@tui
@COPY-008
Feature: Wire selection + copy into the turn-content (message selection) modal
  """
  Wiring point: views/agent/mouse_dispatch.rs handle_turn_modal_mouse (lines ~79-92). When turn_modal_seq.is_some(), feed the MouseEvent to a SelectionRecognizer (COPY-003) BEFORE the ScrollUp/ScrollDown branch. The modal is a fixed centered full-screen rect from dialog_theme_rows::turn_modal_geometry / fixed_dialog_rect; convert mouse (col,row) to body (row,col) by subtracting that rect's inner origin (border + padding). Non-selection wheel keeps emitting Action::TurnModalScrollUp/Down.
  State: hold selection on AgentView (e.g. turn_modal_selection: Option<Selection>) alongside turn_modal_seq/turn_modal_offset. The rows are the modal's styled_rows() (diff_decode::style_modal_lines) windowed by turn_modal_offset; the same windowing is shared by highlight and copy so they agree. Recognizer + Osc52Clipboard on AgentView (action bus + stdout). Reuse COPY-002/003/001 unchanged; reuse COPY-004's gutter-exclusion reader against the modal body width from turn_modal_geometry.
  Render: TurnContentModal::render (turn_modal.rs) paints REVERSED cells (COPY-005 overlay) for the visible selection rows after building the dialog body and before/around the scrollbar paint. Clears: scroll_turn_modal/jump_turn_modal/turn_modal_page (dispatch_scroll.rs) clear the selection; Esc precedence added in dispatch_select.rs so the first Esc clears selection, second Esc closes the modal (CloseTurnModal). Mouse capture stays on.
  Testing strategy: (1) unit — reducer/AgentView test: Down+Drag+Up over the modal body with a seeded turn produces the expected clipboard bytes via injected Vec<u8> Osc52Clipboard, gutter-free; selection maps correctly after a non-zero turn_modal_offset. (2) dispatch — wheel event still yields TurnModalScroll and clears any active selection; first Esc clears selection, second closes modal. (3) render — buffer shows REVERSED cells for the selected visible rows and never over the scrollbar column. Prefer real TurnContentModal + injected writer over mocks.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While the turn-content modal is open (turn_modal_seq is Some), Event::Mouse over the modal body rect is fed to the gesture recognizer (COPY-003) before the existing wheel-scroll branch in handle_turn_modal_mouse
  #   2. Selection coordinates are mapped to the modal's styled body rows using the current modal scroll offset (turn_modal_offset), so the highlight tracks the same rows the user sees
  #   3. The modal's own scrollbar gutter (paint_scrollbar column) and border/padding are excluded from the copied text and highlight
  #   4. On Commit the reconstructed selected text is written via the OSC 52 writer (COPY-001) and the highlight is retained; mouse capture stays enabled throughout
  #   5. Mouse wheel over the modal continues to scroll it (Action::TurnModalScrollUp/Down); a wheel event that is not part of a selection scrolls as before
  #   6. Scrolling the modal, or pressing Esc, clears an active modal selection; the first Esc clears the selection before the existing Esc-closes-modal behaviour
  #   7. A quick click in the modal body does not create a selection or write to the clipboard
  #
  # EXAMPLES:
  #   1. User opens a message in the modal, drags across three body lines, releases; those lines' text (gutter-free) is copied and stays highlighted
  #   2. User scrolls the modal down, then drags to select; the highlighted/copied rows are the ones now visible after scrolling, not the original top rows
  #   3. User selects a wide line that abuts the modal scrollbar; the copied text contains the message content but not the scrollbar glyph
  #   4. User has a modal selection then scrolls the wheel; the selection clears and the modal scrolls normally
  #   5. User has a modal selection then presses Esc; the highlight clears and the modal stays open; a second Esc closes the modal
  #   6. User quickly clicks in the modal body; nothing is selected and nothing is copied
  #
  # ========================================
  Background: User Story
    As a user reading a message in the turn-content modal
    I want to select and copy part or all of the message body with the mouse
    So that I can grab an agent answer, a code block, or an error message straight from the full-message view

  Scenario: Dragging across modal body lines copies their text and keeps the highlight
    Given an open turn-content modal showing a message with mouse capture enabled
    When I drag across three body lines and release
    Then those lines' text without the scrollbar gutter is written to the clipboard
    And the modal selection stays highlighted

  Scenario: Selection tracks the visible rows after scrolling the modal
    Given an open turn-content modal that I have scrolled down
    When I drag to select across the currently visible body rows
    Then the highlighted and copied rows are the ones visible after scrolling

  Scenario: Copying a wide line abutting the modal scrollbar excludes the scrollbar glyph
    Given an open turn-content modal whose body line abuts the scrollbar
    When I select that wide line and release
    Then the clipboard text contains the message content but not the scrollbar glyph

  Scenario: Wheel scrolling clears an active modal selection and scrolls normally
    Given an open turn-content modal with an active text selection
    When I scroll the mouse wheel over the modal
    Then the modal selection is cleared
    And the modal scrolls normally

  Scenario: First Esc clears the selection and a second Esc closes the modal
    Given an open turn-content modal with an active text selection
    When I press Esc
    Then the modal highlight clears and the modal stays open
    And nothing is written to the clipboard by the Esc press
    When I press Esc again
    Then the modal closes

  Scenario: A quick click in the modal body does not select or copy
    Given an open turn-content modal showing a message with mouse capture enabled
    When I quickly click in the modal body without dragging
    Then nothing is selected and nothing is written to the clipboard
