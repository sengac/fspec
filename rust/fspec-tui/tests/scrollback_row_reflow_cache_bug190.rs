//! BUG-190 — Scrollback row reflow cache.
//!
//! Feature: spec/features/scrollback-row-reflow-cache.feature
//!
//! While a session is busy the 16ms render tick draws full frames at ~60fps,
//! and every frame re-wrapped + re-grapheme-scanned every visible
//! scrollback row (ratatui Paragraph rebuilt per row per frame in
//! `paint_chunk_rows`). These tests pin the acceptance: steady-state frames
//! re-wrap ZERO unchanged rows and blit cached reflected rows; the cached
//! path is byte-identical to a fresh render; width changes invalidate once
//! and return to cache-hits; tail/appended/mutated rows reflow on change
//! only; and the scrollbar + selection overlays keep painting per frame.
//!
//! RED PHASE: the `RowReflowCache` surface (and its wiring into
//! `render_count_visited` / `paint_chunk_rows`) does not exist yet — every
//! test below fails to compile until the implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::store::agent_view::chunk_wrap::wrap_source;
use codelet_fspec_tui::views::agent::{
    ChunkKind, ChunkSource, RenderedChunk, ScrollbackList, TurnDir,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn rect(w: u16, h: u16) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }
}

/// Opaque (source-less) chunk with one visual row per entry of `rows`.
fn opaque_chunk(seq: u64, rows: &[&str]) -> RenderedChunk {
    RenderedChunk {
        seq,
        lines: rows.iter().map(|r| Line::from(r.to_string())).collect(),
        source: None,
    }
}

/// Source-carrying chunk with pre-wrapped lines at `width` — the same
/// shape `SessionContext::push_source` builds (lines + Some(source)).
fn source_chunk(
    seq: u64,
    text: &str,
    kind: ChunkKind,
    color: Color,
    is_streaming: bool,
    width: u16,
) -> RenderedChunk {
    let source = ChunkSource {
        text: text.to_string(),
        color,
        kind,
        is_streaming,
        full_text: None,
    };
    let lines = wrap_source(&source, width);
    RenderedChunk {
        seq,
        lines,
        source: Some(source),
    }
}

/// Cell-for-cell dump of a buffer (symbol, fg, bg, modifier per cell) —
/// the byte-stability oracle (R2).
fn dump(buf: &Buffer, w: u16, h: u16) -> String {
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            let c = &buf[(x, y)];
            out.push_str(&format!(
                "{}|{:?}|{:?}|{:?};",
                c.symbol(),
                c.fg,
                c.bg,
                c.modifier
            ));
        }
        out.push('\n');
    }
    out
}

/// Symbols-only dump (rows joined by `\n`) — for "text is on screen"
/// assertions (the full `dump` separates every cell with style metadata,
/// so multi-char substrings cannot match there).
fn symbols(buf: &Buffer, w: u16, h: u16) -> String {
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Number of single-line opaque chunks: `count` chunks x 1 row each.
fn seed_single_row_chunks(list: &mut ScrollbackList, count: usize) {
    for i in 0..count {
        let row = format!("row-{i}");
        list.push(opaque_chunk(i as u64, &[row.as_str()]));
    }
}

// ---------------------------------------------------------------------
// Scenario 1: Steady-state frames re-wrap zero unchanged rows and blit
//             from cache
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn steady_state_frames_rewrap_zero_unchanged_rows_and_blit_from_cache() {
    // @step Given a ScrollbackList with settled chunks totalling 50 visual rows rendered into an 80-column by 20-row area
    let mut list = ScrollbackList::new();
    seed_single_row_chunks(&mut list, 50);
    let area = rect(80, 20);

    // @step When the scrollback is rendered a first time
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    // @step Then every visible row has been reflected exactly once
    // 50 rows > 20-row viewport → 20 rows painted; each reflected once.
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        20,
        "first paint must reflect exactly the 20 visible rows"
    );
    assert_eq!(
        list.reflow_cache().blit_hits(),
        0,
        "nothing is cached before the first paint"
    );

    // @step When the scrollback is rendered five more times at the same width with no content changes
    for _ in 0..5 {
        let mut buf = Buffer::empty(area);
        list.render_count_visited(area, &mut buf);
    }
    // @step Then the wrap/reflow count for unchanged rows has not increased
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        20,
        "steady-state frames must re-wrap ZERO unchanged rows (R1)"
    );
    // @step And the cached-blit count equals five times the visible row count
    assert_eq!(
        list.reflow_cache().blit_hits(),
        5 * 20,
        "each steady-state frame must blit all 20 visible rows from cache"
    );
}

