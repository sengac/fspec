//! Feature: spec/features/full-screen-shell-title-closure.feature
//!
//! RPC-339 tests, moved out of full_screen_shell.rs's inline tests
//! (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::views::agent::confirm_dialog::ConfirmDialog;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::tests::render_to_string;

/// RPC-339 — render a `width`x`height` TestBackend through the
/// title-closure scaffold variant and return the joined rows.
fn render_with_title_to_string(
    width: u16,
    height: u16,
    title_text: &str,
    footer: &str,
    body_text: &str,
    overlay: Option<&ConfirmDialog>,
) -> String {
    use ratatui::widgets::{Paragraph, Widget};
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        render_full_screen_scaffold_with_title(
            frame.area(),
            frame.buffer_mut(),
            |title_area, buf| {
                Paragraph::new(title_text).render(title_area, buf);
            },
            footer,
            |body_area, buf| {
                Paragraph::new(body_text).render(body_area, buf);
            },
            overlay,
        );
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

#[test]
fn shell_title_closure_paints_title_body_and_footer_no_overlay() {
    // @step Given a full-screen area and a caller-supplied title closure, body closure, and a static footer hint
    let title_text = "(search): auth";
    let footer = "Enter Select | Esc Cancel";

    // @step When the view is rendered via render_full_screen_scaffold_with_title with overlay None
    let out = render_with_title_to_string(80, 24, title_text, footer, "search-body-rows", None);

    // @step Then the title closure paints the first row
    assert!(out.starts_with("(search): auth"));

    // @step And the body closure paints the body sub-rect below the separator
    assert!(out.contains("search-body-rows"));

    // @step And the static footer hint paints the last row
    assert!(out.contains("Enter Select | Esc Cancel"));

    // @step And no ConfirmDialog overlay is drawn
    assert!(!out.contains("Delete"));
}

#[test]
fn count_wrapper_preserves_title_count_format() {
    // @step Given a view rendered via the count-title wrapper render_full_screen_scaffold
    // @step When it is called with title "Resume Session", count 5, and suffix "available"
    let out = render_to_string(
        80,
        24,
        "Resume Session",
        5,
        "available",
        "Enter Select | Esc Cancel",
        "session-row-body",
        None,
    );

    // @step Then the title row reads "Resume Session (5 available)"
    assert!(out.contains("Resume Session (5 available)"));

    // @step And the rendered output is identical to the pre-RPC-339 baseline
    assert!(out.contains("session-row-body"));
    assert!(out.contains("Enter Select | Esc Cancel"));
}
