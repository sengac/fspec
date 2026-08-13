//! RPC-396 — Integration tests for the scrollable, space-filling HelpDialog.
//!
//! Feature: spec/features/scrollable-space-filling-help-dialog-with-scrollbar.feature
//!
//! ACDD testing phase: these tests are written FIRST and are expected to
//! FAIL TO COMPILE until the RPC-396 implementation lands, because they
//! require the following NOT-YET-EXISTING public API on `HelpDialog`:
//!
//!   * `HelpDialog::with_lines_for_test(lines: Vec<String>) -> Self`
//!     — inject a deterministic large content vec (independent of the
//!     real HELP_LINES content, which RPC-397 owns).
//!   * `HelpDialog::scroll_offset(&self) -> usize`
//!     — read the current scroll offset for assertions.
//!   * `HelpDialog::visible_rows(&self) -> usize`
//!     — read the measured body height (set during `render`).
//!
//! Scroll/paging/wheel/ESC events are dispatched through the existing
//! `Component::handle_event(&mut self, &Event)` trait method; rendering
//! through `Component::render(&mut self, area, buf)`.
//!
//! Scenarios covered (1:1 with the feature file):
//!   S1. PageDown advances a page when content overflows the terminal
//!   S2. End jumps to the bottom and the thumb sits at the bottom
//!   S3. Mouse wheel scrolls the content and is clamped at the top
//!   S4. No scrollbar and inert paging when all content fits

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use codelet_fspec_tui::components::help_dialog::HelpDialog;
use codelet_fspec_tui::components::Component;

// ─────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────

/// Build a deterministic 40-line content vec. Each line is uniquely
/// identifiable so we can assert exactly which lines are on screen.
fn forty_lines() -> Vec<String> {
    (0..40).map(|i| format!("HELP-LINE-{i:02}")).collect()
}

/// Render `dialog` into a fresh `w x h` TestBackend and return the buffer.
fn render_into(dialog: &mut HelpDialog, w: u16, h: u16) -> Buffer {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            dialog.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

/// Flatten a buffer into a single newline-joined string of glyphs.
fn join_buffer(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn wheel(kind: MouseEventKind) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    })
}

/// True when the buffer contains the scrollbar THUMB glyph (`■`)
/// painted by `render_list_scrollbar`.
///
/// NOTE: we deliberately detect ONLY the `■` thumb, NOT the `│` track.
/// HelpDialog renders inside a rounded dialog border (RPC-027 §B), and
/// that border paints `│` verticals on the left/right body columns for
/// every row. The scrollbar track glyph is also `│`, so matching `│`
/// would be indistinguishable from the border. The `■` thumb is unique
/// to `render_list_scrollbar` and is only ever painted when content
/// overflows — making it the correct, unambiguous scrollbar signal.
fn has_scrollbar_glyph(text: &str) -> bool {
    text.contains('■')
}

// ─────────────────────────────────────────────────────────────────────
// S1 — PageDown advances a page when content overflows the terminal
// ─────────────────────────────────────────────────────────────────────