// ---------------------------------------------------------------------
// Scenario 2: Cached frames paint byte-identical cells to a fresh render
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn cached_frames_paint_byte_identical_cells_to_a_fresh_render() {
    // @step Given a ScrollbackList with multi-style rows (colored spans, wide CJK glyphs, and a long line that wraps across several visual rows)
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    let styled = Line::from(vec![
        Span::styled("Red", Style::default().fg(Color::Red)),
        Span::styled(" bold", Style::default().add_modifier(Modifier::BOLD)),
    ]);
    list.push(RenderedChunk {
        seq: 0,
        lines: vec![styled],
        source: None,
    });
    list.push(RenderedChunk {
        seq: 1,
        lines: vec![Line::from(Span::raw("日本語のテキスト"))],
        source: None,
    });
    list.push(opaque_chunk(2, &["x".repeat(200).as_str()]));
    // Long line that wraps across several visual rows (source-carrying).
    let long = "word ".repeat(40);
    list.push(source_chunk(
        3,
        &long,
        ChunkKind::AssistantText,
        Color::White,
        false,
        80,
    ));
    let area = rect(80, 24);

    // @step When the scrollback is rendered a first time into one buffer
    let mut buf1 = Buffer::empty(area);
    list.render_count_visited(area, &mut buf1);
    // @step And the scrollback is rendered again into two further buffers with no content change
    let mut buf2 = Buffer::empty(area);
    list.render_count_visited(area, &mut buf2);
    let mut buf3 = Buffer::empty(area);
    list.render_count_visited(area, &mut buf3);

    // @step Then all three buffers are cell-for-cell identical across the whole area (symbol, foreground, background, modifier)
    let d1 = dump(&buf1, 80, 24);
    let d2 = dump(&buf2, 80, 24);
    let d3 = dump(&buf3, 80, 24);
    assert_eq!(
        d1, d2,
        "cached frame 2 must be byte-identical to frame 1 (R2)"
    );
    assert_eq!(
        d1, d3,
        "cached frame 3 must be byte-identical to frame 1 (R2)"
    );
    // Sanity: the styled content is actually present (not blanked by the cache).
    assert_eq!(buf1[(0, 0)].fg, Color::Red);
    assert!(buf1[(3, 0)].modifier.contains(Modifier::BOLD));
    // Wide CJK glyphs occupy alternate columns (continuation cells are
    // blank), so the symbols read "日 本 語 …" in the dump.
    let symbols = symbols(&buf1, 80, 24);
    assert!(
        symbols.contains("日 本 語"),
        "CJK row must be on screen: {symbols:?}"
    );
}

// ---------------------------------------------------------------------
// Scenario 3: Newly appended tail lines reflow on first appearance then
//             hit cache
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn newly_appended_tail_lines_reflow_on_first_appearance_then_hit_cache() {
    // @step Given a settled chunk rendered at a fixed width with all its rows cached
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    let settled = source_chunk(
        0,
        "one\ntwo\nthree",
        ChunkKind::AssistantText,
        Color::White,
        false,
        80,
    );
    let settled_rows = settled.lines.len(); // 3 hard lines + 1 trailing separator
    list.push(settled);
    let area = rect(80, 20);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(list.reflow_cache().reflow_calls(), settled_rows);

    // @step When a new chunk is pushed and the scrollback is re-rendered
    let new = source_chunk(
        1,
        "fresh tail content",
        ChunkKind::AssistantText,
        Color::White,
        false,
        80,
    );
    let new_rows = new.lines.len();
    list.push(new);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);

    // @step Then only the new chunk's rows were re-reflected
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        settled_rows + new_rows,
        "only the new chunk's rows may re-reflow (R1)"
    );
    // @step And the settled chunk's rows were blitted from the cache
    assert_eq!(
        list.reflow_cache().blit_hits(),
        settled_rows,
        "the settled chunk's rows must be cache-hits on this frame"
    );

    // @step When the scrollback is re-rendered again with no further change
    let reflows_before = list.reflow_cache().reflow_calls();
    let blits_before = list.reflow_cache().blit_hits();
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    // @step Then no row was re-reflected on that frame
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        reflows_before,
        "fully steady frame must re-wrap zero rows (R1)"
    );
    assert!(
        list.reflow_cache().blit_hits() > blits_before,
        "the steady frame must paint from cache"
    );
}

