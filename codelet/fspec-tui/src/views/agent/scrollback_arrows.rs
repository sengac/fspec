//! RPC-381 — turn-selection arrow-bar painting for the AgentView
//! scrollback. Extracted from `scrollback_paint.rs` to keep that file
//! under the 300-LoC source-shape ceiling pinned by
//! `rpc094-agentview-scrollback-scroll.feature`.
//!
//! Feature: spec/features/agentview-turn-select-mode.feature
//!
//! Ports `turnSelection.ts` `generateArrowBar` + the AgentView
//! arrow-bar rendering (`AgentView.tsx:5325-5343`): a `▼` bar above
//! and a `▲` bar below the selected turn, on a gray background.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Widget};

use super::RenderedChunk;

/// RPC-381: direction of a turn-selection arrow bar. `Top` renders the
/// `▼` bar above the selected turn; `Bottom` the `▲` bar below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::views::agent) enum ArrowDir {
    Top,
    Bottom,
}

/// RPC-381: build an arrow-bar string `width` columns wide. Port of
/// `turnSelection.ts` `generateArrowBar`: one arrow glyph every
/// `spacing` columns (`▼` for `Top`, `▲` for `Bottom`), spaces
/// elsewhere. `spacing == 0` is treated as `1` to avoid div-by-zero.
pub(in crate::views::agent) fn generate_arrow_bar(
    width: usize,
    dir: ArrowDir,
    spacing: usize,
) -> String {
    let glyph = match dir {
        ArrowDir::Top => '\u{25BC}',    // ▼
        ArrowDir::Bottom => '\u{25B2}', // ▲
    };
    let step = spacing.max(1);
    let mut out = String::with_capacity(width);
    for col in 0..width {
        if col % step == 0 {
            out.push(glyph);
        } else {
            out.push(' ');
        }
    }
    out
}

/// RPC-381: paint the ▼ (top) and ▲ (bottom) arrow bars framing the
/// selected turn `sel`, on a gray-background / white-foreground style.
/// The geometry mirrors `paint_chunk_rows`: rows are laid out from
/// `area.y` after skipping the first `skip_rows` visual rows. The top
/// bar overwrites the row immediately ABOVE the selected chunk's first
/// visible row; the bottom bar the row immediately BELOW its last.
///
/// **RPC-401**: because every chunk now carries a trailing blank
/// separator row, the selected chunk's LAST visible row is its own blank
/// gutter, so the ▲ bar (painted below it) lands one row past the gutter
/// — still a blank separator — and the ▼ bar lands on the PREVIOUS
/// chunk's trailing gutter. Both bars therefore sit on blank rows and no
/// longer overwrite adjacent content.
pub(in crate::views::agent) fn paint_selection_arrow_bars(
    area: Rect,
    buf: &mut Buffer,
    chunks: &[RenderedChunk],
    content_width: u16,
    skip_rows: usize,
    sel: usize,
) {
    // Walk the same row layout as `paint_chunk_rows` to find the screen
    // y of the selected chunk's first and last painted rows.
    let mut row_idx: usize = 0;
    let mut y = area.y;
    let y_end = area.y.saturating_add(area.height);
    let mut first_y: Option<u16> = None;
    let mut last_y: Option<u16> = None;
    for (ci, chunk) in chunks.iter().enumerate() {
        for _ in &chunk.lines {
            if row_idx < skip_rows {
                row_idx += 1;
                continue;
            }
            if y >= y_end {
                break;
            }
            if ci == sel {
                if first_y.is_none() {
                    first_y = Some(y);
                }
                last_y = Some(y);
            }
            y = y.saturating_add(1);
            row_idx += 1;
        }
        if y >= y_end {
            break;
        }
    }
    let style = Style::default().bg(Color::Gray).fg(Color::White);
    let width = content_width as usize;
    if let Some(fy) = first_y {
        if fy > area.y {
            let bar = generate_arrow_bar(width, ArrowDir::Top, 4);
            let row = Rect {
                x: area.x,
                y: fy.saturating_sub(1),
                width: content_width,
                height: 1,
            };
            Paragraph::new(bar).style(style).render(row, buf);
        }
    }
    if let Some(ly) = last_y {
        // **RPC-401**: every source-backed chunk now ends with a trailing
        // blank separator row, so the selected chunk's LAST visible row
        // (`ly`) is its own gutter — paint the ▲ bar THERE so it occupies
        // the blank separator instead of overwriting the next turn's first
        // content row (`ly + 1`). Legacy source-less chunks (no separator)
        // keep the original `ly + 1` placement so the RPC-381 geometry is
        // preserved.
        let selected_has_separator = chunks
            .get(sel)
            .and_then(|c| c.lines.last())
            .map(is_blank_line)
            .unwrap_or(false);
        let by = if selected_has_separator {
            ly
        } else {
            ly.saturating_add(1)
        };
        if by < y_end {
            let bar = generate_arrow_bar(width, ArrowDir::Bottom, 4);
            let row = Rect {
                x: area.x,
                y: by,
                width: content_width,
                height: 1,
            };
            Paragraph::new(bar).style(style).render(row, buf);
        }
    }
}

/// **RPC-401**: true when a rendered line is the empty blank separator
/// row appended by `wrap_source` — no spans, or only whitespace spans.
fn is_blank_line(line: &ratatui::text::Line<'static>) -> bool {
    line.spans
        .iter()
        .all(|s| s.content.as_ref().trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_bar_places_glyph_every_spacing_columns_char_by_char() {
        // RPC-381 design §5: pin the EXACT glyph/space pattern, not just
        // `.contains`. width=12, spacing=4 ⇒ ▼ at cols 0,4,8; spaces else.
        let bar = generate_arrow_bar(12, ArrowDir::Top, 4);
        let chars: Vec<char> = bar.chars().collect();
        assert_eq!(chars.len(), 12, "string length must equal width");
        for (col, ch) in chars.iter().enumerate() {
            if col % 4 == 0 {
                assert_eq!(*ch, '\u{25BC}', "expected ▼ at col {col}");
            } else {
                assert_eq!(*ch, ' ', "expected space at col {col}");
            }
        }
        // And as a whole-string sanity check.
        assert_eq!(bar, "\u{25BC}   \u{25BC}   \u{25BC}   ");
    }

    #[test]
    fn bottom_bar_uses_up_glyph_at_same_columns() {
        let bar = generate_arrow_bar(12, ArrowDir::Bottom, 4);
        let chars: Vec<char> = bar.chars().collect();
        assert_eq!(chars.len(), 12);
        for (col, ch) in chars.iter().enumerate() {
            if col % 4 == 0 {
                assert_eq!(*ch, '\u{25B2}', "expected ▲ at col {col}");
            } else {
                assert_eq!(*ch, ' ', "expected space at col {col}");
            }
        }
        assert_eq!(bar, "\u{25B2}   \u{25B2}   \u{25B2}   ");
    }

    #[test]
    fn boundary_widths_lock_edge_behaviour() {
        // width=0 ⇒ empty string.
        assert_eq!(generate_arrow_bar(0, ArrowDir::Top, 4), "");
        // width=3, spacing=4 ⇒ single leading glyph then two spaces.
        let bar = generate_arrow_bar(3, ArrowDir::Top, 4);
        let chars: Vec<char> = bar.chars().collect();
        assert_eq!(chars, vec!['\u{25BC}', ' ', ' ']);
    }
}
