//! Manual single-column scrollbar painter for the AgentView
//! scrollback. Mirrors the TS Ink TUI `Scrollbar` component in
//! `src/tui/components/VirtualList.tsx` (lines 17-88) byte-for-byte
//! in glyphs (`■` / `│`), style (`Modifier::DIM`), and integer math
//! (`thumb_height = max(1, floor(vh*vh / total))`,
//! `thumb_pos = floor(offset*vh / total)`).
//!
//! Extracted from `scrollback.rs` to keep that file under the
//! 300-LoC ceiling pinned by `rpc024-source-shape.feature`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Paragraph, Widget};

use super::scrollback::ScrollState;
use super::RenderedChunk;

/// Paint a single-column scrollbar into `area`'s rightmost column.
///
/// Precondition: `total_rows > vh > 0` (the caller checks overflow).
/// The divisions below assume `total_rows >= vh + 1 >= 2`.
pub(super) fn paint_scrollbar(
    area: Rect,
    buf: &mut Buffer,
    vh: usize,
    total_rows: usize,
    state: ScrollState,
) {
    let effective_offset = if state.stick_to_bottom {
        total_rows.saturating_sub(vh)
    } else {
        state.offset.min(total_rows.saturating_sub(vh))
    };
    let scrollbar_height = vh;
    // TS: `thumb_height = max(1, floor((vh/total) * scrollbar_height))`
    //     `thumb_pos    = floor((offset/total) * scrollbar_height)`
    let thumb_height = std::cmp::max(1, vh.saturating_mul(scrollbar_height) / total_rows);
    let thumb_pos = effective_offset.saturating_mul(scrollbar_height) / total_rows;
    let dim = Style::default().add_modifier(Modifier::DIM);
    let x = area.x.saturating_add(area.width.saturating_sub(1));
    for i in 0..scrollbar_height {
        let row_y = area.y.saturating_add(i as u16);
        if row_y >= area.y.saturating_add(area.height) {
            break;
        }
        let is_thumb = i >= thumb_pos && i < thumb_pos.saturating_add(thumb_height);
        let glyph = if is_thumb { "\u{25A0}" } else { "\u{2502}" };
        let cell = &mut buf[(x, row_y)];
        cell.set_symbol(glyph);
        cell.set_style(dim);
    }
}