#[test]
fn s1_pagedown_advances_a_page_when_content_overflows() {
    // @step Given a HelpDialog whose content has 40 lines rendered against an 80x24 TestBackend
    let mut dialog = HelpDialog::with_lines_for_test(forty_lines());
    let page1 = render_into(&mut dialog, 80, 24);
    let page1_text = join_buffer(&page1);

    // @step And the rendered dialog shows the first page with a scrollbar in a reserved gutter
    assert!(
        has_scrollbar_glyph(&page1_text),
        "first page must show a scrollbar glyph in the gutter:\n{page1_text}"
    );
    assert_eq!(
        dialog.scroll_offset(),
        0,
        "first page must start at scroll_offset 0"
    );
    let visible = dialog.visible_rows();
    assert!(
        visible > 0 && visible < 40,
        "visible_rows must be a positive overflow-inducing window, got {visible}"
    );

    // @step When the user presses PageDown
    let _ = dialog.handle_event(&key(KeyCode::PageDown));
    let page2 = render_into(&mut dialog, 80, 24);
    let page2_text = join_buffer(&page2);

    // @step Then the scroll offset advances by one visible page
    assert_eq!(
        dialog.scroll_offset(),
        visible,
        "PageDown must advance scroll_offset by one visible page ({visible})"
    );

    // @step And the rendered buffer shows lines from further down the content
    let deeper_line = format!("HELP-LINE-{visible:02}");
    assert!(
        page2_text.contains(&deeper_line),
        "page 2 must reveal a line further down the content ({deeper_line}):\n{page2_text}"
    );
    assert!(
        !page1_text.contains(&deeper_line),
        "the deeper line {deeper_line} must NOT have been visible on page 1:\n{page1_text}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// S2 — End jumps to the bottom and the thumb sits at the bottom
// ─────────────────────────────────────────────────────────────────────

#[test]
fn s2_end_jumps_to_bottom_and_thumb_sits_at_bottom() {
    // @step Given a HelpDialog whose content has 40 lines rendered against an 80x24 TestBackend
    let mut dialog = HelpDialog::with_lines_for_test(forty_lines());
    let _ = render_into(&mut dialog, 80, 24);
    let visible = dialog.visible_rows();
    assert!(
        visible > 0 && visible < 40,
        "expected an overflowing window, got visible_rows={visible}"
    );

    // @step When the user presses End
    let _ = dialog.handle_event(&key(KeyCode::End));
    let bottom = render_into(&mut dialog, 80, 24);
    let bottom_text = join_buffer(&bottom);

    // @step Then the last content line is visible
    assert!(
        bottom_text.contains("HELP-LINE-39"),
        "End must reveal the last content line HELP-LINE-39:\n{bottom_text}"
    );

    // @step And the scrollbar thumb is drawn at the bottom of the gutter
    assert_eq!(
        dialog.scroll_offset(),
        40 - visible,
        "End must clamp scroll_offset to total - visible_rows ({})",
        40 - visible
    );
    assert!(
        has_scrollbar_glyph(&bottom_text),
        "the scrollbar thumb must be drawn while content overflows:\n{bottom_text}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// S3 — Mouse wheel scrolls the content and is clamped at the top
// ─────────────────────────────────────────────────────────────────────

#[test]
fn s3_mouse_wheel_scrolls_and_clamps_at_top() {
    // @step Given a HelpDialog whose content has 40 lines rendered against an 80x24 TestBackend
    let mut dialog = HelpDialog::with_lines_for_test(forty_lines());
    let _ = render_into(&mut dialog, 80, 24);
    assert_eq!(dialog.scroll_offset(), 0, "starts at the top");

    // @step When the user scrolls the mouse wheel down
    let _ = dialog.handle_event(&wheel(MouseEventKind::ScrollDown));
    let _ = render_into(&mut dialog, 80, 24);

    // @step Then the content scrolls down
    assert!(
        dialog.scroll_offset() > 0,
        "wheel down must increase scroll_offset, got {}",
        dialog.scroll_offset()
    );

    // @step When the user scrolls the mouse wheel up past the top
    for _ in 0..10 {
        let _ = dialog.handle_event(&wheel(MouseEventKind::ScrollUp));
        let _ = render_into(&mut dialog, 80, 24);
    }

    // @step Then the scroll offset stays at zero
    assert_eq!(
        dialog.scroll_offset(),
        0,
        "wheel up past the top must clamp scroll_offset to zero (no underflow)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// S4 — No scrollbar and inert paging when all content fits
// ─────────────────────────────────────────────────────────────────────

#[test]
fn s4_no_scrollbar_and_inert_paging_when_all_content_fits() {
    // @step Given a HelpDialog whose content fits within a 200x60 TestBackend
    let mut dialog = HelpDialog::new();
    let fitted = render_into(&mut dialog, 200, 60);
    let fitted_text = join_buffer(&fitted);

    // @step When the rendered dialog is drawn
    // (rendered above)

    // @step Then no scrollbar gutter is rendered
    assert!(
        !has_scrollbar_glyph(&fitted_text),
        "no scrollbar glyph must be painted when all content fits:\n{fitted_text}"
    );

    // @step And pressing PageDown leaves the scroll offset at zero
    let _ = dialog.handle_event(&key(KeyCode::PageDown));
    let _ = render_into(&mut dialog, 200, 60);
    assert_eq!(
        dialog.scroll_offset(),
        0,
        "PageDown must be inert (scroll_offset stays 0) when all content fits"
    );
}
