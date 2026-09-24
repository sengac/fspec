@done
@bug
@rust
@tui
@scroll
@unit
@truncation
@scrollback
@agent-view
@performance
@large-session
@BUG-192
Feature: AgentView scrollback unbounded growth — cap total visual rows
  """
  Cap is MAX_SCROLLBACK_VISUAL_ROWS = 20,000 (const, scrollback.rs). Trim (trim_to_cap) is a ScrollbackList method that drops the oldest complete chunks until total <= cap, never removing the last real chunk; trimmed content is replaced by one dim opaque (source-less, non-rewrapping) marker line whose count accumulates. Trim is driven from SessionContext::push_source / insert_source_at so every chunk producer is covered; SessionContext shifts in_flight_assistant / in_flight_thinking indices by the net shift (inserted marker - removed chunks) so streaming chunks survive. Scroll offset is compensated by net removed rows (scrolled-up viewports stay pinned; offsets inside the removed region clamp to 0; stick-to-bottom recomputes). Reset clears chunks, marker and the trimmed-row counter. Per-session state (each SessionContext owns its ScrollbackList) gives mux-pane isolation for free. See spec/attachments/BUG-192/BUG-192-scrollback-cap-research.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The scrollback's total visual row count per session is capped at MAX_SCROLLBACK_VISUAL_ROWS = 20,000 (const). Trim runs after every chunk-producing push/insert, removing OLDEST complete chunks first, and never removes the last real chunk.
  #   2. Trimmed content is replaced by a single dim marker line at the top ('… N older lines trimmed …') whose count accumulates across trims. The marker is an opaque (source-less, non-rewrapping) chunk so its geometry is stable on resize.
  #   3. Scroll offset is compensated by the net removed rows so a scrolled-up viewport stays pinned to the same content; offsets that point inside the removed region clamp to the marker (offset 0); stick-to-bottom sessions show no visible change.
  #   4. Trimming one session's scrollback never affects other open sessions (per-session isolation, incl. mux panes). /clear (reset_scrollback) clears chunks, the marker, and the trimmed-row counter.
  #   5. Trim never removes in-flight (streaming) chunks: after a trim, in_flight_assistant / in_flight_thinking indices must still point at the same chunk (indices are shifted to compensate).
  #   6. Yes — 20,000 rows is the chosen cap (2026-09-22, per owner request to pick a reasonable value via simulation). Measured steady-state frame cost at 200x45 viewport: 20k rows ≈ 10.4 ms (65% of the 16ms tick), 50k ≈ 27 ms (171% — stutter threshold), 200k ≈ 128 ms (the reported cliff). 20k ≈ 400 pages and bounds per-session memory at ~3.3 MB. If post-ship profiling shows 20k is tight on low-end machines, lowering the const to 10,000 is a one-line change.
  #
  # EXAMPLES:
  #   1. A session streams while stick-to-bottom: 40 turns of assistant prose cross the cap; the last visible rows stay identical before and after the trim — the user only ever sees the marker when they scroll to the top.
  #   2. Pushing chunks totalling 21,500 wrapped rows: after the push, total_visual_rows() ≤ 20,000, the 1,500 oldest rows are gone, and the first chunk is the marker line '… 1,500 older lines trimmed …'.
  #   3. User has scrolled up 3,000 rows from the tail when a trim removes 2,000 rows above them: the viewport still shows the same rows (offset compensated by 1,999), and the marker is now the first row when they scroll all the way up.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Is 20,000 visual rows a reasonable cap? Simulation (rust/fspec-tui/examples/scrollback_perf.rs, 200x45 viewport, release): 20k rows ≈ 10.4 ms/frame (65% of the 16ms tick), 50k ≈ 27ms (171%), 200k ≈ 128ms (800%). 20k ≈ ~400 pages / ~40 heavy turns and bounds per-session memory at ~3.3 MB.
  #   A: Yes — 20,000 rows is the chosen cap (2026-09-22, per owner request to pick a reasonable value via simulation). Measured steady-state frame cost at 200x45 viewport: 20k rows ≈ 10.4 ms (65% of the 16ms tick), 50k ≈ 27 ms (171% — stutter threshold), 200k ≈ 128 ms (the reported cliff). 20k ≈ 400 pages and bounds per-session memory at ~3.3 MB. If post-ship profiling shows 20k is tight on low-end machines, lowering the const to 10,000 is a one-line change.
  #
  # ========================================
  Background: User Story
    As a TUI user with a long-running agent session
    I want to keep the AgentView responsive regardless of conversation length
    So that the scrollback stays fast and bounded in memory for arbitrarily long sessions

  Scenario: A scrollback below the cap is never trimmed
    Given a session's ScrollbackList holding chunks totalling 10,000 visual rows
    When a 100-row chunk is pushed
    Then the scrollback still holds every chunk that was pushed
    And the total visual row count is 10,100
    And no trim-marker chunk exists in the scrollback

  Scenario: Pushing past the cap trims the oldest chunks
    Given a session's ScrollbackList holding chunks totalling 20,900 visual rows
    When a 500-row chunk is pushed
    Then the total visual row count is at most 20,000
    And the oldest chunks have been removed while the newest chunks remain intact
    And the first chunk is a trim-marker line

  Scenario: The trim marker accumulates the total number of trimmed rows
    Given a session's ScrollbackList that has already trimmed 1,500 older rows
    When further pushes cause another 2,000 older rows to be trimmed
    Then the trim-marker chunk is still exactly one line
    And the marker counts 3,500 trimmed older rows in total

  Scenario: Sticky streaming shows no visible change after a trim
    Given a stick-to-bottom scrollback whose total rows exceed the cap
    When a push trims older chunks
    Then the last visible rows paint the same content as before the trim
    And the trim marker only becomes visible when scrolled to the top

  Scenario: A scrolled-up viewport stays pinned to the same content
    Given a scrollback with 21,000 visual rows where the user has scrolled up so row 3,000 from the tail is visible
    When a trim removes 2,000 rows above the viewport
    Then the viewport still shows the same rows at the same positions
    And the scroll offset has been compensated by the net removed rows

  Scenario: A scroll offset inside the removed region clamps to the marker
    Given a scrollback where the user has scrolled up past the rows that a trim removes
    When the trim removes those rows
    Then the scroll offset is clamped to 0
    And the first visible row is the trim marker

  Scenario: The trim never removes an in-flight streaming chunk
    Given a session with 20,500 total rows and an in-flight assistant chunk at the tail still accumulating deltas
    When more text deltas are streamed into the in-flight chunk so the cap is crossed
    Then the in-flight chunk is not removed by the trim
    And the in-flight chunk's index still points at the same chunk
    And further text deltas continue to append to that same chunk

  Scenario: SELECT-mode selection on a trimmed turn is cleared
    Given a scrollback in SELECT (item) mode with a turn selected that the trim removes
    When the trim removes that turn's chunk
    Then the scrollback has no selected turn

  Scenario: The trim marker has stable geometry on resize
    Given a scrollback whose first chunk is a trim marker
    When the viewport width changes and every chunk is re-wrapped
    Then the trim marker chunk keeps exactly one line
    And its marker text is unchanged

  Scenario: Reset clears the chunks, the marker and the trimmed counter
    Given a session whose scrollback has been trimmed at least once
    When the session's scrollback is reset (slash-clear)
    Then the scrollback holds no chunks and no trim marker
    And the trimmed-row counter is back to zero
    And the next push starts a fresh scrollback with no marker

  Scenario: Trimming one session never affects another open session
    Given two open sessions whose scrollbacks are both below the cap
    When session A streams until its trim fires
    Then session A's total visual row count is at most 20,000
    And session B's scrollback is unchanged (same chunks, same total rows, no marker)

  Scenario: Every push keeps the total at or under the cap
    Given a session's ScrollbackList
    When 200 successive pushes of 200-row chunks are applied (40,000 rows total)
    Then after each push the total visual row count is at most 20,000
    And the newest content is always present in the scrollback
