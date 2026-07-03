@done
@integration
@rust
@mouse-events
@clipboard
@agent-view
@text-selection
@tui
@COPY-006
Feature: Wire selection + copy into AgentView scrollback end-to-end

  """
  Wiring point: views/agent/mouse_dispatch.rs handle_scrollback_mouse. Feed the MouseEvent to a SelectionRecognizer (COPY-003) held on the AgentView (or the focused SessionContext) BEFORE the existing ScrollUp/ScrollDown wheel branch. Non-selection wheel events keep emitting Action::ScrollbackMouseWheelUp/Down. Down/Drag/Up drive selection.
  State: add selection: Option<Selection> to ScrollbackList (co-located with scroll + turn-selection state, same rationale as scrollback_select.rs). The recognizer + Osc52Clipboard live on the AgentView/SessionContext (they need the action bus + stdout). Mouse coords are converted to scrollback (row,col) by subtracting last_scrollback_area.x/y (the cached rect already used by handle_scrollback_mouse).
  New Actions in components/mod.rs: SelectionBegin(Cell), SelectionExtend(Cell), SelectionCommit, SelectionClear (reduced on the App task like the existing Scrollback* actions). tick() from the recognizer is driven from the App run loop's existing 16ms render tick arm (app/events.rs) for long-press Begin.
  Copy on commit: reducer for SelectionCommit calls scrollback.selected_text(spans) (COPY-004) with content_width, then Osc52Clipboard::copy(text) (COPY-001). Selection clears on: Action for scroll (ScrollbackMouseWheel*/line/page scroll), input submit path, and the Esc handler in dispatch.rs (add a clear-selection level BEFORE the existing AgentEscPressed / turn-select-exit cascade, or after turn-select per Q below).
  Render: render_count_visited computes content_width and viewport RowSpans from scrollback.selection (mapped by scroll offset), then calls paint_selection_highlight (COPY-005). This is the single mapping shared with selected_text so highlight and copy always agree.
  Testing strategy: (1) unit — reducer test: SelectionCommit with a seeded scrollback + selection produces the expected clipboard bytes via an injected Vec<u8> Osc52Clipboard; SelectionClear on scroll/Esc. (2) dispatch — a Down+Drag+Up sequence through handle_scrollback_mouse produces Begin/Extend/Commit; a wheel event still yields ScrollbackMouseWheel and no selection. (3) render — buffer shows REVERSED cells for an active selection. Prefer real ScrollbackList + injected writer over mocks.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mouse events inside the scrollback rect are fed to the gesture recognizer (COPY-003) before falling through to wheel-scroll handling
  #   2. A Begin gesture sets the live selection anchor and cursor to the gesture cell; an Extend gesture moves the cursor
  #   3. On Commit, the reconstructed selected text (COPY-004) is written to the clipboard via the OSC 52 writer (COPY-001), and the selection stays highlighted
  #   4. The active selection is repainted as a highlight (COPY-005) on every frame while it exists
  #   5. Mouse wheel scrolling continues to work; a wheel event that is not part of a selection scrolls the scrollback as before
  #   6. A quick click (no drag, short hold) does not create a selection and does not write to the clipboard
  #   7. The selection is cleared when the user scrolls, submits new input, or presses Esc
  #   8. Mouse capture remains enabled throughout selection and copy (no DisableMouseCapture is issued for this flow)
  #   9. The empty content width (viewport minus gutter) is used to clamp the selection so copied and highlighted text never include scrollbar glyphs
  #   10. Confirmed. Esc cascade order in dispatch.rs: (1) if a text selection is active -> SelectionClear and consume; (2) else if turn-select mode -> exit turn-select and consume (existing RPC-381 level); (3) else the normal AgentEscPressed cascade. Text selection is the most transient/foreground state so it clears first.
  #
  # EXAMPLES:
  #   1. User drags from mid-transcript across two lines and releases; those two lines' text (gutter-free) lands on the clipboard and stays highlighted
  #   2. User long-presses on a line for ~0.5s then releases; the line under the press becomes selected and its text is copied
  #   3. User scrolls the wheel over the transcript; it scrolls normally and no selection or clipboard write occurs
  #   4. User quickly clicks a line; nothing is selected and nothing is copied
  #   5. User has a selection then presses Esc; the highlight disappears and no copy happens on Esc
  #   6. User has a selection then scrolls; the selection clears (highlight removed) and scrolling proceeds
  #   7. User selects a full-width answer line that visually abuts the scrollbar; the copied clipboard text contains the answer but not the │ scrollbar glyph
  #
  # QUESTIONS (ANSWERED):
  #   Q: Esc precedence — when both a text selection is active AND turn-select (Tab) mode is on, which does Esc clear first? Proposal: Esc clears the text selection first (highest priority, consume), then a second Esc handles turn-select exit, then the normal AgentEscPressed cascade. Confirm this ordering.
  #   A: Confirmed. Esc cascade order in dispatch.rs: (1) if a text selection is active -> SelectionClear and consume; (2) else if turn-select mode -> exit turn-select and consume (existing RPC-381 level); (3) else the normal AgentEscPressed cascade. Text selection is the most transient/foreground state so it clears first.
  #
  # ========================================

  Background: User Story
    As a TUI user
    I want to drag or long-press over the transcript to select text and have it copied to my clipboard on release, with scroll and click still working
    So that I can copy transcript text end-to-end without leaving the app or disabling mouse capture

  Scenario: Dragging across two lines copies their text and keeps the highlight
    Given an AgentView showing a multi-line transcript with mouse capture enabled
    When I drag from the middle of one line to the middle of the line below and release
    Then the two lines of text without any scrollbar glyphs are written to the clipboard
    And the selection stays highlighted

  Scenario: Long-pressing a line selects and copies it
    Given an AgentView showing a multi-line transcript with mouse capture enabled
    When I press and hold on a line for about half a second and release
    Then the line under the press becomes selected and its text is written to the clipboard

  Scenario: Wheel scrolling still works and does not select or copy
    Given an AgentView showing a multi-line transcript with mouse capture enabled
    When I scroll the mouse wheel over the transcript
    Then the scrollback scrolls normally
    And no selection is created and nothing is written to the clipboard

  Scenario: A quick click does not select or copy
    Given an AgentView showing a multi-line transcript with mouse capture enabled
    When I quickly click a line without dragging
    Then nothing is selected and nothing is written to the clipboard

  Scenario: Esc clears an active selection without copying
    Given an AgentView with an active text selection
    When I press Esc
    Then the highlight disappears
    And nothing is written to the clipboard by the Esc press

  Scenario: Scrolling clears an active selection
    Given an AgentView with an active text selection
    When I scroll the transcript
    Then the selection is cleared and the highlight is removed
    And the transcript scrolls

  Scenario: Copying a line abutting the scrollbar excludes the scrollbar glyph
    Given an AgentView whose answer line visually abuts the scrollbar gutter
    When I select that full line and release
    Then the clipboard text contains the answer text but not the │ scrollbar glyph

  Scenario: Mouse capture remains enabled throughout selection and copy
    Given an AgentView showing a multi-line transcript with mouse capture enabled
    When I complete a drag selection and copy
    Then mouse capture was never disabled during the flow
