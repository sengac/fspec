//! RPC-029 shared row-chrome layout helpers.
//!
//! Both `SessionHeader` and `SessionFooter` paint a 1-row strip with a
//! dark-grey `#333333` background and a 1-column horizontal pad on
//! each side. They both also need to measure a `Line`'s total width
//! when right-aligning or truncating spans.
//!
//! These two helpers were duplicated verbatim between `header.rs`
//! and `footer.rs` after the RPC-029 rewrite. Consolidated here so the
//! row-chrome contract (background + padX=1 + width measurement) has
//! exactly one source of truth, and so future row-strip widgets can
//! reuse the same primitives alongside [`super::paint_row_bg`].

use ratatui::layout::Rect;
use ratatui::text::Line;

/// Carve `pad` columns off the left and right of `area`. Returns an
/// area with `width = 0` when `pad * 2 >= area.width` so callers can
/// short-circuit without underflowing.
pub(crate) fn horizontal_pad(area: Rect, pad: u16) -> Rect {
    let pad2 = pad.saturating_mul(2);
    if area.width <= pad2 {
        return Rect {
            x: area.x,
            y: area.y,
            width: 0,
            height: area.height,
        };
    }
    Rect {
        x: area.x + pad,
        y: area.y,
        width: area.width - pad2,
        height: area.height,
    }
}

/// Sum of `chars().count()` across every span in `line`. Used by the
/// row-strip widgets to right-align or truncate a `Line` to fit its
/// inner area.
pub(crate) fn line_width(line: &Line<'_>) -> usize {
    line.spans.iter().map(|s| s.content.chars().count()).sum()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::style::Style;
    use ratatui::text::Span;

    #[test]
    fn horizontal_pad_carves_one_column_each_side() {
        let inner = horizontal_pad(Rect::new(0, 0, 10, 1), 1);
        assert_eq!(inner, Rect::new(1, 0, 8, 1));
    }

    #[test]
    fn horizontal_pad_returns_zero_width_when_pad_too_large() {
        let inner = horizontal_pad(Rect::new(0, 0, 2, 1), 1);
        assert_eq!(inner.width, 0);
    }

    #[test]
    fn line_width_counts_characters_across_spans() {
        let line = Line::from(vec![
            Span::styled("ab", Style::default()),
            Span::styled("cde", Style::default()),
        ]);
        assert_eq!(line_width(&line), 5);
    }
}
