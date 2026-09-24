@done
@bug
@scrollback
@agent-view
@tui
@performance
@rust
@BUG-190
Feature: Scrollback row reflow cache — steady-state 60fps repaint blits cached rows instead of re-wrapping every row
  """
  While a session is busy, the 16ms render tick draws full frames at ~60fps.
  Every frame re-wrapped and re-grapheme-scanned every visible scrollback row
  (ratatui Paragraph rebuilt per row per frame in paint_chunk_rows), even
  though a busy turn's steady state changes only the tail: which rows are
  visible and newly-appended content.

  Fix: memoize each scrollback row's reflected result, keyed by (chunk
  content, content width). Steady-state frames blit cached rows (O(changed)
  instead of O(visible) re-wraps); a width change (including the 2-col
  gutter toggle) invalidates once, then returns to cache-hits. The cached
  path is byte-identical to the uncached path; scrollbar, arrow-bar and
  selection-highlight painting stay per-frame.

  Work unit: BUG-190 (performance epic). Evidence + fix plan:
  spec/attachments/BUG-190/BUG-190-evidence-and-fix-plan.md
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: A scrollback row's reflected (wrapped/blitted) result is cached per (chunk seq, row index) and is valid exactly while (chunk content fingerprint, content_width) is unchanged; a steady-state frame must re-wrap ZERO unchanged rows — only newly-arrived or fingerprint/width-changed rows are recomputed.
  #   2. R3: Chunk content fingerprint covers everything that determines the rendered cells: for source-carrying chunks the ChunkSource (kind, color, text, is_streaming, full_text); for source-less chunks the stored lines' span contents+styles. Width is part of the key, so any viewport-width change (including the RPC-094 2-col gutter toggle) invalidates once, then returns to cache-hits.
  #   3. R4: Every production mutation of chunk content already funnels through ScrollbackList rewrap paths (push/insert/rewrap_at/set_viewport_width → rewrap_chunk), and any external chunks_mut() mutation is caught by the content fingerprint (direct lines/source writes change the fingerprint), so no extra invalidation call sites are required; reset() clears the cache entirely.
  #   4. R2: The cache must be byte-identical to the uncached path — a cached frame paints exactly the same cells (symbol/fg/bg/modifier) as a fresh Paragraph render of the same rows at the same width.
  #   5. R5: Scrollbar painting, SELECT-mode arrow bars, and selection-highlight painting stay per-frame and unchanged (cheap by design); the cache only covers the chunk-row painting done by paint_chunk_rows.
  #
  # EXAMPLES:
  #   1. Busy steady-state: a scrollback with 500 settled rows is painted N times at the same width with no content changes; the wrap/reflow computation runs exactly once per row (on first paint), and frames 2..N produce zero re-wrap calls and zero grapheme-scan calls for unchanged rows.
  #   2. Byte-stability: the buffer produced by the cached path after a second render equals the buffer produced by the first (uncached) render cell-for-cell (symbol, fg, bg, modifier) for the same area.
  #   3. Tail streaming: the in-flight assistant chunk's source.text grows per delta; only that chunk's rows re-wrap on each delta (fingerprint changed), all settled chunks above it stay cache-hits.
  #   4. External mutation: a caller writes chunk.lines / chunk.source directly through chunks_mut() (as reconnect_notice and chunk_tool_result do); the next frame detects the fingerprint change for that chunk and re-reflects only its rows — no stale pixels remain.
  #   5. Width invalidation: paint at width 80 (all rows cache-hit after the first frame); then paint at width 60 — every row's fingerprint+width key mismatches, so all visible rows re-wrap exactly once; subsequent frames at 60 are cache-hits again. The gutter toggle (total_rows crosses the viewport-height boundary) counts as a width change.
  #
  # ASSUMPTIONS:
  #   1. Whether the busy-state forces 60fps at all (is_session_busy tick cadence) is a product decision OUT of scope — this card keeps the 60fps cadence and makes each frame cheap.
  #
  # ========================================

  Background: User Story
    As a TUI developer running a long agent turn
    I want to watch the 60fps busy-state repaint
    So that each steady-state frame blits cached scrollback rows instead of re-wrapping and re-grapheme-scanning every visible row

  Scenario: Steady-state frames re-wrap zero unchanged rows and blit from cache
    Given a ScrollbackList with settled chunks totalling 50 visual rows rendered into an 80-column by 20-row area
    When the scrollback is rendered a first time
    Then every visible row has been reflected exactly once
    When the scrollback is rendered five more times at the same width with no content changes
    Then the wrap/reflow count for unchanged rows has not increased
    And the cached-blit count equals five times the visible row count

  Scenario: Cached frames paint byte-identical cells to a fresh render
    Given a ScrollbackList with multi-style rows (colored spans, wide CJK glyphs, and a long line that wraps across several visual rows)
    When the scrollback is rendered a first time into one buffer
    And the scrollback is rendered again into two further buffers with no content change
    Then all three buffers are cell-for-cell identical across the whole area (symbol, foreground, background, modifier)

  Scenario: Newly appended tail lines reflow on first appearance then hit cache
    Given a settled chunk rendered at a fixed width with all its rows cached
    When a new chunk is pushed and the scrollback is re-rendered
    Then only the new chunk's rows were re-reflected
    And the settled chunk's rows were blitted from the cache
    When the scrollback is re-rendered again with no further change
    Then no row was re-reflected on that frame

  Scenario: A viewport width change reflows every visible row exactly once
    Given a ScrollbackList rendered at width 80 whose rows are all cached
    When the viewport width becomes 60 and the scrollback is re-rendered
    Then every visible row was re-reflected exactly once
    When the scrollback is re-rendered again at width 60
    Then no row was re-reflected on that frame

  Scenario: Streaming in-flight chunks reflow only the in-flight chunk per delta
    Given an in-flight assistant chunk streaming below settled chunks, all rows cached
    When a text delta is appended to the in-flight chunk and the scrollback is re-rendered
    Then only the in-flight chunk's rows were re-reflected
    And the settled chunks' rows above it were blitted from the cache

  Scenario: Externally mutated chunk content is detected and re-reflected once
    Given a ScrollbackList whose rows are all cached
    When a caller rewrites one chunk's lines in place through the mutable chunk access
    And the scrollback is re-rendered
    Then the mutated chunk's rows were re-reflected and show the new text
    And every other chunk's rows were blitted from the cache

  Scenario: Reset clears the row reflow cache
    Given a ScrollbackList that has been rendered so its rows are cached
    When the scrollback is reset and a new chunk is pushed
    Then the cache holds no chunk entries
    And the new chunk's rows are reflected fresh on the next render

  Scenario: The reflow cache is bounded even for very long sessions
    Given a ScrollbackList with 600 single-row chunks rendered at a fixed width
    Then the reflow cache holds at most 512 chunk entries
    And rendering again at the same width re-reflects only evicted rows and blits the rest from cache

  Scenario: A gutter toggle counts as a width change and reflows every row once
    Given a ScrollbackList with 25 one-line chunks rendered into a 20-row area (overflowing, 2-col gutter reserved)
    When the trailing chunks are removed so the content no longer overflows
    And the scrollback is re-rendered at the same terminal width
    Then the content width changed from width-minus-2 to full width and every visible row was re-reflected exactly once
    When the scrollback is re-rendered again
    Then no row was re-reflected on that frame

  Scenario: Scrollbar and selection overlays keep painting every frame
    Given an overflowing ScrollbackList with a visible scrollbar and a SELECT-mode selected turn
    When the scrollback is re-rendered several times at the same width
    Then the scrollbar glyphs and the selection arrow bars are present in the buffer on every frame
    And the chunk-row cache does not alter the scrollbar or selection cells
