@done
@tui-component
@scrollback
@agent-view
@tui
@rust
@RPC-094
Feature: AgentView scrollback mouse wheel + line scroll parity with TS VirtualList
  """
  Reuse the existing WheelVelocity primitive in codelet/fspec-tui/src/components/scroll_viewport.rs (RPC-028); no new velocity logic
  ScrollbackList gains a last_rect: Option<Rect> field so mouse_dispatch.rs can hit-test without leaking layout from agent.rs
  Five new Action variants in components/mod.rs: ScrollbackLineUp, ScrollbackLineDown, ScrollbackHome, ScrollbackMouseWheelUp(u32), ScrollbackMouseWheelDown(u32). u32 carries the velocity multiplier
  Scrollbar uses the ratatui core widget (StatefulWidget with ScrollbarState) — same approach the VirtualList port spec (RPC-002 §8 attachment §3.7) prescribes. No custom glyph painting
  Up/Down arrow interception lives in views/agent/dispatch.rs: BEFORE forwarding to self.input.handle_event, check input.cursor_at_top()/cursor_at_bottom(). If at edge AND there is scrollback content beyond the viewport, emit Action::ScrollbackLineUp/Down. Otherwise pass through to MultiLineInput unchanged
  Mouse wheel routing order: handle_mode_view_mouse → handle_popup_mouse → NEW handle_scrollback_mouse → otherwise EventResult::ignored. handle_scrollback_mouse hit-tests the cached ScrollbackList::last_rect
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mouse wheel ScrollUp/ScrollDown over the scrollback rect must scroll the ScrollbackList with 1×–5× velocity ramp matching TS AgentView.tsx:4435-4458 (cap 5 within 150ms, reset to 1 after 150ms gap)
  #   2. Mouse wheel events outside the scrollback rect (over header, footer, role banner, input, or an open popup) MUST NOT scroll the scrollback — popups/mode-views absorb first per RPC-028
  #   3. Up/Down arrow keys that the MultiLineInput does NOT consume (cursor at first or last visual line of the input buffer) scroll the scrollback by exactly 1 line
  #   4. ScrollUp / arrow-up / Home / PageUp all drop stick_to_bottom mode; ScrollDown / arrow-down / End / PageDown re-engage stick when offset reaches the tail (RPC-019 invariant preserved)
  #   5. A 1-cell ratatui Scrollbar widget renders on the right edge of the scrollback area when total_visual_rows > viewport_height; the thumb position reflects offset/total. The scrollbar is hidden when content fits
  #   6. All edits must keep every touched .rs file under 300 LoC; no new crate dependencies; reuse existing WheelVelocity primitive from components/scroll_viewport.rs (RPC-028)
  #
  # EXAMPLES:
  #   1. User has been chatting with the assistant for a long session and now wants to re-read what was said five turns ago. They scroll the mouse wheel up over the chat area and see the earlier messages move into view
  #   2. User flicks the trackpad fast over the scrollback — the content jumps several lines per flick instead of crawling line-by-line (1×–5× acceleration matches the TS Ink behaviour)
  #   3. User pauses, then scrolls again after a second — the next scroll moves only one line again because the velocity reset (not still stuck at 5×)
  #   4. User scrolls back down to the bottom — new assistant messages automatically follow the latest line again (stick-to-bottom re-engages)
  #   5. User has the input box empty. They press the Up arrow key once and the scrollback moves up by one line; pressing Down returns it
  #   6. User has typed a multi-line message. While the cursor sits in the middle of the buffer, Up/Down move the cursor inside the message — they do NOT scroll the chat history
  #   7. User opens the /help slash-command popup. While hovering inside the popup, the trackpad scrolls the popup contents. Hovering OUTSIDE the popup (but still inside the chat area) the trackpad does nothing — the popup absorbs the event
  #   8. Once the chat history is longer than the visible area, a thin vertical scrollbar appears on the right edge showing how far the user has scrolled. When the user scrolls to the bottom and the content shrinks back to fit, the scrollbar disappears
  #   9. User presses Home (input not consuming it) — the scrollback jumps to the very first message of the session
  #
  # ========================================
  Background: User Story
    As a Rust TUI user
    I want to scroll the AgentView scrollback with mouse wheel and arrow keys like the TS Ink VirtualList does
    So that I can read back through earlier conversation, tool output, and thinking blocks without being forced to use only PageUp/PageDown

  Scenario: Mouse wheel up over the scrollback area scrolls the chat history up
    Given an AgentView with a chat session whose scrollback has 200 visual rows of content
    And the viewport shows the latest 30 rows with stick-to-bottom engaged
    When the user emits a mouse wheel ScrollUp event whose row/column falls inside the scrollback rect
    Then the visible scrollback content shifts so an earlier row is now at the bottom of the viewport
    And stick-to-bottom is no longer engaged

  Scenario: A fast flick of the wheel accelerates 1x to 5x within 150ms
    Given an AgentView whose scrollback has 200 visual rows
    And the wheel velocity has just been reset to 1
    When the user emits 5 ScrollUp events in rapid succession with less than 150ms between each
    Then the 5th event scrolls by 5 lines while the 1st scrolled by 1
    And the cumulative offset change equals 1 + 2 + 3 + 4 + 5 = 15 lines

  Scenario: Wheel velocity resets to 1x after a 150ms gap
    Given an AgentView whose wheel velocity has just reached 5
    When the user waits more than 150ms then emits one more ScrollUp event
    Then the next scroll moves the content by exactly 1 line

  Scenario: Scrolling back down to the tail re-engages stick-to-bottom
    Given an AgentView whose scrollback has been scrolled up so stick-to-bottom is disengaged
    When the user emits enough ScrollDown events that the offset reaches the tail
    Then stick-to-bottom is engaged again
    And subsequent new chunks pushed into the scrollback remain visible at the bottom edge

  Scenario: Up arrow with an empty input scrolls the scrollback up by one line
    Given an AgentView whose MultiLineInput is empty and focused
    And the scrollback has more rows than the viewport
    When the user presses the Up arrow key
    Then the scrollback offset decreases by exactly 1 visual row
    And stick-to-bottom is no longer engaged
    When the user then presses the Down arrow key
    Then the scrollback offset increases by exactly 1 visual row back to the previous position

  Scenario: Up arrow with the cursor mid-buffer stays inside the input
    Given an AgentView whose MultiLineInput buffer is "line-a\nline-b\nline-c" with the cursor at the start of "line-b"
    And the scrollback has more rows than the viewport
    When the user presses the Up arrow key
    Then the MultiLineInput cursor moves to "line-a"
    And the scrollback offset is unchanged

  Scenario: Mouse wheel inside a popup does not scroll the scrollback
    Given an AgentView with the /help SlashCommandPopup open
    And the popup occupies a sub-rect of the screen
    When the user emits a ScrollUp event whose row/column falls INSIDE the popup rect
    Then the popup scrolls its own contents
    And the scrollback offset is unchanged

  Scenario: Mouse wheel over the input area does not scroll the scrollback
    Given an AgentView whose scrollback has 200 visual rows
    When the user emits a ScrollUp event whose row/column falls inside the input rect (not the scrollback rect)
    Then the scrollback offset is unchanged
    And stick-to-bottom remains in its prior state

  Scenario: Scrollbar gutter appears when content exceeds the viewport
    Given an AgentView whose viewport height is 10 rows
    When the scrollback contains 25 visual rows of content
    Then a 1-cell vertical scrollbar widget is painted on the rightmost column of the scrollback area
    And the thumb position reflects the current offset divided by the total visual rows

  Scenario: Scrollbar gutter is hidden when content fits the viewport
    Given an AgentView whose viewport height is 10 rows
    When the scrollback contains 5 visual rows of content
    Then no scrollbar widget is painted in the scrollback area

  Scenario: Home jumps the scrollback to the very first message
    Given an AgentView whose MultiLineInput is empty and the scrollback has 200 visual rows
    When the user presses Home and the input does not consume it
    Then the scrollback offset becomes 0
    And stick-to-bottom is no longer engaged

  Scenario: Source shape — every touched module stays under 300 lines
    Given the RPC-094 patch has landed
    When source-shape inspection enumerates the touched .rs files
    Then every file under codelet/fspec-tui/src/views/agent/ has at most 300 lines
    And codelet/fspec-tui/src/views/agent.rs has at most 300 lines
    And codelet/fspec-tui/src/components/mod.rs has at most 300 lines per-file-equivalent budget
