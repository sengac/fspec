//! RPC-351 — selection styling constants + helpers shared by the
//! full-screen ModelSelector row/header renderers.
//!
//! Feature: spec/features/model-selector-selection-style.feature
//!
//! Mirrors the TS reference (`ModelSelectorView.tsx`) and the
//! parity-correct `/provider` view: a selected row paints a solid
//! `Color::Cyan` background band with `Color::Black` foreground (NOT
//! terminal reverse-video), filled edge-to-edge, with a `> ` selection
//! marker. Every inline coloured token flips to `Color::Black` when its
//! row is selected.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// Selection marker painted on a selected row (`> ` + trailing space),
/// matching `provider_settings/icons.rs:31`.
pub(super) const SEL: &str = "> ";

/// Placeholder painted on an unselected row (two spaces — same width as
/// `SEL` so columns stay aligned).
pub(super) const NOSEL: &str = "  ";

/// The base row style: a uniform cyan band (`bg=Cyan, fg=Black`) when
/// selected, otherwise a `fg=White` label style. Uniform cyan for BOTH
/// header and model rows — the per-kind tint scheme `/provider` uses is
/// NOT adopted here (TS parity: `color={isSelected ? 'black' : 'white'}`).
pub(super) fn base_style(is_selected: bool) -> Style {
    if is_selected {
        Style::default().bg(Color::Cyan).fg(Color::Black)
    } else {
        Style::default().fg(Color::White)
    }
}

/// Style for an inline coloured token: `Color::Black` on the band when
/// selected, otherwise its own `accent` foreground.
pub(super) fn token_style(is_selected: bool, accent: Color) -> Style {
    if is_selected {
        Style::default().fg(Color::Black)
    } else {
        Style::default().fg(accent)
    }
}

/// Pre-fill the entire row width with `style` so the colour band covers
/// the full row even on rows shorter than `area.width`. Mirrors
/// `provider_settings/row_render.rs:132-136`.
pub(super) fn fill_row(area: Rect, buf: &mut Buffer, style: Style) {
    for x in area.x..area.x + area.width {
        let cell = &mut buf[(x, area.y)];
        cell.set_symbol(" ");
        cell.set_style(style);
    }
}
