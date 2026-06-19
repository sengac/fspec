//! RPC-104 — Per-row icons, indents and color coding (failing red tests).
//!
//! Feature: spec/features/rpc104-provider-settings-row-icons-indents-colors.feature
//!
//! Pure widget tests against the new `row_render` module:
//!
//! - `views::provider_settings::row_render::RowKind` — six-variant enum
//!   mirroring `NavItemKind` for rendering purposes.
//! - `views::provider_settings::row_render::render_row(kind, label,
//!   selected, area, buf)` — paints a single row into the buffer with
//!   the correct selection prefix, inner indent, icon, and color band.
//! - `views::provider_settings::icons` — glyph constants (EXPANDED,
//!   COLLAPSED, FOLDER, KEY, PLUS, INDENT, SEL, NOSEL).
//!
//! Every test uses ratatui's `TestBackend` so no NAPI, no async, no
//! real terminal — runs in <100ms.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::icons;
use codelet_fspec_tui::views::provider_settings::row_render::{render_row, RowKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn paint(kind: RowKind, label: &str, selected: bool, width: u16) -> Buffer {
    let area = Rect {
        x: 0,
        y: 0,
        width,
        height: 1,
    };
    let mut buf = Buffer::empty(area);
    render_row(kind, label, selected, area, &mut buf);
    buf
}

fn cell_symbol(buf: &Buffer, x: u16) -> String {
    buf[(x, 0)].symbol().to_string()
}

fn cell_fg(buf: &Buffer, x: u16) -> Color {
    buf[(x, 0)].fg
}

fn cell_bg(buf: &Buffer, x: u16) -> Color {
    buf[(x, 0)].bg
}

fn cell_modifier(buf: &Buffer, x: u16) -> Modifier {
    buf[(x, 0)].modifier
}

/// Concatenate the first `width` cells into a String for label-text
/// assertions.
fn row_string(buf: &Buffer, width: u16) -> String {
    let mut s = String::new();
    for x in 0..width {
        s.push_str(buf[(x, 0)].symbol());
    }
    s
}

// ────────────────────────────────────────────────────────────────────────
// Selection band — colour matrix
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Selected provider row paints a yellow background band
#[test]
fn selected_provider_row_paints_yellow_band() {
    // @step Given a ProviderSettings row of kind Provider labelled "OpenAI"
    // @step And the row is in the selected state
    let buf = paint(RowKind::Provider { expanded: false }, "OpenAI", true, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then every cell on the row carries bg=Yellow and fg=Black
    for x in 0..40u16 {
        assert_eq!(
            cell_bg(&buf, x),
            Color::Yellow,
            "cell {x} bg should be Yellow on selected provider row"
        );
        assert_eq!(
            cell_fg(&buf, x),
            Color::Black,
            "cell {x} fg should be Black on selected provider row"
        );
    }
    // @step And the row uses Modifier::BOLD
    let any_bold = (0..40u16).any(|x| cell_modifier(&buf, x).contains(Modifier::BOLD));
    assert!(
        any_bold,
        "selected provider row should carry Modifier::BOLD"
    );
}

/// Scenario: Unselected provider row paints white foreground on default background
#[test]
fn unselected_provider_row_paints_white_on_reset() {
    // @step Given a ProviderSettings row of kind Provider labelled "OpenAI"
    // @step And the row is in the unselected state
    let buf = paint(RowKind::Provider { expanded: false }, "OpenAI", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the name span carries fg=White and bg=Reset
    // The name starts at cell 4 (2-cell marker "  " + 1 glyph "▶" + 1 space = 4).
    // We assert the first label character (capital 'O') is fg=White on bg=Reset.
    let first_label_cell = (0..40u16)
        .find(|x| buf[(*x, 0)].symbol() == "O")
        .expect("label 'OpenAI' first 'O' should appear in row");
    assert_eq!(cell_fg(&buf, first_label_cell), Color::White);
    assert_eq!(cell_bg(&buf, first_label_cell), Color::Reset);
    // @step And no Modifier::REVERSED flag is set on the row
    for x in 0..40u16 {
        assert!(
            !cell_modifier(&buf, x).contains(Modifier::REVERSED),
            "cell {x} should not carry Modifier::REVERSED on unselected provider row"
        );
    }
}

/// Scenario: Selected profile row paints a cyan background band
#[test]
fn selected_profile_row_paints_cyan_band() {
    // @step Given a ProviderSettings row of kind Profile labelled "dev"
    // @step And the row is in the selected state
    let buf = paint(RowKind::Profile, "dev", true, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then every cell on the row carries bg=Cyan and fg=Black
    for x in 0..40u16 {
        assert_eq!(cell_bg(&buf, x), Color::Cyan, "cell {x}");
        assert_eq!(cell_fg(&buf, x), Color::Black, "cell {x}");
    }
}

/// Scenario: Selected oauth-login row paints a magenta background band
#[test]
fn selected_oauth_login_row_paints_magenta_band() {
    // @step Given a ProviderSettings row of kind OauthLogin labelled "Sign in to GitHub"
    // @step And the row is in the selected state
    let buf = paint(RowKind::OauthLogin, "Sign in to GitHub", true, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then every cell on the row carries bg=Magenta and fg=Black
    for x in 0..40u16 {
        assert_eq!(cell_bg(&buf, x), Color::Magenta, "cell {x}");
        assert_eq!(cell_fg(&buf, x), Color::Black, "cell {x}");
    }
}

/// Scenario: Selected oauth-status row paints a green background band
#[test]
fn selected_oauth_status_row_paints_green_band() {
    // @step Given a ProviderSettings row of kind OauthStatus labelled "✓ Signed in as user"
    // @step And the row is in the selected state
    let buf = paint(RowKind::OauthStatus, "✓ Signed in as user", true, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then every cell on the row carries bg=Green and fg=Black
    for x in 0..40u16 {
        assert_eq!(cell_bg(&buf, x), Color::Green, "cell {x}");
        assert_eq!(cell_fg(&buf, x), Color::Black, "cell {x}");
    }
}

/// Scenario: Selected add-profile row paints a green background band
#[test]
fn selected_add_profile_row_paints_green_band() {
    // @step Given a ProviderSettings row of kind AddProfile labelled "Add Profile"
    // @step And the row is in the selected state
    let buf = paint(RowKind::AddProfile, "Add Profile", true, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then every cell on the row carries bg=Green and fg=Black
    for x in 0..40u16 {
        assert_eq!(cell_bg(&buf, x), Color::Green, "cell {x}");
        assert_eq!(cell_fg(&buf, x), Color::Black, "cell {x}");
    }
}

/// Scenario: Selected api-key row paints a yellow background band
#[test]
fn selected_api_key_row_paints_yellow_band() {
    // @step Given a ProviderSettings row of kind ApiKey labelled "API Key"
    // @step And the row is in the selected state
    let buf = paint(RowKind::ApiKey, "API Key", true, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then every cell on the row carries bg=Yellow and fg=Black
    for x in 0..40u16 {
        assert_eq!(cell_bg(&buf, x), Color::Yellow, "cell {x}");
        assert_eq!(cell_fg(&buf, x), Color::Black, "cell {x}");
    }
}

/// Scenario: Unselected child rows are tinted by their kind on the default background
#[test]
fn unselected_profile_row_paints_cyan_on_reset() {
    // @step Given a ProviderSettings row of kind Profile labelled "dev"
    // @step And the row is in the unselected state
    let buf = paint(RowKind::Profile, "dev", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the label span carries fg=Cyan and bg=Reset
    let label_cell = (0..40u16)
        .find(|x| buf[(*x, 0)].symbol() == "d")
        .expect("profile label 'dev' should appear in row");
    assert_eq!(cell_fg(&buf, label_cell), Color::Cyan);
    assert_eq!(cell_bg(&buf, label_cell), Color::Reset);
}

// ────────────────────────────────────────────────────────────────────────
// Indents, prefixes, and glyphs
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Every non-provider row prepends a 4-space inner indent after the selection prefix
#[test]
fn child_rows_have_four_space_inner_indent() {
    // @step Given a ProviderSettings row of kind Profile labelled "dev"
    let buf = paint(RowKind::Profile, "dev", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then cells at indices 0 and 1 are the selection prefix "  "
    assert_eq!(cell_symbol(&buf, 0), " ");
    assert_eq!(cell_symbol(&buf, 1), " ");
    // @step And cells at indices 2, 3, 4, and 5 are spaces forming the inner indent
    for x in 2u16..=5 {
        assert_eq!(
            cell_symbol(&buf, x),
            " ",
            "cell {x} should be space (inner indent)"
        );
    }
}

/// Scenario: Provider rows have no 4-space inner indent — the expand glyph follows the marker directly
#[test]
fn provider_row_has_no_inner_indent() {
    // @step Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=true
    let buf = paint(RowKind::Provider { expanded: true }, "OpenAI", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then cells at indices 0 and 1 are the selection prefix "  "
    assert_eq!(cell_symbol(&buf, 0), " ");
    assert_eq!(cell_symbol(&buf, 1), " ");
    // @step And cell at index 2 is the expanded glyph "▼"
    assert_eq!(cell_symbol(&buf, 2), "▼");
    // @step And cell at index 2 is NOT a space
    assert_ne!(cell_symbol(&buf, 2), " ");
}

/// Scenario: Provider row paints the ▼ expanded glyph when expanded is true
#[test]
fn provider_row_paints_expanded_glyph_when_expanded() {
    // @step Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=true
    let buf = paint(RowKind::Provider { expanded: true }, "OpenAI", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the expand glyph at cell index 2 is "▼"
    assert_eq!(cell_symbol(&buf, 2), "▼");
}

/// Scenario: Provider row paints the ▶ collapsed glyph when expanded is false
#[test]
fn provider_row_paints_collapsed_glyph_when_collapsed() {
    // @step Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=false
    let buf = paint(RowKind::Provider { expanded: false }, "OpenAI", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the expand glyph at cell index 2 is "▶"
    assert_eq!(cell_symbol(&buf, 2), "▶");
}

/// Scenario: Selected row prefix is "> " and unselected prefix is "  "
#[test]
fn selection_prefix_flips_between_gt_and_spaces() {
    // @step Given a ProviderSettings row of kind Provider labelled "OpenAI" with expanded=false
    // @step When the row is painted selected into a TestBackend buffer of width 40
    let sel = paint(RowKind::Provider { expanded: false }, "OpenAI", true, 40);
    // @step Then cells at indices 0 and 1 are "> "
    assert_eq!(cell_symbol(&sel, 0), ">");
    assert_eq!(cell_symbol(&sel, 1), " ");
    // @step When the same row is painted unselected into a TestBackend buffer of width 40
    let unsel = paint(RowKind::Provider { expanded: false }, "OpenAI", false, 40);
    // @step Then cells at indices 0 and 1 are "  "
    assert_eq!(cell_symbol(&unsel, 0), " ");
    assert_eq!(cell_symbol(&unsel, 1), " ");
}

/// Scenario: Profile row carries the 📁 folder icon directly after the inner indent
#[test]
fn profile_row_has_folder_icon_at_index_6() {
    // @step Given a ProviderSettings row of kind Profile labelled "dev"
    let buf = paint(RowKind::Profile, "dev", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the icon cell at index 6 starts with "📁"
    assert_eq!(cell_symbol(&buf, 6), "📁");
}

/// Scenario: OauthLogin row carries the 🔑 key icon directly after the inner indent
#[test]
fn oauth_login_row_has_key_icon_at_index_6() {
    // @step Given a ProviderSettings row of kind OauthLogin labelled "Sign in to GitHub"
    let buf = paint(RowKind::OauthLogin, "Sign in to GitHub", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the icon cell at index 6 starts with "🔑"
    assert_eq!(cell_symbol(&buf, 6), "🔑");
}

/// Scenario: AddProfile row carries the "+ " glyph directly after the inner indent
#[test]
fn add_profile_row_has_plus_icon_at_index_6() {
    // @step Given a ProviderSettings row of kind AddProfile labelled "Add Profile"
    let buf = paint(RowKind::AddProfile, "Add Profile", false, 40);
    // @step When the row is painted into a TestBackend buffer of width 40
    // @step Then the icon cell at index 6 is "+"
    assert_eq!(cell_symbol(&buf, 6), "+");
}

// ────────────────────────────────────────────────────────────────────────
// icons module — glyph constants
// ────────────────────────────────────────────────────────────────────────

/// Sanity: the icons module exports the canonical glyph constants used
/// by row_render. These are referenced from list.rs and other render
/// sites and must stay stable.
#[test]
fn icons_module_exposes_canonical_glyphs() {
    assert_eq!(icons::EXPANDED, "▼ ");
    assert_eq!(icons::COLLAPSED, "▶ ");
    assert_eq!(icons::FOLDER, "📁 ");
    assert_eq!(icons::KEY, "🔑 ");
    assert_eq!(icons::PLUS, "+ ");
    assert_eq!(icons::INDENT, "    ");
    assert_eq!(icons::SEL, "> ");
    assert_eq!(icons::NOSEL, "  ");
}

/// Sanity check on full row text: an unselected expanded OpenAI provider
/// row should start with "  ▼ OpenAI" — proves the prefix → glyph →
/// space → label ordering is wired correctly even before colours are
/// inspected.
#[test]
fn provider_row_string_layout_is_canonical() {
    let buf = paint(RowKind::Provider { expanded: true }, "OpenAI", false, 40);
    let s = row_string(&buf, 12);
    assert!(
        s.starts_with("  ▼ OpenAI"),
        "row layout should start with '  ▼ OpenAI', got: {s:?}"
    );
}
