//! Unit tests for the BUG-190 row reflow cache (`reflow_cache`).
//!
//! Feature: spec/features/scrollback-row-reflow-cache.feature
//!
//! These exercise the cache mechanics directly (fingerprint, blit vs
//! uncached byte-identity, eviction) behind the integration scenarios
//! in `tests/scrollback_row_reflow_cache_bug190.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

fn buf_cells(buf: &Buffer, w: u16, h: u16) -> String {
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

/// A chunk carrying exactly the given line — the unit-level
/// `RowKey`/`RenderedChunk` pair (seq is the stable identity).
fn chunk_with_line(seq: u64, line: Line<'static>) -> RenderedChunk {
    RenderedChunk {
        seq,
        lines: vec![line],
        source: None,
    }
}

/// Cached blit of a row must equal the uncached `Paragraph` render of
/// the same row at the same width (R2 — the byte-identity oracle at
/// unit level).
#[test]
fn cached_blit_is_byte_identical_to_the_uncached_paragraph_render() {
    let line = Line::from(vec![
        Span::styled("Red", Style::default().fg(Color::Red)),
        Span::styled(" 日本語", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" tail"),
    ]);
    let width = 40u16;
    let chunk = chunk_with_line(7, line.clone());
    let mut cache = RowReflowCache::default();

    // Frame 1 (miss → reflect + store), frame 2 (hit → blit).
    let mut buf1 = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, width), &line, 0, 0, 0, &mut buf1);
    let mut buf2 = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, width), &line, 0, 0, 0, &mut buf2);

    assert_eq!(
        buf_cells(&buf1, width, 1),
        buf_cells(&buf2, width, 1),
        "frame 2 (cache hit) must be byte-identical to frame 1"
    );
    let counters = cache.counters();
    assert_eq!(counters.reflow_calls(), 1, "only frame 1 reflects");
    assert_eq!(counters.blit_hits(), 1, "frame 2 blits from cache");
}

/// A wide glyph must not shift the blit: the cached cells must land in
/// the same columns as the uncached render (multi-width cell reset).
#[test]
fn cached_blit_preserves_multi_width_glyph_positions() {
    let line = Line::from(Span::raw("a😀b")); // '😀' is width 2
    let width = 10u16;
    let chunk = chunk_with_line(1, line.clone());
    let mut cache = RowReflowCache::default();
    let mut buf1 = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, width), &line, 0, 0, 0, &mut buf1);
    let mut buf2 = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, width), &line, 0, 0, 0, &mut buf2);
    assert_eq!(
        buf_cells(&buf1, width, 1),
        buf_cells(&buf2, width, 1),
        "multi-width glyphs must blit into identical columns"
    );
    // 'b' lands at column 3 (a=0, 😀=1..2).
    assert_eq!(buf2[(3, 0)].symbol(), "b");
}

/// A width change is a cache miss (re-reflect), not a stale blit.
#[test]
fn a_width_change_is_a_cache_miss() {
    let line = Line::from("0123456789");
    let chunk = chunk_with_line(1, line.clone());
    let mut cache = RowReflowCache::default();
    let mut buf_a = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 8,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, 8), &line, 0, 0, 0, &mut buf_a);
    let mut buf_b = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 8,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, 8), &line, 0, 0, 0, &mut buf_b);
    // Same width again → hit; narrower width → miss (re-reflect).
    let counters = cache.counters();
    assert_eq!(counters.reflow_calls(), 1);
    assert_eq!(counters.blit_hits(), 1);
    let mut buf_c = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 5,
        height: 1,
    });
    // Width is part of the key (R3): a width-5 key for the same chunk
    // content must re-reflect instead of blitting the width-8 cells.
    let key_narrow = RowKey {
        seq: 1,
        fingerprint: fingerprint_lines(std::slice::from_ref(&line)),
        width: 5,
    };
    cache.paint_row(key_narrow, &line, 0, 0, 0, &mut buf_c);
    assert_eq!(
        cache.counters().reflow_calls(),
        2,
        "width 5 re-reflects once"
    );
    // "01234" in 5 cols, default (Reset) style, no modifiers.
    assert_eq!(
        buf_cells(&buf_c, 5, 1),
        "0|Reset|Reset|NONE;1|Reset|Reset|NONE;2|Reset|Reset|NONE;3|Reset|Reset|NONE;4|Reset|Reset|NONE;\n"
    );
}