/// RPC-094: paint the windowed slice of chunk rows into `area`, skipping the
/// first `skip_rows` visual rows and stopping at the area's bottom edge.
/// Returns the number of distinct chunks that contributed at least one
/// painted row. Extracted from `ScrollbackList::render_count_visited` to keep
/// `scrollback.rs` under the 300-LoC source-shape ceiling.
pub(super) fn paint_chunk_rows(
    area: Rect,
    buf: &mut Buffer,
    chunks: &[RenderedChunk],
    content_width: u16,
    skip_rows: usize,
) -> usize {
    let mut row_idx: usize = 0;
    let mut y = area.y;
    let y_end = area.y.saturating_add(area.height);
    let mut visited = 0_usize;
    for chunk in chunks {
        if y >= y_end {
            break;
        }
        let mut chunk_visited = false;
        for line in &chunk.lines {
            if row_idx < skip_rows {
                row_idx += 1;
                continue;
            }
            if y >= y_end {
                break;
            }
            let row = Rect {
                x: area.x,
                y,
                width: content_width,
                height: 1,
            };
            Paragraph::new(line.clone()).render(row, buf);
            y = y.saturating_add(1);
            row_idx += 1;
            if !chunk_visited {
                visited = visited.saturating_add(1);
                chunk_visited = true;
            }
        }
    }
    visited
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(area: Rect, vh: usize, total: usize, state: ScrollState) -> Vec<(String, Modifier)> {
        let mut buf = Buffer::empty(area);
        paint_scrollbar(area, &mut buf, vh, total, state);
        let x = area.x + area.width - 1;
        (0..area.height)
            .map(|i| {
                let cell = &buf[(x, area.y + i)];
                (cell.symbol().to_string(), cell.modifier)
            })
            .collect()
    }

    #[test]
    fn scrollbar_uses_ts_thumb_and_track_glyphs() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let state = ScrollState {
            offset: 0,
            stick_to_bottom: false,
        };
        let col = run(area, 10, 20, state);
        // TS thumb = ■ (U+25A0), TS track = │ (U+2502)
        let glyphs: Vec<&str> = col.iter().map(|(g, _)| g.as_str()).collect();
        assert!(glyphs.iter().all(|g| *g == "\u{25A0}" || *g == "\u{2502}"));
        assert!(glyphs.contains(&"\u{25A0}"));
        assert!(glyphs.contains(&"\u{2502}"));
    }

    #[test]
    fn scrollbar_cells_carry_dim_modifier() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let state = ScrollState {
            offset: 0,
            stick_to_bottom: false,
        };
        let col = run(area, 10, 20, state);
        for (_, modifier) in col {
            assert!(modifier.contains(Modifier::DIM));
        }
    }

    #[test]
    fn thumb_pinned_to_top_when_offset_zero_not_sticking() {
        // TS: floor(vh*vh/total) = floor(100/20) = 5
        // TS: thumb_pos at offset=0 = 0
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let state = ScrollState {
            offset: 0,
            stick_to_bottom: false,
        };
        let col = run(area, 10, 20, state);
        let glyphs: Vec<&str> = col.iter().map(|(g, _)| g.as_str()).collect();
        assert_eq!(
            glyphs,
            vec![
                "\u{25A0}", "\u{25A0}", "\u{25A0}", "\u{25A0}", "\u{25A0}", "\u{2502}", "\u{2502}",
                "\u{2502}", "\u{2502}", "\u{2502}",
            ]
        );
    }

    #[test]
    fn thumb_pinned_to_bottom_when_sticking() {
        // total=20, vh=10 → effective_offset = max_offset = 10
        // thumb_pos = floor(10*10/20) = 5  → thumb occupies rows 5..10
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let state = ScrollState {
            offset: 0,
            stick_to_bottom: true,
        };
        let col = run(area, 10, 20, state);
        let glyphs: Vec<&str> = col.iter().map(|(g, _)| g.as_str()).collect();
        assert_eq!(
            glyphs,
            vec![
                "\u{2502}", "\u{2502}", "\u{2502}", "\u{2502}", "\u{2502}", "\u{25A0}", "\u{25A0}",
                "\u{25A0}", "\u{25A0}", "\u{25A0}",
            ]
        );
    }

    #[test]
    fn thumb_height_floor_min_one_for_huge_content() {
        // vh=10, total=10_000 → floor(100/10_000)=0 → max(1, …)=1
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let state = ScrollState {
            offset: 0,
            stick_to_bottom: false,
        };
        let col = run(area, 10, 10_000, state);
        let thumb_count = col.iter().filter(|(g, _)| g == "\u{25A0}").count();
        assert_eq!(
            thumb_count, 1,
            "thumb height must clamp to 1 for huge content"
        );
    }

    #[test]
    fn scrollbar_paints_into_rightmost_column_with_offset_origin() {
        // Origin offset must be honored: scrollbar at x = area.x + area.width - 1.
        let area = Rect {
            x: 5,
            y: 7,
            width: 10,
            height: 4,
        };
        let mut buf = Buffer::empty(area);
        let state = ScrollState {
            offset: 0,
            stick_to_bottom: false,
        };
        paint_scrollbar(area, &mut buf, 4, 16, state);
        let scrollbar_x = area.x + area.width - 1; // 14
        for y in area.y..area.y + area.height {
            let cell = &buf[(scrollbar_x, y)];
            let g = cell.symbol();
            assert!(g == "\u{25A0}" || g == "\u{2502}");
            assert!(cell.modifier.contains(Modifier::DIM));
        }
        // No paint on columns other than the rightmost.
        for x in area.x..scrollbar_x {
            for y in area.y..area.y + area.height {
                let cell = &buf[(x, y)];
                assert!(cell.symbol() == " " || cell.symbol().is_empty());
            }
        }
    }
}
