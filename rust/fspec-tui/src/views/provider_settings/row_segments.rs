//! RPC-350 R4 — span-aware row painter for provider/api-key rows.
//!
//! Feature: spec/features/rpc350-provider-settings-list-mode-parity.feature
//!
//! `row_render::render_row` paints a whole row with a SINGLE `Style`. That is
//! correct for profile / oauth rows but wrong for provider + api-key rows
//! whose inline status decorations must each carry their own colour (TS
//! `ProviderSettingsPanel.tsx:586-633` and `:728-749`):
//!   * name           -> white  (black when selected)
//!   * `✓ {masked}`   -> green  (black when selected)
//!   * ` [{source}]`  -> dim    (black when selected)
//!   * `(not configured)` / `(not set)` -> gray (black when selected)
//!   * ` (N profile/s)` (openai header only) -> dim (black when selected)
//!
//! This module composes a `Vec<Segment>` (text + per-segment foreground) and
//! paints them on top of the row's full-width colour band, preserving the
//! wide-glyph band so emoji continuation cells keep their background
//! (mirrors the band-repair loop in `row_render::render_row`).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

/// A semantic colour role for one inline label fragment. The concrete
/// foreground is resolved by [`SegmentRole::fg`], which flips every role to
/// `Black` on a selected row so the text stays readable on the colour band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentRole {
    /// Provider/api-key name — white when unselected.
    Name,
    /// `✓ {masked}` masked-key fragment — green when unselected.
    Key,
    /// ` [{source}]` provenance tag — dim gray when unselected.
    Dim,
    /// `(not configured)` / `(not set)` empty state — gray when unselected.
    Gray,
}

impl SegmentRole {
    fn fg(self, selected: bool) -> Color {
        if selected {
            return Color::Black;
        }
        match self {
            SegmentRole::Name => Color::White,
            SegmentRole::Key => Color::Green,
            SegmentRole::Dim => Color::DarkGray,
            SegmentRole::Gray => Color::Gray,
        }
    }
}

/// One styled fragment of a provider/api-key row label.
#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub role: SegmentRole,
}

impl Segment {
    pub fn new(text: impl Into<String>, role: SegmentRole) -> Self {
        Self {
            text: text.into(),
            role,
        }
    }
}

/// Paint a `prefix` + per-segment-coloured label into `area` (a single row).
/// Returns the absolute buffer column AFTER the last painted text cell (so the
/// RPC-158 inline test-result decoration can append after it), clamped to the
/// row's right boundary.
///
/// * The whole row is first filled with the band style: `bg = band`, plus
///   `Modifier::BOLD` when `selected` (matching `row_render::row_style`).
/// * The `prefix` (selection marker + expand glyph / indent + icon) is painted
///   with the band foreground (white when unselected, black when selected).
/// * Each `Segment` is then painted with its own foreground, on the band bg.
/// * Finally the band style is re-applied to every cell's bg/modifier so
///   wide-glyph continuation cells keep the band (preserving the
///   `row_render.rs:148-150` repair behaviour), without disturbing the
///   per-segment foregrounds already written.
pub fn render_segmented_row(
    prefix: &str,
    segments: &[Segment],
    selected: bool,
    band_bg: Color,
    area: Rect,
    buf: &mut Buffer,
) -> u16 {
    if area.height == 0 || area.width == 0 {
        return area.x;
    }
    let bold = if selected {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };
    let band_style = Style::default().bg(band_bg).add_modifier(bold);

    // 1. Pre-fill the full row with the band so the colour band spans the
    //    entire width even past the label text.
    for x in area.x..area.x + area.width {
        let cell = &mut buf[(x, area.y)];
        cell.set_symbol(" ");
        cell.set_style(band_style);
    }

    // 2. Prefix foreground: white on unselected, black on selected (the
    //    provider parent-row convention — see row_render::row_style).
    let prefix_fg = if selected { Color::Black } else { Color::White };
    let prefix_style = band_style.fg(prefix_fg);
    let right = area.x.saturating_add(area.width);
    let remaining = right.saturating_sub(area.x) as usize;
    let (mut cursor_x, _) = buf.set_stringn(area.x, area.y, prefix, remaining, prefix_style);

    // 3. Each segment with its own foreground over the band bg.
    for seg in segments {
        if cursor_x >= right {
            break;
        }
        let seg_style = band_style.fg(seg.role.fg(selected));
        let remaining = right.saturating_sub(cursor_x) as usize;
        let (end_x, _) = buf.set_stringn(cursor_x, area.y, &seg.text, remaining, seg_style);
        cursor_x = end_x;
    }

    // 4. Band repair: re-apply bg + modifier to every cell WITHOUT clobbering
    //    the per-segment foregrounds. `Cell::set_bg` + modifier insert leave
    //    fg/symbol intact, fixing wide-glyph continuation cells whose style
    //    `set_stringn` reset.
    for x in area.x..area.x + area.width {
        let cell = &mut buf[(x, area.y)];
        cell.set_bg(band_bg);
        cell.modifier.insert(bold);
    }

    cursor_x.min(right)
}
