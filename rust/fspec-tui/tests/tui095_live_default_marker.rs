//! TUI-095 — Live-update (default) marker when D pressed in Rust /thinking dialog.
//!
//! Feature: spec/features/thinking-dialog-live-default-marker.feature
//!
//! Drives ThinkingLevelDialog::handle_event with crossterm KeyCode events and
//! asserts the rendered 80x24 buffer moves the (default) marker to the row the
//! user pressed D on (live, no reopen) — mirroring the TS ThinkingLevelDialog.tsx
//! onSetDefault + AgentView defaultLevel re-render loop.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use codelet_fspec_tui::components::thinking_level_dialog::ThinkingLevelDialog;
use codelet_fspec_tui::components::{Action, Component, EventResult};
use codelet_rpc_types::{SessionId, ThinkingLevel};

fn render_component_80x24<C: Component>(component: &mut C) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            component.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

#[allow(dead_code)]
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

/// First (x, y) where `needle` appears on a single row.
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

/// The full text of the row (y) that contains `label`.
fn row_containing(buf: &Buffer, label: &str) -> String {
    let (_, y) = find_text(buf, label).unwrap_or_else(|| panic!("{label} present"));
    let mut row = String::new();
    for x in 0..buf.area.width {
        row.push_str(buf[(x, y)].symbol());
    }
    row
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn dialog(current: ThinkingLevel, default: Option<ThinkingLevel>) -> ThinkingLevelDialog {
    ThinkingLevelDialog::new(SessionId::new("tui095"), current).with_default_level(default)
}

/// Count how many rendered rows contain the substring "(default)".
fn default_marker_row_count(buf: &Buffer) -> usize {
    let mut count = 0usize;
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        if row.contains("(default)") {
            count += 1;
        }
    }
    count
}

/// Scenario: Pressing D moves the (default) marker to the selected row live
#[test]
fn pressing_d_moves_default_marker_to_selected_row_live() {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level Medium
    let mut d = dialog(ThinkingLevel::Off, Some(ThinkingLevel::Medium));
    // @step And I navigate the selection down to the High row
    // Off (index 0) -> High (index 3): 3 Downs
    let _ = d.handle_event(&key_event(KeyCode::Down));
    let _ = d.handle_event(&key_event(KeyCode::Down));
    let _ = d.handle_event(&key_event(KeyCode::Down));
    assert_eq!(d.selected_level(), ThinkingLevel::High);
    // @step When I send a KeyCode::Char('d') event
    let _ = d.handle_event(&key_event(KeyCode::Char('d')));
    // @step And I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d);
    // @step Then the High row reads "(default)"
    let high_row = row_containing(&buf, "High");
    assert!(
        high_row.contains("(default)"),
        "High row must read (default) after pressing D, got {high_row:?}"
    );
    // @step And the Medium row no longer reads "(default)"
    let medium_row = row_containing(&buf, "Medium");
    assert!(
        !medium_row.contains("(default)"),
        "Medium row must no longer read (default), got {medium_row:?}"
    );
}

/// Scenario: Pressing D sets the marker when no default was previously set
#[test]
fn pressing_d_sets_marker_when_no_default_was_set() {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level None
    let mut d = dialog(ThinkingLevel::Off, None);
    // @step And I navigate the selection down to the Low row
    // Off (index 0) -> Low (index 1): 1 Down
    let _ = d.handle_event(&key_event(KeyCode::Down));
    assert_eq!(d.selected_level(), ThinkingLevel::Low);
    // @step When I send a KeyCode::Char('d') event
    let _ = d.handle_event(&key_event(KeyCode::Char('d')));
    // @step And I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d);
    // @step Then the Low row reads "(default)"
    let low_row = row_containing(&buf, "Low");
    assert!(
        low_row.contains("(default)"),
        "Low row must read (default) after pressing D, got {low_row:?}"
    );
}

