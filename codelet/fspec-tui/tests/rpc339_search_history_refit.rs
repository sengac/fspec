//! RPC-339 — SearchHistoryView refit onto the shared full-screen shell.
//!
//! Feature: spec/features/search-history-shell-refit.feature
//!
//! These tests pin the SearchHistoryView render contract AFTER the refit
//! onto `render_full_screen_scaffold_with_title`: the editable-query
//! title (`(search): <query>` + inverse cursor), the body match list /
//! placeholder, and the static footer must all render identically to the
//! pre-refit baseline. Validation is by buffer-walking (no insta
//! snapshots cover SearchHistory).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::SearchHistoryView;
use codelet_rpc_types::{HistoryMatch, SessionId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

mod common;

fn hmatch(text: &str) -> HistoryMatch {
    HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: text.to_string(),
        timestamp_iso: "2026-05-25T00:00:00Z".to_string(),
    }
}

/// Read the cells of row `y` from `x = 0` for `len` columns and join them.
fn read_row(buf: &Buffer, y: u16, len: u16) -> String {
    let mut acc = String::new();
    for x in 0..len.min(buf.area.width) {
        acc.push_str(buf[(x, y)].symbol());
    }
    acc
}

/// Locate the (col, row) where the row paints `row_substring`, matching
/// on the first character. Mirrors the helper in `search_view_rpc064.rs`.
fn find_first_col_of(
    buf: &Buffer,
    row_substring: &str,
    needle_first_char: char,
) -> Option<(u16, u16)> {
    let needle_buf: String = needle_first_char.to_string();
    for y in 0..buf.area.height {
        for x_start in 0..buf.area.width {
            if buf[(x_start, y)].symbol() != needle_buf {
                continue;
            }
            let mut acc = String::new();
            let mut x_walk = x_start;
            while x_walk < buf.area.width && acc.len() < row_substring.len() {
                acc.push_str(buf[(x_walk, y)].symbol());
                x_walk += 1;
            }
            if acc.starts_with(row_substring) {
                return Some((x_start, y));
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Refit preserves the editable query title
// ─────────────────────────────────────────────────────────────────────

#[test]
fn refit_preserves_editable_query_title() {
    // @step Given a SearchHistoryView whose query is "auth"
    let mut view = SearchHistoryView::new();
    view.set_query("auth");

    // @step When the view is rendered through the shell title-closure variant
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // @step Then the title row reads "(search): auth"
    let title_row = read_row(&buf, 0, 20);
    assert!(
        title_row.starts_with("(search): auth"),
        "title row 0 actually reads: {title_row:?}"
    );

    // @step And an inverse REVERSED cursor cell is painted immediately after the query
    // "(search): auth".len() == 14 -> the cursor block sits at column 14.
    let cursor_cell = &buf[(14u16, 0u16)];
    assert!(
        cursor_cell.modifier.contains(Modifier::REVERSED),
        "expected REVERSED cursor cell at col=14 row=0 ({:?})",
        cursor_cell.symbol()
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Refit preserves the body match list rendering
// ─────────────────────────────────────────────────────────────────────

#[test]
fn refit_preserves_body_match_list() {
    // @step Given a SearchHistoryView with matching history entries and a selected row
    let mut view = SearchHistoryView::new();
    view.set_query("git");
    view.set_matches(vec![hmatch("git status now"), hmatch("git push origin")]);

    // @step When the view is rendered through the shell title-closure variant
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // @step Then the body renders the scroll-windowed match list
    let (g_col, g_row) =
        find_first_col_of(&buf, "git status now", 'g').expect("must find the first matched row");
    assert!(
        g_row >= 2,
        "match rows must paint below the title/separator chrome"
    );

    // @step And the selected row is painted with the REVERSED modifier
    // The selected row (index 0) prefix marker ' ▸ ' is painted with REVERSED.
    let marker_cell = &buf[(g_col.saturating_sub(2), g_row)];
    assert!(
        marker_cell.modifier.contains(Modifier::REVERSED),
        "expected the selected row to be REVERSED ({:?})",
        marker_cell.symbol()
    );

    // @step And the query substring is BOLD-highlighted within each matching row
    for offset in 0..3u16 {
        let cell = &buf[(g_col + offset, g_row)];
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "expected the 'git' substring cell at offset {offset} ({:?}) to be BOLD",
            cell.symbol()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Refit preserves the empty-query placeholder and footer
// ─────────────────────────────────────────────────────────────────────

#[test]
fn refit_preserves_empty_placeholder_and_footer() {
    // @step Given a SearchHistoryView with an empty query
    let view = SearchHistoryView::new();

    // @step When the view is rendered through the shell title-closure variant
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // @step Then the body shows the centered placeholder "(type to search history)"
    assert!(
        find_first_col_of(&buf, "(type to search history)", '(').is_some(),
        "body must show the empty-query placeholder"
    );

    // @step And the footer row reads "Enter Select | ↑↓ Navigate | Esc Cancel"
    let footer_row = read_row(&buf, area.height - 1, area.width);
    assert!(
        footer_row.contains("Enter Select | ↑↓ Navigate | Esc Cancel"),
        "footer row actually reads: {footer_row:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Source-shape test accepts the refit delegate
// ─────────────────────────────────────────────────────────────────────

fn fspec_tui_src() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

#[test]
fn search_history_render_delegates_to_shell() {
    // @step Given SearchHistoryView::render delegates to render_full_screen_scaffold_with_title as its first statement
    let search = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("search_history_view.rs");
    let body = common::read_to_string_or_panic(&search);

    // @step When the source-shape test in tests/rpc026_source_shape.rs runs
    let render_idx = body
        .find("pub fn render(&self, area: Rect, buf: &mut Buffer)")
        .expect("search render fn");
    let after = &body[render_idx..];
    let brace_idx = after.find('{').expect("opening brace");
    let trimmed = after[brace_idx + 1..].trim_start();

    // @step Then the relaxed first-statement assertion accepts the shell delegate
    assert!(
        trimmed.starts_with("render_full_screen_scaffold_with_title")
            || trimmed.starts_with(
                "crate::views::full_screen_shell::render_full_screen_scaffold_with_title"
            ),
        "search render fn first stmt must delegate to the title-closure shell variant; got: {}",
        &trimmed[..trimmed.len().min(80)]
    );

    // @step And search_history_view.rs remains under 300 lines of code
    assert!(body.lines().count() < 300);
}
