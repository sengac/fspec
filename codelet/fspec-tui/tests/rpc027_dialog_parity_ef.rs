//! RPC-027 — Tests for ModelSelectorDialog + ConfirmDialog parity.
//!
//! Feature: spec/features/rpc027-model-confirm-dialogs.feature
//! Covers Sections E (ModelSelectorDialog) and F (ConfirmDialog).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use codelet_fspec_tui::views::agent::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};

fn render_confirm_80x24(c: &ConfirmDialog) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|f| c.render(f.area(), f.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn find_text(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() {
        return None;
    }
    for y in 0..buf.area.height {
        let row: Vec<char> = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
            .collect();
        for start in 0..row.len() {
            if start + needle_chars.len() > row.len() {
                break;
            }
            if row[start..start + needle_chars.len()] == needle_chars[..] {
                return Some((start as u16, y));
            }
        }
    }
    None
}

fn find_border_color(buf: &Buffer) -> Color {
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].symbol() == "╭" {
                return buf[(x, y)].fg;
            }
        }
    }
    Color::Reset
}

fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

// ============================================================
// Section F — ConfirmDialog
// ============================================================

fn make_confirm() -> ConfirmDialog {
    ConfirmDialog::new(
        "Delete Session",
        "Delete this session?",
        "Delete",
        Some("Archive".to_string()),
        "Cancel",
    )
}

/// Scenario: ConfirmDialog renders with the yellow accent and caller-supplied title
#[test]
fn confirm_dialog_renders_with_yellow_accent_and_caller_supplied_title() {
    // @step Given a ConfirmDialog with title "Delete Session" and body "Delete this session?"
    let dialog = make_confirm();
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_confirm_80x24(&dialog);
    // @step Then the border cells use foreground color Color::Yellow
    assert_eq!(find_border_color(&buf), Color::Yellow);
    // @step And the body's first non-padding row contains the text "Delete Session"
    let (x, y) = find_text(&buf, "Delete Session").expect("title present");
    // @step And the title cells have foreground color Color::Yellow with BOLD modifier
    for i in 0..("Delete Session".chars().count() as u16) {
        let cell = &buf[(x + i, y)];
        assert_eq!(cell.fg, Color::Yellow);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }
    // @step And the source no longer uses Block::default().borders(Borders::ALL)
    let src = std::fs::read_to_string("src/views/agent/confirm_dialog.rs")
        .expect("confirm_dialog.rs exists");
    assert!(
        !src.contains("Block::default()") || !src.contains("Borders::ALL"),
        "ConfirmDialog must no longer use raw Block::default().borders(Borders::ALL)"
    );
}

/// Scenario: ConfirmDialog button row uses inverse highlight on the focused button
#[test]
fn confirm_dialog_button_row_uses_inverse_highlight_on_focused_button() {
    // @step Given a ConfirmDialog with primary "Delete", secondary "Archive", cancel "Cancel" and focused index 0
    let dialog = make_confirm();
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_confirm_80x24(&dialog);

    // Helper: scan a specific row for `needle` and return the starting
    // column index of the first match (in cells, not bytes).
    fn find_on_row(buf: &Buffer, needle: &str, y: u16) -> Option<u16> {
        let needle_chars: Vec<char> = needle.chars().collect();
        let row: Vec<char> = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
            .collect();
        for start in 0..row.len() {
            if start + needle_chars.len() > row.len() {
                break;
            }
            if row[start..start + needle_chars.len()] == needle_chars[..] {
                return Some(start as u16);
            }
        }
        None
    }

    // Locate the button row: the only row that contains "Archive" (the
    // body has "Delete" too, and every bordered row has " │ " at the
    // borders, so we anchor on the unique button label).
    let mut button_row_y: Option<u16> = None;
    for y in 0..buf.area.height {
        let row: String = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        if row.contains("Archive") {
            button_row_y = Some(y);
            break;
        }
    }
    let by = button_row_y.expect("button row with 'Archive' label");

    let dx = find_on_row(&buf, "Delete", by).expect("Delete on button row");
    let cell = &buf[(dx, by)];
    // @step Then the " Delete " span has background Color::Yellow and foreground Color::Black with BOLD modifier
    assert_eq!(cell.bg, Color::Yellow);
    assert_eq!(cell.fg, Color::Black);
    assert!(cell.modifier.contains(Modifier::BOLD));

    // @step And the " Archive " and " Cancel " spans have default Style
    let arch_x = find_on_row(&buf, "Archive", by).expect("Archive on button row");
    let arch = &buf[(arch_x, by)];
    assert_ne!(arch.bg, Color::Yellow, "Archive must not be highlighted");
    let can_x = find_on_row(&buf, "Cancel", by).expect("Cancel on button row");
    let can = &buf[(can_x, by)];
    assert_ne!(can.bg, Color::Yellow, "Cancel must not be highlighted");

    // @step And the spans are separated by " │ "
    let text = buffer_text(&buf);
    assert!(text.contains(" │ "), "buttons must be separated by ' │ '");
}

/// Scenario: ConfirmDialog Left and Right cycle button focus
#[test]
fn confirm_dialog_left_and_right_cycle_button_focus() {
    // @step Given a ConfirmDialog with three buttons and focused index 0
    let mut dialog = make_confirm();
    assert_eq!(dialog.focused(), 0);
    // @step When I send KeyCode::Right
    let _ = dialog.handle_key(KeyCode::Right, KeyModifiers::NONE);
    // @step Then focused is 1
    assert_eq!(dialog.focused(), 1);
    // @step When I send KeyCode::Right
    let _ = dialog.handle_key(KeyCode::Right, KeyModifiers::NONE);
    // @step Then focused is 2
    assert_eq!(dialog.focused(), 2);
    // @step When I send KeyCode::Right
    let _ = dialog.handle_key(KeyCode::Right, KeyModifiers::NONE);
    // @step Then focused is 0
    assert_eq!(dialog.focused(), 0);
    // @step When I send KeyCode::Left
    let _ = dialog.handle_key(KeyCode::Left, KeyModifiers::NONE);
    // @step Then focused is 2
    assert_eq!(dialog.focused(), 2);
}

/// Scenario: ConfirmDialog Esc returns Cancel from any focused index
#[test]
fn confirm_dialog_esc_returns_cancel_from_any_focused_index() {
    // @step Given a ConfirmDialog with three buttons and focused index 1
    let mut dialog = make_confirm();
    let _ = dialog.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(dialog.focused(), 1);
    // @step When I send KeyCode::Esc
    let outcome = dialog.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    // @step Then the outcome is ConfirmDialogOutcome::Cancel
    assert_eq!(outcome, ConfirmDialogOutcome::Cancel);
}

// Silence unused warnings — area constructor is referenced via Rect below
#[allow(dead_code)]
fn _touch_rect() -> Rect {
    Rect::new(0, 0, 1, 1)
}
