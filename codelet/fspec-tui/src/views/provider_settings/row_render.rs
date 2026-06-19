//! RPC-104 — Per-row icons, indents and color coding.
//!
//! Feature: spec/features/rpc104-provider-settings-row-icons-indents-colors.feature
//!
//! Pure widget renderer for a single ProviderSettings nav-item row.
//! Each `RowKind` variant owns its own colour pairing (yellow=provider/
//! api-key, cyan=profile, magenta=oauth-login, green=oauth-status/
//! add-profile). Selected rows invert the row tint into a background
//! band; unselected rows paint the tint as the foreground on the
//! default background.
//!
//! Mirrors the TypeScript visual contract in
//! `src/tui/components/ProviderSettingsPanel.tsx` (lines 569–770).
//!
//! Scope: this module is intentionally pure — no view state, no
//! NavItem coupling. Callers translate `NavItemKind` into `RowKind`
//! and pass the display label in. Inline status decorations
//! (`✓ masked-key`, `(not configured)`, profile-count badge,
//! test-result span) are deferred to follow-up cards
//! (RPC-105, RPC-107, RPC-108, RPC-158).
//!
//! Strict 1-row contract: every paint clamps to a single buffer row
//! and pads to `area.width` so the background band extends across
//! the full row.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::icons;

/// Render-layer projection of `NavItemKind` — six variants, no payload
/// other than provider-row expansion state which drives the ▼/▶ glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Provider header row. `expanded` flips the marker glyph between
    /// `▼` and `▶`.
    Provider { expanded: bool },
    /// Profile row (openai only). Rendered with the 📁 icon.
    Profile,
    /// OAuth-login action row. Rendered with the 🔑 icon.
    OauthLogin,
    /// OAuth-status row (label carries its own glyph — no prefix icon).
    OauthStatus,
    /// API-key edit-entry row. Rendered with the 🔑 icon.
    ApiKey,
    /// Add-profile pseudo-row (openai only). Rendered with the `+`
    /// icon.
    AddProfile,
}

/// Compute the row's foreground/background pair from the visual matrix.
/// Selected rows always invert into `bg = <tint>, fg = Black` and add
/// `Modifier::BOLD`; unselected rows paint the tint as the foreground
/// on the default (`Color::Reset`) background. `Provider` is the only
/// kind whose unselected foreground is `Color::White` rather than its
/// selection tint — that is the parent-of-the-tree convention used by
/// the TS reference (ProviderSettingsPanel.tsx:587-588).
fn row_style(kind: RowKind, selected: bool) -> Style {
    let tint = match kind {
        RowKind::Provider { .. } | RowKind::ApiKey => Color::Yellow,
        RowKind::Profile => Color::Cyan,
        RowKind::OauthLogin => Color::Magenta,
        RowKind::OauthStatus | RowKind::AddProfile => Color::Green,
    };
    if selected {
        Style::default()
            .bg(tint)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        let fg = match kind {
            RowKind::Provider { .. } => Color::White,
            _ => tint,
        };
        Style::default().fg(fg)
    }
}

/// Build the row's pre-label prefix: selection marker + (for provider
/// rows) the ▼/▶ expand glyph, OR (for child rows) the four-space
/// inner indent + kind-specific icon. The label itself is appended
/// after this prefix by the caller.
fn row_prefix(kind: RowKind, selected: bool) -> String {
    let marker = if selected { icons::SEL } else { icons::NOSEL };
    match kind {
        RowKind::Provider { expanded } => {
            let glyph = if expanded {
                icons::EXPANDED
            } else {
                icons::COLLAPSED
            };
            format!("{marker}{glyph}")
        }
        RowKind::Profile => format!("{marker}{}{}", icons::INDENT, icons::FOLDER),
        RowKind::OauthLogin => format!("{marker}{}{}", icons::INDENT, icons::KEY),
        RowKind::ApiKey => format!("{marker}{}{}", icons::INDENT, icons::KEY),
        RowKind::AddProfile => format!("{marker}{}{}", icons::INDENT, icons::PLUS),
        // OAuth-status carries its own glyph inside the label — no
        // icon prefix beyond the inner indent.
        RowKind::OauthStatus => format!("{marker}{}", icons::INDENT),
    }
}

/// Render a single ProviderSettings nav-item row into `buf` at `area`.
///
/// The row is clamped to a single line of `area.width` cells: prefix +
/// label is right-padded with spaces so the entire row carries the
/// kind+selection style (this is how the selection band stretches the
/// full row width, mirroring TS Ink's `<Box width="100%" bgColor=…>`).
///
/// Wide-cell glyphs (`📁`, `🔑`) are positioned by ratatui's
/// `Buffer::set_string` using `unicode-width`, so the first label
/// character of e.g. a profile row lands at display column 9
/// (`"  " + "    " + "📁 " = 2 + 4 + 3 = 9` — emoji glyph occupies the
/// 2-cell slot at indices 6/7, trailing space at 8).
///
/// **Return value (RPC-158):** the absolute buffer column AFTER the
/// last cell of the painted `prefix+label` text. Callers can use this
/// as the x-coordinate to append inline decorations (e.g. test-result
/// span) on top of the row's colour band. The returned column is
/// clamped to `area.x + area.width` and saturates at `u16::MAX`.
pub fn render_row(kind: RowKind, label: &str, selected: bool, area: Rect, buf: &mut Buffer) -> u16 {
    if area.height == 0 || area.width == 0 {
        return area.x;
    }
    let style = row_style(kind, selected);
    let prefix = row_prefix(kind, selected);
    let raw = format!("{prefix}{label}");
    // Pre-fill the entire row with the row style so the colour band
    // covers the full width even on rows shorter than `area.width`.
    for x in area.x..area.x + area.width {
        let cell = &mut buf[(x, area.y)];
        cell.set_symbol(" ");
        cell.set_style(style);
    }
    // Paint the text on top. `set_stringn` clamps to `area.width`
    // display columns and correctly handles unicode-width for wide
    // glyphs (📁/🔑 take two display cells). The returned (x, _) is
    // the column AFTER the last painted display cell — exactly what
    // we want to expose to callers.
    let (end_x, _end_y) = buf.set_stringn(area.x, area.y, &raw, area.width as usize, style);
    // Wide-glyph continuation cells (the second half of 📁/🔑) get
    // reset to default style by `set_stringn`, which would punch a
    // hole in the colour band. Walk the row once more and force the
    // row style on every cell — this leaves symbols intact (it only
    // updates fg/bg/modifier).
    for x in area.x..area.x + area.width {
        buf[(x, area.y)].set_style(style);
    }
    end_x
}

/// RPC-158: returns the row's background tint when `selected` (so the
/// inline test-result decoration can match the colour band) or
/// `Color::Reset` when unselected. Mirrors the bg side of `row_style`.
pub fn row_band_bg(kind: RowKind, selected: bool) -> Color {
    if !selected {
        return Color::Reset;
    }
    match kind {
        RowKind::Provider { .. } | RowKind::ApiKey => Color::Yellow,
        RowKind::Profile => Color::Cyan,
        RowKind::OauthLogin => Color::Magenta,
        RowKind::OauthStatus | RowKind::AddProfile => Color::Green,
    }
}
