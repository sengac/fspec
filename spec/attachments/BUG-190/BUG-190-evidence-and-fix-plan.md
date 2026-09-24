# BUG-190 — 60fps busy-state TUI repaint re-wraps every scrollback row every frame

## What must be done

Stop re-doing the expensive text reflow (wrap + grapheme segmentation +
width) for every scrollback row on every frame while the session is
busy. Cache the reflowed result per (line, width) so the steady-state
~60fps repaint just blits cached rows and only recomputes when a line's
text or the pane width actually changes.

## Evidence (profile `/tmp/fspec-prof-0925`, 60s sample)

- Main thread `140751` (com.apple.main-thread) spent ~**11,000** of its
  41,726 samples in the render subtree, of which ~**4,600** were in
  `codelet_fspec_tui::views::agent::scrollback_paint::paint_chunk_rows`
  and the rest in the surrounding ratatui wrap/grapheme/width work.
- Driver: `run_loop.rs:73` — `is_session_busy()` (session status
  `Running`/`Compacting`) forces the 16 ms render tick
  (`RENDER_TICK`, `run_loop.rs:17`) to draw a **full frame every tick**
  (~60 fps) for as long as the session is busy, which is most of the
  time during a turn.
- Per frame, `paint_chunk_rows`
  (`rust/fspec-tui/src/views/agent/scrollback_paint.rs:60-100`) walks the
  visible chunk window and, for **each** visible row, calls
  `Paragraph::new(line.clone()).render(row, buf)` (line 90). ratatui's
  `Paragraph` then re-runs its reflow on that line: `LineTruncator`
  (line wrapping at `content_width`) + `unicode_segmentation::Graphemes`
  (grapheme boundaries) + `unicode_width` (display width) — all
  recomputed from scratch, per row, per frame.
- Note: this is a *baseline* cost (the agent scrollback render path is
  essentially unchanged since v0.10.8; only `animation.rs` was touched).
  It is not itself a v0.10.8 regression, but it is the main-thread CPU
  that makes the UI feel sluggish on top of the runtime being starved by
  BUG-187 (a pinned worker delays the tasks that update the stores this
  render reads).

## Why it is expensive

For a busy turn the scrollback holds many chunks/lines. Each 16 ms frame
re-wraps and re-segments **every currently-visible line** even though:
- the line text has not changed, and
- the pane width has not changed,
- nothing about the row's reflow is different from the previous frame.

The only thing that legitimately changes per frame during a turn is
*which* lines are visible (scroll offset) and the newly-appended lines at
the tail. The reflow of already-rendered, unchanged lines is pure waste.

## Changes required

1. **Cache reflowed lines.** For each scrollback line, memoize the result
   of the wrap/reflow at a given `content_width` (e.g. a
   `HashMap<(line_key, width), Vec<ReflectedRow>>` or a per-line
   `{ width, wrapped }` field on the `RenderedChunk` row struct). On
   paint, look up the cached wrap for the current width; only recompute
   when the width differs or the line's text changed since it was last
   reflected.
2. **Key by (text, width).** A line's content and the target width fully
   determine its wrap, so cache hits are exact. When a new line is
   appended at the tail (the only per-frame change during a turn), reflow
   only that line.
3. **Invalidate on width change.** On terminal resize / pane re-layout,
   bump the width key so all rows reflow once, then steady-state frames
   are cache-hits again.
4. Keep the scrollbar + selection-overlay painting (which is cheap and
   per-frame by design) unchanged.

## Acceptance criteria

- While the session is busy and the scrollback content is static,
  consecutive frames do NOT re-wrap unchanged lines: the wrap/reflow
  call count per frame drops from O(visible_rows) to O(changed_rows)
  (typically 0–1).
- A line that has not changed and whose width has not changed renders
  byte-identically to the uncached path (no visual regression — existing
  scrollback render snapshot tests stay green).
- On a width change (resize), all visible rows reflow once and then
  return to cache-hits.
- Newly appended tail lines reflow on first appearance and then hit
  cache on subsequent frames.

## Tests to add

- `rust/fspec-tui/tests/` (scrollback paint):
  - cache-hit assertion: paint the same chunk set at the same width N
    times; assert the wrap/reflow computation runs only on the first
    paint (instrument via a counting seam or by exposing the cache
    hit/miss counters on the view state).
  - width-change invalidation: paint at width W, then W+1; assert a
    reflow happens on the width change and not before.
  - unchanged-byte-stability: the buffer produced by the cached path is
    identical to a fresh uncached render (guards against memoizing a
    stale/stale-width result).
- Reuse `rust/test-helpers/` and existing scrollback fixtures.

## Out of scope

- Whether the busy-state forces 60fps at all (a product decision about
  the `is_session_busy` tick; out of scope here — this card keeps the
  60fps cadence but makes each frame cheap).
- The git-capture hang that starves the runtime → **BUG-187/188/189**.

## Related

- `rust/fspec-tui/src/views/agent/scrollback_paint.rs`
  (`paint_chunk_rows`, `paint_scrollbar`),
  `rust/fspec-tui/src/app/run_loop.rs` (RENDER_TICK, `is_session_busy`)
- BUG-187/188/189 (same performance epic)
