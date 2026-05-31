@done
@rust
@scrollback
@agent-view
@bug
@tui
@RPC-078
Feature: AgentView scrollback wraps long lines and tracks visual rows for stick-to-bottom

  """
  ratatui Paragraph rendered without an explicit wrap clips at the right edge. The TS Ink reference pre-wraps every chunk into one Line per visual row using a width-aware splitter. RPC-078 ports that splitter to Rust so (a) no characters are dropped at the right edge and (b) stick-to-bottom math counts visual rows — not chunks — keeping the most recent line anchored to the last viewport row.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #  10. Scrollback lines wider than the viewport MUST be pre-wrapped into one Line per visual row before rendering
  #  11. Stick-to-bottom mode counts visual rows (wrapped) not chunks, so the latest chunk stays fully visible
  #
  # EXAMPLES:
  #   - A 300-char `API Error:` line in an 80-column scrollback wraps across 4 Line entries; no characters are clipped at the right edge
  #   - After a long wrapped chunk, the latest "You: hi" line stays anchored to the bottom row of the rendered buffer
  #
  # ========================================

  Background: User Story
    As a user reading streamed output that is wider than my terminal
    I want long lines to wrap inside the visible scrollback
    So that no characters are clipped at the right edge and the latest line stays anchored to the bottom row

  Scenario: Long line is pre-wrapped into one rendered Line per visual row
    Given an AgentView with a fresh SessionContext for session s-1 viewed in a 80-column terminal
    When the chunks subscriber forwards StreamChunk::Error containing a 300-character body for s-1
    Then the rendered chunk's lines vector contains at least 4 Line entries (one per visual row of the wrapped body inside an 80-column viewport)
    Then concatenating every span's content across every Line yields the original "API Error: ..." body with no characters dropped or truncated


  Scenario: Stick to bottom counts visual rows so the latest chunk stays visible after a long wrapped chunk
    Given a ScrollbackList rendered into an 80-column by 10-row area in stick-to-bottom mode
    When a chunk whose pre-wrapped lines fill 4 visual rows is pushed
    Then the bottom row of the rendered buffer contains "You: hi"
    When a follow-up short chunk "You: hi" is pushed
    Then no visual row of the wrapped chunk that should be visible is missing from the rendered buffer

  Scenario: Short scrollback content fills from the top of the viewport
    Given a ScrollbackList rendered into an 80-column by 24-row area in stick-to-bottom mode with only two chunks 'You: hi' and 'API Error: rate limit'
    When the scrollback is rendered into the buffer
    Then the first row of the rendered buffer contains 'You: hi'
    Then no row above the first message contains any rendered content


  Scenario: End-to-end App.render top-fills scrollback when only a user line and an API error are present
    Given an App with an active session s-1 and the AgentView routed
    When the App dispatches Action::ChunkReceived(s-1, StreamChunk::UserInput{text:"what is this card about?"}) followed by Action::ChunkReceived(s-1, StreamChunk::Error{error:"provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests"})
    Then row y=1 (the first row of the scrollback area, immediately below the SessionHeader at y=0) contains "You: what is this card about?"
    When App.render is called into an 80x24 TestBackend buffer
    Then row y=2 contains "API Error:"
    Then no row between y=1 and the bottom of the scrollback area (y=h-3 where the footer sits) is blank above the first message — the empty rows fall BELOW the API error line, never above the user line