// ---------------------------------------------------------------------
// Scenario 4: A viewport width change reflows every visible row exactly
//             once
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn a_viewport_width_change_reflows_every_visible_row_exactly_once() {
    // @step Given a ScrollbackList rendered at width 80 whose rows are all cached
    let mut list = ScrollbackList::new();
    seed_single_row_chunks(&mut list, 100);
    let area80 = rect(80, 20);
    let mut buf = Buffer::empty(area80);
    list.render_count_visited(area80, &mut buf);
    assert_eq!(list.reflow_cache().reflow_calls(), 20, "20 visible rows");
    // Second frame at the same width: pure cache-hits.
    let mut buf = Buffer::empty(area80);
    list.render_count_visited(area80, &mut buf);
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        20,
        "no re-wrap at unchanged width"
    );

    // @step When the viewport width becomes 60 and the scrollback is re-rendered
    let area60 = rect(60, 20);
    let mut buf = Buffer::empty(area60);
    list.render_count_visited(area60, &mut buf);
    // @step Then every visible row was re-reflected exactly once
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        40,
        "width change invalidates each visible row exactly once (R3)"
    );

    // @step When the scrollback is re-rendered again at width 60
    let reflows_before = list.reflow_cache().reflow_calls();
    let mut buf = Buffer::empty(area60);
    list.render_count_visited(area60, &mut buf);
    // @step Then no row was re-reflected on that frame
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        reflows_before,
        "back to cache-hits after the one-time reflow (R3)"
    );
}

// ---------------------------------------------------------------------
// Scenario 5: Streaming in-flight chunks reflow only the in-flight chunk
//             per delta
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn streaming_in_flight_chunks_reflow_only_the_in_flight_chunk_per_delta() {
    // @step Given an in-flight assistant chunk streaming below settled chunks, all rows cached
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    list.push(opaque_chunk(0, &["settled-a", "settled-b"]));
    let in_flight_text = "alpha beta gamma delta epsilon zeta";
    let in_flight = source_chunk(
        1,
        in_flight_text,
        ChunkKind::AssistantText,
        Color::White,
        true,
        80,
    );
    let in_flight_rows = in_flight.lines.len();
    list.push(in_flight);
    let area = rect(80, 20);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    let reflows_after_first = list.reflow_cache().reflow_calls();
    assert_eq!(reflows_after_first, 2 + in_flight_rows);

    // @step When a text delta is appended to the in-flight chunk and the scrollback is re-rendered
    // Mirrors `chunk_processor::append_assistant_text` (push_str + rewrap_at).
    {
        let source = list
            .chunks_mut()
            .get_mut(1)
            .and_then(|c| c.source.as_mut())
            .expect("in-flight chunk carries a source");
        source.text.push_str(" tail");
    }
    list.rewrap_at(1);
    let new_in_flight_rows = list.chunks().get(1).map(|c| c.lines.len()).expect("chunk");
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);

    // @step Then only the in-flight chunk's rows were re-reflected
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        reflows_after_first + new_in_flight_rows,
        "only the in-flight chunk's rows may re-reflow on a delta (R1)"
    );
    // @step And the settled chunks' rows above it were blitted from the cache
    assert_eq!(
        list.reflow_cache().blit_hits(),
        2,
        "the 2 settled rows must be cache-hits on this frame"
    );
    // The new tail text is actually on screen (no stale pixels).
    let symbols = symbols(&buf, 80, 20);
    assert!(
        symbols.contains("tail"),
        "the appended delta must be visible: {symbols:?}"
    );
}