/// Scenario: Pressing D on the row that is already default is idempotent
#[test]
fn pressing_d_on_already_default_row_is_idempotent() {
    // @step Given a ThinkingLevelDialog seeded with current level High and default level High
    let mut d = dialog(ThinkingLevel::High, Some(ThinkingLevel::High));
    // @step When I send a KeyCode::Char('d') event
    let _ = d.handle_event(&key_event(KeyCode::Char('d')));
    // @step And I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d);
    // @step Then the High row reads "(default)"
    let high_row = row_containing(&buf, "High");
    assert!(
        high_row.contains("(default)"),
        "High row must still read (default), got {high_row:?}"
    );
    // @step And no other row reads "(default)"
    assert_eq!(
        default_marker_row_count(&buf),
        1,
        "exactly one row (the High row) must read (default)"
    );
}

/// Scenario: Navigating after D keeps the marker on the chosen row while only the highlight moves
#[test]
fn navigating_after_d_keeps_marker_on_chosen_row() {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level Medium
    let mut d = dialog(ThinkingLevel::Off, Some(ThinkingLevel::Medium));
    // @step And I navigate the selection down to the High row
    // Off (index 0) -> High (index 3): 3 Downs
    let _ = d.handle_event(&key_event(KeyCode::Down));
    let _ = d.handle_event(&key_event(KeyCode::Down));
    let _ = d.handle_event(&key_event(KeyCode::Down));
    assert_eq!(d.selected_level(), ThinkingLevel::High);
    // @step And I send a KeyCode::Char('d') event
    let _ = d.handle_event(&key_event(KeyCode::Char('d')));
    // @step When I send a KeyCode::Down event to move the selection to the Off row
    // High (index 3) + 1 wraps to index 0 = Off
    let _ = d.handle_event(&key_event(KeyCode::Down));
    assert_eq!(d.selected_level(), ThinkingLevel::Off);
    // @step And I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d);
    // @step Then the Off row begins with the "▸" selection marker
    let (off_x, off_y) = find_text(&buf, "Off").expect("Off row present");
    // The marker sits two cells before the label start (mirrors tui094/rpc027).
    let marker_x = off_x - 2;
    assert_eq!(
        buf[(marker_x, off_y)].symbol(),
        "▸",
        "Off row must begin with the ▸ selection marker"
    );
    // @step And the High row still reads "(default)"
    let high_row = row_containing(&buf, "High");
    assert!(
        high_row.contains("(default)"),
        "High row must still carry (default) after navigating away, got {high_row:?}"
    );
}

/// Scenario: Pressing D emits SetThinkingLevelDefault and keeps the dialog open
#[test]
fn pressing_d_emits_set_default_and_keeps_dialog_open() {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level None
    let mut d = dialog(ThinkingLevel::Off, None);
    // @step And I navigate the selection down to the Medium row
    // Off (index 0) -> Medium (index 2): 2 Downs
    let _ = d.handle_event(&key_event(KeyCode::Down));
    let _ = d.handle_event(&key_event(KeyCode::Down));
    assert_eq!(d.selected_level(), ThinkingLevel::Medium);
    // @step When I send a KeyCode::Char('d') event
    let result = d.handle_event(&key_event(KeyCode::Char('d')));
    // @step Then the dialog emits Action::SetThinkingLevelDefault with the Medium level
    let action = d
        .take_pending_action()
        .expect("D must emit a pending action");
    match action {
        Action::SetThinkingLevelDefault(_sid, level) => {
            assert_eq!(level, ThinkingLevel::Medium);
        }
        other => panic!("expected SetThinkingLevelDefault, got {other:?}"),
    }
    // @step And handle_event returns EventResult::Consumed without closing the dialog
    match result {
        EventResult::Consumed(None) => {}
        EventResult::Consumed(Some(_)) => {
            panic!("D must NOT supply a remove-callback — dialog stays open")
        }
        EventResult::Ignored(_) => panic!("D must be consumed"),
    }
}
