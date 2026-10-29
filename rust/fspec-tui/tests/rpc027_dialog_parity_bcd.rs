//! RPC-027 — Tests for migrated dialog parity (Sections B-D).
//!
//! Feature: spec/features/rpc027-help-disconnect-thinking-dialogs.feature
//!
//! Covers:
//!   B. HelpDialog migration (cyan accent, inner title, body content,
//!      no tui_popup import)
//!   C. DisconnectDialog migration (red accent, inner title, body
//!      content, inline Reconnecting update)
//!   D. ThinkingLevelDialog migration (yellow accent, inverse highlight,
//!      D-key, SetThinkingLevelDefault wiring, footer)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use codelet_fspec_tui::components::disconnect_dialog::DisconnectDialog;
use codelet_fspec_tui::components::help_dialog::HelpDialog;
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
    // The top-left corner of the dialog must be ╭ — find it.
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].symbol() == "╭" {
                return buf[(x, y)].fg;
            }
        }
    }
    Color::Reset
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

// ============================================================
// Section B — HelpDialog migration
// ============================================================

/// Scenario: HelpDialog renders with the cyan accent and inner-title body
#[test]
fn help_dialog_renders_with_cyan_accent_and_inner_title_body() {
    // @step Given an isolated HelpDialog component
    let mut dialog = HelpDialog::new();

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);

    // @step Then the border cells use foreground color Color::Cyan
    assert_eq!(
        find_border_color(&buf),
        Color::Cyan,
        "HelpDialog border must be Cyan"
    );

    // @step And the body's first non-padding row contains the text "Help"
    let (help_x, help_y) = find_text(&buf, "Help").expect("'Help' must appear inside body");

    // @step And the "Help" text cells have foreground color Color::Cyan with BOLD modifier
    for i in 0..("Help".chars().count() as u16) {
        let cell = &buf[(help_x + i, help_y)];
        assert_eq!(cell.fg, Color::Cyan, "Help title cell must be Cyan");
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "Help title cell must be BOLD"
        );
    }

    // @step And the top border row does NOT contain the text "Help"
    let mut top_border = String::new();
    let border_y = (0..buf.area.height)
        .find(|y| (0..buf.area.width).any(|x| buf[(x, *y)].symbol() == "╭"))
        .expect("top border row exists");
    for x in 0..buf.area.width {
        top_border.push_str(buf[(x, border_y)].symbol());
    }
    assert!(
        !top_border.contains("Help"),
        "HelpDialog title MUST NOT be painted into the top border row"
    );
}

/// Scenario: HelpDialog body lists the board keybindings
#[test]
fn help_dialog_body_lists_every_rpc009_keybinding() {
    // @step Given an isolated HelpDialog component
    let mut dialog = HelpDialog::new();
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);
    let text = buffer_text(&buf);

    // RPC-397: the default (board) HelpDialog lists accurate board
    // keybindings rendered via the shared dialog theme. The old
    // generic "j/k / Tab / q / Ctrl+C" list is gone by design. At
    // 80x24 only the top of the (scrollable — RPC-396) list is
    // visible, so we assert the top-of-list board hints here; the
    // full list (incl. the Ctrl+D quit line) is pinned against a
    // 200x60 backend in tests/help_dialog_content_rpc397.rs.
    // @step Then the rendered buffer contains "Navigate"
    assert!(text.contains("Navigate"));
    // @step And the rendered buffer contains "New Agent"
    assert!(text.contains("New Agent"));
    // @step And the rendered buffer contains "Reorder"
    assert!(text.contains("Reorder"));
    // @step And the rendered buffer contains "Enter"
    assert!(text.contains("Enter"));
    // @step And the rendered buffer does NOT contain the old "q       Quit" line
    assert!(!text.contains("q       Quit"));
}

/// Scenario: HelpDialog no longer imports tui_popup
#[test]
fn help_dialog_no_longer_imports_tui_popup() {
    // @step Given the source file rust/fspec-tui/src/components/help_dialog.rs
    let src =
        fs::read_to_string("src/components/help_dialog.rs").expect("help_dialog.rs must exist");

    // @step Then the source does not contain the substring "tui_popup::Popup"
    assert!(
        !src.contains("tui_popup::Popup"),
        "help_dialog.rs must not import tui_popup::Popup"
    );
    // @step And the source does not contain "Popup::new("
    assert!(
        !src.contains("Popup::new("),
        "help_dialog.rs must not call Popup::new"
    );
    // @step And the source imports dialog_theme::render_dialog
    assert!(
        src.contains("dialog_theme::render_dialog")
            || src.contains("use crate::components::dialog_theme")
            || src.contains("use super::dialog_theme"),
        "help_dialog.rs must import dialog_theme::render_dialog"
    );
}

// ============================================================
// Section C — DisconnectDialog migration
// ============================================================

/// Scenario: DisconnectDialog renders with the red accent and the "Disconnected" inner title
#[test]
fn disconnect_dialog_renders_with_red_accent_and_disconnected_inner_title() {
    // @step Given a fresh DisconnectDialog with no Reconnecting action applied
    let mut dialog = DisconnectDialog::new();

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);

    // @step Then the border cells use foreground color Color::Red
    assert_eq!(find_border_color(&buf), Color::Red);

    // @step And the body's first non-padding row contains the text "Disconnected"
    let (disc_x, disc_y) = find_text(&buf, "Disconnected").expect("title found");

    // @step And the "Disconnected" text cells have foreground color Color::Red with BOLD modifier
    for i in 0..("Disconnected".chars().count() as u16) {
        let cell = &buf[(disc_x + i, disc_y)];
        assert_eq!(cell.fg, Color::Red);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    let text = buffer_text(&buf);
    // @step And the body contains the line "daemon disconnected"
    assert!(text.contains("daemon disconnected"));
    // @step And the body contains the line "q to quit"
    assert!(text.contains("q to quit"));
    // @step And the body contains the line "r to reconnect"
    assert!(text.contains("r to reconnect"));
}

/// Scenario: DisconnectDialog updates the body inline on Action::Reconnecting(N)
#[test]
fn disconnect_dialog_updates_body_inline_on_reconnecting_action() {
    // @step Given a fresh DisconnectDialog
    let mut dialog = DisconnectDialog::new();
    // @step When I dispatch Action::Reconnecting(3)
    let _ = dialog.update(Action::Reconnecting(3));
    // @step And I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);
    let text = buffer_text(&buf);
    // @step Then the body contains the substring "auto-reconnecting (attempt 3)"
    assert!(text.contains("auto-reconnecting (attempt 3)"));
    // @step And the border cells still use foreground color Color::Red
    assert_eq!(find_border_color(&buf), Color::Red);
    // @step And the "Disconnected" title is still painted with BOLD red foreground
    let (disc_x, disc_y) = find_text(&buf, "Disconnected").expect("title present");
    let cell = &buf[(disc_x, disc_y)];
    assert_eq!(cell.fg, Color::Red);
    assert!(cell.modifier.contains(Modifier::BOLD));
}

include!("rpc027_dialog_parity_bcd/thinking_level.rs");