/// Changed content (new fingerprint) is a miss; unchanged content with
/// the same fingerprint is a hit.
#[test]
fn a_fingerprint_change_is_a_cache_miss() {
    let line_a = Line::from("alpha");
    let line_b = Line::from("beta");
    let chunk_a = chunk_with_line(1, line_a.clone());
    let chunk_b = chunk_with_line(1, line_b.clone());
    let mut cache = RowReflowCache::default();
    let mut buf = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 10,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk_a, 10), &line_a, 0, 0, 0, &mut buf);
    cache.paint_row(RowKey::new(&chunk_b, 10), &line_b, 0, 0, 0, &mut buf);
    assert_eq!(
        cache.counters().reflow_calls(),
        2,
        "content change must re-reflect (no stale blit)"
    );
}

/// The cache evicts the oldest chunk once the bound is reached, and an
/// evicted chunk's rows re-reflect instead of blitting stale cells.
#[test]
fn the_cache_evicts_the_oldest_chunk_at_the_bound() {
    let mut cache = RowReflowCache::default();
    let area = Rect {
        x: 0,
        y: 0,
        width: 8,
        height: 1,
    };
    for seq in 0..(MAX_CHUNK_ENTRIES as u64) {
        let line = Line::from(format!("row-{seq}"));
        let chunk = chunk_with_line(seq, line.clone());
        let mut buf = Buffer::empty(area);
        cache.paint_row(RowKey::new(&chunk, 8), &line, 0, 0, 0, &mut buf);
    }
    assert_eq!(cache.counters().chunk_count(), MAX_CHUNK_ENTRIES);
    // One more chunk evicts seq 0.
    let line = Line::from("evictor");
    let chunk = chunk_with_line(MAX_CHUNK_ENTRIES as u64, line.clone());
    let mut buf = Buffer::empty(area);
    cache.paint_row(RowKey::new(&chunk, 8), &line, 0, 0, 0, &mut buf);
    assert_eq!(cache.counters().chunk_count(), MAX_CHUNK_ENTRIES);
    // Re-paint the evicted seq 0 → must re-reflect, not blit.
    let line0 = Line::from("row-0");
    let chunk0 = chunk_with_line(0, line0.clone());
    let hits_before = cache.counters().blit_hits();
    let mut buf = Buffer::empty(area);
    cache.paint_row(RowKey::new(&chunk0, 8), &line0, 0, 0, 0, &mut buf);
    assert_eq!(
        cache.counters().blit_hits(),
        hits_before,
        "the evicted row must re-reflect, not blit stale cells"
    );
}

/// An empty row stores an EMPTY cell list — a valid cache hit that
/// blits nothing, so blank separator rows never re-reflect in steady
/// state (the RPC-401 trailing separators are all of these).
#[test]
fn an_empty_row_is_a_cache_hit_that_blits_nothing() {
    let line = Line::default();
    let chunk = chunk_with_line(3, line.clone());
    let mut cache = RowReflowCache::default();
    let mut buf1 = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 20,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, 20), &line, 0, 0, 0, &mut buf1);
    let mut buf2 = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 20,
        height: 1,
    });
    cache.paint_row(RowKey::new(&chunk, 20), &line, 0, 0, 0, &mut buf2);
    assert_eq!(cache.counters().reflow_calls(), 1, "first paint reflects");
    assert_eq!(cache.counters().blit_hits(), 1, "second paint is a hit");
    // Both buffers stay all-blank (a blank row paints nothing extra).
    assert_eq!(buf_cells(&buf1, 20, 1), buf_cells(&buf2, 20, 1));
}
