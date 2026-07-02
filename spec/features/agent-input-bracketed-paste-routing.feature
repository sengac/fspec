@done
@rust
@agent-view
@tui
@RPC-403
Feature: Bracketed paste never reaches agent input — compositor stub drops multi-line pastes
  """
  Replace Compositor::handle_paste char-splitting stub (compositor.rs:188-209): forward real Event::Paste(String) to the top modal layer's handle_event; return consumed/not-consumed
  App::handle_paste (app/events.rs:157-167): if compositor did not consume, fall back through Navigator → AgentView → MultiLineInput::handle_event Event::Paste branch (multiline_input.rs:223-236) — mirror the Event::Key routing path
  Normalize \r\n and lone \r to \n before textarea.insert_str; gate paste behind the same block_edits (Compacting) gate as typed edits in handle_key_gated path
  Audit compositor text-input layers (role_dialog has Event::Paste at :155; check other dialogs) — each must handle Event::Paste or safely ignore; do not regress modal text entry
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A paste with no modal open is inserted verbatim into the agent input at the cursor, preserving embedded newlines
  #   2. When a modal layer is open, the paste event is delivered to the modal as a real paste event (not exploded into per-character key events)
  #   3. If the compositor does not consume the paste, it falls through to the agent view input
  #   4. CRLF line endings in pasted text are normalized to LF before insertion; no carriage returns enter the buffer
  #   5. While the session is Compacting (edit gate active), paste into the agent input is suppressed and the buffer is unchanged
  #   6. A multi-line paste grows the input area up to the 6-row cap
  #
  # EXAMPLES:
  #   1. User pastes a 3-line snippet with no modal open: all 3 lines appear in the input, input grows to 3 rows, cursor at end of pasted text
  #   2. User pastes 'foo\r\nbar' from Windows: buffer becomes 'foo\nbar' with no \r characters
  #   3. User pastes into the input that already has 'prefix' before the cursor: pasted text is inserted at the cursor, existing text preserved
  #   4. User pastes while the role dialog modal is open: the dialog's paste handler receives the full paste string intact and the agent input is untouched
  #   5. User pastes while session is Compacting: nothing is inserted, buffer unchanged
  #   6. User pastes a 10-line snippet: all 10 lines are in the buffer but the input displays at most 6 rows
  #
  # ========================================
  Background: User Story
    As a TUI user in the agent view
    I want to paste text (including multi-line text) into the input and have it inserted verbatim
    So that I can paste code snippets and multi-line messages without losing lines or having the paste silently dropped

  Scenario: Multi-line paste with no modal open is inserted verbatim and grows the input
    Given the agent view is active with no modal open
    And the agent input is empty
    When I paste a 3-line snippet
    Then the input buffer contains all 3 lines separated by newlines
    And the input area reports 3 visible rows
    And the cursor is at the end of the pasted text

  Scenario: CRLF line endings are normalized to LF on paste
    Given the agent view is active with no modal open
    When I paste text containing Windows CRLF line endings
    Then the input buffer contains the lines separated by LF only
    And the input buffer contains no carriage return characters

  Scenario: Paste is inserted at the cursor preserving existing text
    Given the agent input contains "prefix" with the cursor at the end
    When I paste "pasted"
    Then the input buffer contains "prefixpasted"

  Scenario: Paste while a modal is open is delivered to the modal intact
    Given a modal layer with a paste handler is open above the agent view
    When I paste a multi-line string
    Then the modal receives the full paste string as a single paste event
    And the agent input buffer is unchanged

  Scenario: Paste while the session is compacting is suppressed
    Given the agent input contains "draft" and the compacting edit gate is active
    When I paste "more text"
    Then the input buffer still contains only "draft"

  Scenario: Large paste keeps the full buffer but caps the input at 6 visible rows
    Given the agent view is active with no modal open
    And the agent input is empty
    When I paste a 10-line snippet
    Then the input buffer contains all 10 lines
    And the input area reports 6 visible rows
