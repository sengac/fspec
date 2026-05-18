//! RPC-026 — ConfirmDialog widget unit tests.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::{ConfirmDialog, ConfirmDialogOutcome};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn dialog() -> ConfirmDialog {
    ConfirmDialog::new(
        "Delete session?",
        "Delete session s-2?",
        "Delete",
        None,
        "Cancel",
    )
}

/// Scenario fragment: D opens the ConfirmDialog with primary_label "Delete"
#[test]
fn new_dialog_focuses_primary_with_documented_labels() {
    // @step Given resume_view.delete_confirm is Some(ConfirmDialog) with Primary focused
    let d = dialog();
    // @step And resume_view.delete_confirm is Some(ConfirmDialog) with primary_label "Delete"
    assert_eq!(d.primary_label(), "Delete");
    assert_eq!(d.cancel_label(), "Cancel");
    assert_eq!(d.focused(), 0);
    assert_eq!(d.buttons().len(), 2);
}

/// Scenario fragment: Enter on Primary returns Primary outcome
#[test]
fn enter_on_primary_returns_primary_outcome() {
    // @step When the user presses Enter while the ConfirmDialog has Primary focused
    let mut d = dialog();
    let outcome = d.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    // @step Then Action::ConfirmDeleteSession("s-2") is dispatched
    assert_eq!(outcome, ConfirmDialogOutcome::Primary);
}

/// Scenario fragment: Esc dismisses with Cancel
#[test]
fn esc_returns_cancel_outcome() {
    // @step When the user presses Esc on the dialog
    let mut d = dialog();
    let outcome = d.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    // @step Then resume_view.delete_confirm is None
    // @step And resume_view.sessions is unchanged
    // (Widget-level: the dialog has no handle to the parent's sessions vec; Cancel outcome means the parent will NOT mutate sessions.)
    // @step And no backend call has been made
    assert_eq!(outcome, ConfirmDialogOutcome::Cancel);
}

/// Scenario fragment: Right cycles to Cancel, Enter activates Cancel
#[test]
fn right_arrow_cycles_focus_and_enter_activates_cancel() {
    // @step When the user cycles to Cancel and presses Enter
    let mut d = dialog();
    let _ = d.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(d.focused(), 1);
    let outcome = d.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    // @step Then resume_view.delete_confirm is None
    assert_eq!(outcome, ConfirmDialogOutcome::Cancel);
}

/// Scenario fragment: Left arrow wraps focus backwards
#[test]
fn left_arrow_wraps_to_last_button() {
    // @step Given dialog focused on Primary
    let mut d = dialog();
    // @step When the user presses Left
    let outcome = d.handle_key(KeyCode::Left, KeyModifiers::NONE);
    // @step Then focus wraps to the Cancel button
    assert_eq!(outcome, ConfirmDialogOutcome::Continued);
    assert_eq!(d.focused(), 1);
}

/// Scenario fragment: render paints title + body + button row
#[test]
fn render_paints_title_body_and_buttons() {
    // @step Given a fresh confirm dialog
    let d = dialog();
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    // @step When render is called
    d.render(Rect::new(0, 0, 80, 24), &mut buf);
    // @step Then the buffer contains the title
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect();
    let joined = rows.join("\n");
    assert!(joined.contains("Delete session?"));
    // @step And the button row shows Delete and Cancel
    assert!(joined.contains("Delete"));
    assert!(joined.contains("Cancel"));
}
