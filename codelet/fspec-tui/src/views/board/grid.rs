//! Pure-function grid helpers for the BoardView rich box-drawing topology.
//!
//! Feature: spec/features/rpc014-grid-helpers.feature
//! Card: RPC-014.
//!
//! These helpers are a literal port of `calculateColumnWidths`,
//! `getColumnWidth` and `buildBorderRow` from
//! `src/tui/components/UnifiedBoardLayout.tsx:64-93`. They are pure
//! functions (no `ratatui::Buffer` dependency) so they can be unit
//! tested without a `TestBackend`.

use crate::store::COLUMN_ORDER;

/// Result of distributing the available terminal width across the
/// seven canonical kanban columns. `base_width` is the floor of the
/// per-column share; `remainder` columns at the leading edge receive
/// `base_width + 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnWidths {
    pub base_width: u16,
    pub remainder: u16,
}

/// Which separator glyph fills the inner runs of dashes between
/// column-width spans for a given border row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorType {
    /// `─` — plain horizontal rule (no column junctions).
    Plain,
    /// `┬` — top junctions (used on the details→column-headers
    /// separator).
    Top,
    /// `┼` — cross junctions (used on the column-headers→content
    /// separator).
    Cross,
    /// `┴` — bottom junctions (used on the content→footer separator).
    Bottom,
}

/// Mirror of the TS `calculateColumnWidths(terminalWidth)`:
///   availableWidth = terminal_width - 2 outer borders - 6 separators
///   base = floor(availableWidth / 7) clamped to >= 8
///   remainder = availableWidth % 7 only when base_width >= 8 else 0
///
/// Returns sentinel `{ base_width: 8, remainder: 0 }` for terminal
/// widths so narrow that `base_width` would otherwise drop below 8.
pub fn calculate_column_widths(terminal_width: u16) -> ColumnWidths {
    let cols = COLUMN_ORDER.len() as u16;
    let borders: u16 = 2;
    let separators: u16 = cols.saturating_sub(1);
    let overhead = borders.saturating_add(separators);
    let available_width = terminal_width.saturating_sub(overhead);
    let raw_base = available_width / cols;
    let raw_remainder = available_width % cols;
    if raw_base >= 8 {
        ColumnWidths {
            base_width: raw_base,
            remainder: raw_remainder,
        }
    } else {
        // Match TS: `Math.max(8, baseWidth)` floor with zero remainder
        // so the upstream caller can detect the narrow-mode fallback
        // via `remainder == 0 && terminal_width < 64`.
        ColumnWidths {
            base_width: 8,
            remainder: 0,
        }
    }
}

/// Per-column width: leading `remainder` columns get `base_width + 1`,
/// the rest get `base_width`. Mirror of TS `getColumnWidth`.
pub fn column_width_at(idx: usize, widths: ColumnWidths) -> u16 {
    if (idx as u16) < widths.remainder {
        widths.base_width + 1
    } else {
        widths.base_width
    }
}

/// Build one of the four canonical separator strings using box-drawing
/// characters. Mirror of TS `buildBorderRow` — the fill glyph is always
/// `─`; the inner column-junction is dictated by `separator`.
///
/// Output shape: `<left><─*w0><sep><─*w1>…<sep><─*w6><right>`.
pub fn build_border_row(
    widths: ColumnWidths,
    left: &str,
    right: &str,
    separator: SeparatorType,
) -> String {
    let sep_char: &str = match separator {
        SeparatorType::Plain => "─",
        SeparatorType::Top => "┬",
        SeparatorType::Cross => "┼",
        SeparatorType::Bottom => "┴",
    };
    let cols = COLUMN_ORDER.len();
    let mut out = String::with_capacity(left.len() + right.len() + 256);
    out.push_str(left);
    for idx in 0..cols {
        if idx > 0 {
            out.push_str(sep_char);
        }
        let w = column_width_at(idx, widths) as usize;
        for _ in 0..w {
            out.push('─');
        }
    }
    out.push_str(right);
    out
}

/// Mirror of TS `calculateViewportHeight` — how many rows are
/// available for the column-content area between the column-header
/// separator and the footer separator.
///
/// Fixed rows used elsewhere in the layout:
///   1 (top border) + 1 (header-bottom separator placeholder) +
///   5 (details strip) + 1 (details-bottom separator) +
///   1 (column-header row) + 1 (column-header separator) +
///   1 (footer separator) + 1 (footer) + 1 (bottom border) = 13.
///
/// Note: RPC-014 does not render the 4-row "logo + checkpoints"
/// header (that lands in RPC-015) so the placeholder above keeps the
/// math local to this slice. RPC-015 will rebase this constant.
pub fn calculate_viewport_height(terminal_height: u16) -> u16 {
    let fixed_rows: u16 = 13;
    terminal_height.saturating_sub(fixed_rows).max(5)
}

/// RPC-023: slice a horizontal strip (`area`, height = whatever the
/// caller passes — typically the column-header row or the content
/// area) into per-column [`Rect`]s, accounting for the leading `│`
/// border, each column's width from [`column_width_at`], and the `│`
/// separator between columns. The returned array is indexed by
/// [`crate::store::COLUMN_ORDER`] position.
pub fn slice_column_rects(area: ratatui::layout::Rect, widths: ColumnWidths) -> [ratatui::layout::Rect; 7] {
    let mut rects = [ratatui::layout::Rect::default(); 7];
    let mut x = area.x.saturating_add(1);
    for (idx, slot) in rects.iter_mut().enumerate() {
        let w = column_width_at(idx, widths);
        *slot = ratatui::layout::Rect {
            x,
            y: area.y,
            width: w,
            height: area.height,
        };
        x = x.saturating_add(w);
        if idx < 6 {
            x = x.saturating_add(1);
        }
    }
    rects
}