// ---------------------------------------------------------------------
// Scenario 6: Externally mutated chunk content is detected and
//             re-reflected once
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn externally_mutated_chunk_content_is_detected_and_re_reflected_once() {
    // @step Given a ScrollbackList whose rows are all cached
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    list.push(opaque_chunk(0, &["old-line-a", "old-line-b"]));
    list.push(opaque_chunk(1, &["keeper-1", "keeper-2"]));
    let area = rect(80, 20);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(list.reflow_cache().reflow_calls(), 4);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(list.reflow_cache().reflow_calls(), 4, "steady-state cached");

    // @step When a caller rewrites one chunk's lines in place through the mutable chunk access
    {
        let chunks = list.chunks_mut();
        chunks.get_mut(0).expect("chunk 0").lines.clear();
        chunks
            .get_mut(0)
            .expect("chunk 0")
            .lines
            .push(Line::from("REPLACED"));
    }
    // @step And the scrollback is re-rendered
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);

    // @step Then the mutated chunk's rows were re-reflected and show the new text
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        5,
        "the mutated chunk's single row re-reflects once (R4)"
    );
    let symbols = symbols(&buf, 80, 20);
    assert!(
        symbols.contains("REPLACED"),
        "new text must be on screen: {symbols:?}"
    );
    assert!(
        !symbols.contains("old-line-a"),
        "stale content must be gone"
    );
    // @step And every other chunk's rows were blitted from the cache
    assert_eq!(
        list.reflow_cache().blit_hits(),
        6,
        "frame 2 blitted the 4 cached rows; frame 3 blits the 2 keeper rows"
    );
}

// ---------------------------------------------------------------------
// Scenario 7: Reset clears the row reflow cache
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn reset_clears_the_row_reflow_cache() {
    // @step Given a ScrollbackList that has been rendered so its rows are cached
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    list.push(opaque_chunk(0, &["a", "b"]));
    let area = rect(80, 20);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(list.reflow_cache().reflow_calls(), 2);

    // @step When the scrollback is reset and a new chunk is pushed
    list.reset();
    // @step Then the cache holds no chunk entries
    assert_eq!(
        list.reflow_cache().chunk_count(),
        0,
        "reset must drop every cached chunk entry (R4)"
    );
    list.push(opaque_chunk(1, &["fresh"]));

    // @step And the new chunk's rows are reflected fresh on the next render
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        3,
        "2 stale reflections from before reset + 1 fresh row"
    );
    assert_eq!(
        list.reflow_cache().blit_hits(),
        0,
        "nothing may be blitted after a reset"
    );
    assert_eq!(list.reflow_cache().chunk_count(), 1);
}

// ---------------------------------------------------------------------
// Scenario 8: The reflow cache is bounded even for very long sessions
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn the_reflow_cache_is_bounded_even_for_very_long_sessions() {
    // @step Given a ScrollbackList with 600 single-row chunks rendered at a fixed width
    let mut list = ScrollbackList::new();
    seed_single_row_chunks(&mut list, 600);
    let area = rect(80, 20);
    // Paint every chunk once by scrolling through the whole list in pages.
    for start in (0..600).step_by(20) {
        list.jump_to_offset(start);
        let mut buf = Buffer::empty(area);
        list.render_count_visited(area, &mut buf);
    }

    // @step Then the reflow cache holds at most 512 chunk entries
    assert_eq!(
        list.reflow_cache().chunk_count(),
        512,
        "the cache must evict the oldest entries past the 512 bound"
    );

    // @step And rendering again at the same width re-reflects only evicted rows and blits the rest from cache
    // Jump to the TOP: rows 0..19 belong to the oldest (evicted) chunks,
    // so every visible row must re-reflect; nothing visible is cached.
    let reflows_before = list.reflow_cache().reflow_calls();
    let blits_before = list.reflow_cache().blit_hits();
    list.jump_to_offset(0);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        reflows_before + 20,
        "the 20 evicted top rows must re-reflect"
    );
    assert_eq!(
        list.reflow_cache().blit_hits(),
        blits_before,
        "no visible row at the top is still cached"
    );
    assert_eq!(
        list.reflow_cache().chunk_count(),
        512,
        "re-reflecting evicted rows must not grow the cache past the bound"
    );
}

// ---------------------------------------------------------------------
// Scenario 9: A gutter toggle counts as a width change and reflows
//             every row once
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn a_gutter_toggle_counts_as_a_width_change_and_reflows_every_row_once() {
    // @step Given a ScrollbackList with 25 one-line chunks rendered into a 20-row area (overflowing, 2-col gutter reserved)
    let mut list = ScrollbackList::new();
    seed_single_row_chunks(&mut list, 25);
    let area = rect(80, 20);
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    // 25 rows > 20-row viewport → gutter reserved → content width 78.
    assert_eq!(list.reflow_cache().reflow_calls(), 20, "first paint");
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        20,
        "steady at gutter width"
    );

    // @step When the trailing chunks are removed so the content no longer overflows
    // 5 single-row chunks out: 20 rows == viewport height → NO overflow,
    // so the 2-col gutter is released (content width 78 → 80).
    for _ in 0..5 {
        list.chunks_mut().pop();
    }
    // @step And the scrollback is re-rendered at the same terminal width
    let reflows_before = list.reflow_cache().reflow_calls();
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);

    // @step Then the content width changed from width-minus-2 to full width and every visible row was re-reflected exactly once
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        reflows_before + 20,
        "the gutter toggle (78 → 80) must reflow every visible row exactly once (R3)"
    );

    // @step When the scrollback is re-rendered again
    let reflows_before = list.reflow_cache().reflow_calls();
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    // @step Then no row was re-reflected on that frame
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        reflows_before,
        "back to cache-hits after the gutter-toggle reflow (R3)"
    );
}

// ---------------------------------------------------------------------
// Scenario 10: Scrollbar and selection overlays keep painting every frame
// ---------------------------------------------------------------------

/// Feature: spec/features/scrollback-row-reflow-cache.feature
#[test]
fn scrollbar_and_selection_overlays_keep_painting_every_frame() {
    // @step Given an overflowing ScrollbackList with a visible scrollbar and a SELECT-mode selected turn
    let mut list = ScrollbackList::new();
    seed_single_row_chunks(&mut list, 50);
    list.enter_item_mode(); // selects the last turn (seq 49)
    list.navigate_turn(TurnDir::Up); // select seq 48 (row 48; both bars visible)
    let area = rect(80, 20);

    let mut dumps = Vec::new();
    for _ in 0..3 {
        let mut buf = Buffer::empty(area);
        list.render_count_visited(area, &mut buf);
        dumps.push(dump(&buf, 80, 20));
    }

    // @step When the scrollback is re-rendered several times at the same width
    // (done above — 3 frames)
    // @step Then the scrollbar glyphs and the selection arrow bars are present in the buffer on every frame
    for (i, d) in dumps.iter().enumerate() {
        let has_thumb = d.contains('\u{25A0}'); // ■
        let has_track = d.contains('\u{2502}'); // │
        assert!(
            has_thumb && has_track,
            "frame {i}: scrollbar must paint ■ thumb + │ track every frame (R5)"
        );
        assert!(
            d.contains('\u{25BC}'),
            "frame {i}: ▼ arrow bar above the selected turn must paint"
        );
        assert!(
            d.contains('\u{25B2}'),
            "frame {i}: ▲ arrow bar below the selected turn must paint"
        );
    }
    // @step And the chunk-row cache does not alter the scrollbar or selection cells
    assert_eq!(
        dumps[0], dumps[1],
        "frame 1 must equal frame 0 cell-for-cell (overlay included)"
    );
    assert_eq!(
        dumps[0], dumps[2],
        "frame 2 must equal frame 0 cell-for-cell (overlay included)"
    );
    assert_eq!(
        list.reflow_cache().reflow_calls(),
        20,
        "only the first frame reflects rows; overlays paint per frame for free"
    );
}
