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
        .find(|y| {
            (0..buf.area.width).any(|x| buf[(x, *y)].symbol() == "╭")
        })
        .expect("top border row exists");
    for x in 0..buf.area.width {
        top_border.push_str(buf[(x, border_y)].symbol());
    }
    assert!(
        !top_border.contains("Help"),
        "HelpDialog title MUST NOT be painted into the top border row"
    );
}

/// Scenario: HelpDialog body lists every RPC-009 keybinding
#[test]
fn help_dialog_body_lists_every_rpc009_keybinding() {
    // @step Given an isolated HelpDialog component
    let mut dialog = HelpDialog::new();
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the rendered buffer contains "j/k"
    assert!(text.contains("j/k"));
    // @step And the rendered buffer contains "Tab"
    assert!(text.contains("Tab"));
    // @step And the rendered buffer contains "?"
    assert!(text.contains('?'));
    // @step And the rendered buffer contains "q"
    assert!(text.contains('q'));
    // @step And the rendered buffer contains "Enter"
    assert!(text.contains("Enter"));
    // @step And the rendered buffer contains "Ctrl+C"
    assert!(text.contains("Ctrl+C"));
    // @step And the rendered buffer contains "ESC"
    assert!(text.contains("ESC"));
}

/// Scenario: HelpDialog no longer imports tui_popup
#[test]
fn help_dialog_no_longer_imports_tui_popup() {
    // @step Given the source file codelet/fspec-tui/src/components/help_dialog.rs
    let src = fs::read_to_string("src/components/help_dialog.rs")
        .expect("help_dialog.rs must exist");

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

// ============================================================
// Section D — ThinkingLevelDialog migration
// ============================================================

fn make_thinking_dialog(current: ThinkingLevel) -> ThinkingLevelDialog {
    ThinkingLevelDialog::new(SessionId::new("test-session"), current)
}

/// Scenario: ThinkingLevelDialog renders with the yellow accent and inner-title body
#[test]
fn thinking_level_dialog_renders_with_yellow_accent_and_inner_title() {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);
    // @step Then the border cells use foreground color Color::Yellow
    assert_eq!(find_border_color(&buf), Color::Yellow);
    // @step And the body's first non-padding row contains the text "Thinking Level"
    let (x, y) = find_text(&buf, "Thinking Level").expect("title present");
    // @step And the "Thinking Level" text cells have foreground color Color::Yellow with BOLD modifier
    for i in 0..("Thinking Level".chars().count() as u16) {
        let cell = &buf[(x + i, y)];
        assert_eq!(cell.fg, Color::Yellow);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }
}

/// Scenario: ThinkingLevelDialog highlights the current level with the inverse style
#[test]
fn thinking_level_dialog_highlights_the_current_level_with_inverse_style() {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);

    let (off_x, off_y) = find_text(&buf, "Off").expect("Off row present");
    // @step Then the "Off" row has background color Color::Yellow
    // @step And the "Off" row has foreground color Color::Black with BOLD modifier
    let cell = &buf[(off_x, off_y)];
    assert_eq!(cell.bg, Color::Yellow);
    assert_eq!(cell.fg, Color::Black);
    assert!(cell.modifier.contains(Modifier::BOLD));

    // @step And the "Off" row begins with the two-character marker "▸ "
    let marker_x = off_x - 2;
    assert_eq!(buf[(marker_x, off_y)].symbol(), "▸");
    assert_eq!(buf[(marker_x + 1, off_y)].symbol(), " ");

    // @step And the "Low", "Medium", and "High" rows begin with the two-character marker "  "
    for label in &["Low", "Medium", "High"] {
        let (lx, ly) = find_text(&buf, label).unwrap_or_else(|| panic!("{label} present"));
        let m_x = lx - 2;
        assert_eq!(buf[(m_x, ly)].symbol(), " ", "unselected marker[0] for {label}");
        assert_eq!(buf[(m_x + 1, ly)].symbol(), " ", "unselected marker[1] for {label}");
    }

    // @step And the description text for unselected rows carries Modifier::DIM
    let (low_x, low_y) = find_text(&buf, "Low").expect("Low present");
    // Description starts after "Low - " — look a few cells to the right
    let after_label = low_x + ("Low - ".chars().count() as u16);
    let desc_cell = &buf[(after_label, low_y)];
    assert!(
        desc_cell.modifier.contains(Modifier::DIM),
        "unselected description must be DIM"
    );
}

/// Scenario: ThinkingLevelDialog footer documents the four key bindings
#[test]
fn thinking_level_dialog_footer_documents_the_four_key_bindings() {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog);
    let text = buffer_text(&buf);

    // @step Then the last body row contains the substring "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"
    assert!(
        text.contains("↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"),
        "footer must include the D Set Default binding"
    );

    // @step And the footer text carries Modifier::DIM
    let (fx, fy) = find_text(&buf, "↑↓ Navigate").expect("footer present");
    assert!(buf[(fx, fy)].modifier.contains(Modifier::DIM));
    // @step And the footer text is horizontally centered (cannot trivially verify
    // without computing rect; the dialog_theme test pins centering directly)
}

/// Scenario: Pressing D in ThinkingLevelDialog emits Action::SetThinkingLevelDefault and keeps the dialog open
#[test]
fn pressing_d_emits_set_thinking_level_default_and_keeps_dialog_open() {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off and currently highlighting "Medium"
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // Navigate to Medium (index 2)
    let _ = dialog.handle_event(&key_event(KeyCode::Down));
    let _ = dialog.handle_event(&key_event(KeyCode::Down));
    assert_eq!(dialog.selected_level(), ThinkingLevel::Medium);

    // @step When I send a KeyCode::Char('d') event
    let result = dialog.handle_event(&key_event(KeyCode::Char('d')));

    // @step Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::Medium)
    let action = dialog
        .take_pending_action()
        .expect("D must emit a pending action");
    match action {
        Action::SetThinkingLevelDefault(_sid, level) => {
            assert_eq!(level, ThinkingLevel::Medium);
        }
        other => panic!("expected SetThinkingLevelDefault, got {other:?}"),
    }

    // @step And the dialog returns EventResult::Consumed without a remove-callback
    match result {
        EventResult::Consumed(None) => {}
        EventResult::Consumed(Some(_)) => {
            panic!("D must NOT supply a remove-callback — dialog stays open")
        }
        EventResult::Ignored(_) => panic!("D must be consumed"),
    }

    // @step And the dialog is still mounted on the compositor
    // (the absence of a remove-callback above is the operational proof)
}

/// Scenario: Pressing uppercase D in ThinkingLevelDialog behaves identically to lowercase d
#[test]
fn pressing_uppercase_d_behaves_identically_to_lowercase_d() {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::High
    let mut dialog = make_thinking_dialog(ThinkingLevel::High);
    // @step When I send a KeyCode::Char('D') event
    let result = dialog.handle_event(&key_event(KeyCode::Char('D')));
    // @step Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::High)
    let action = dialog
        .take_pending_action()
        .expect("uppercase D must emit pending action");
    match action {
        Action::SetThinkingLevelDefault(_sid, level) => {
            assert_eq!(level, ThinkingLevel::High);
        }
        other => panic!("expected SetThinkingLevelDefault, got {other:?}"),
    }
    // @step And the dialog is still mounted on the compositor
    match result {
        EventResult::Consumed(None) => {}
        _ => panic!("uppercase D must be Consumed(None)"),
    }
}

/// Scenario: SetThinkingLevelDefault is wired through the backend trait stack
#[test]
fn set_thinking_level_default_is_wired_through_the_backend_trait_stack() {
    // @step Given the codelet_rpc_types::Action enum
    // @step Then it contains the variant SetThinkingLevelDefault(SessionId, ThinkingLevel)
    let _variant = Action::SetThinkingLevelDefault(
        SessionId::new("s"),
        ThinkingLevel::Medium,
    );

    // @step Given the SessionManagerHandle trait
    // @step Then it declares set_thinking_level_default with a default no-op implementation returning Ok(())
    //
    // This is a source-shape assertion. The trait lives in
    // codelet/core/src/session_manager_handle.rs. We grep the source.
    let trait_src = fs::read_to_string(
        Path::new("..").join("core").join("src").join("session_manager_handle.rs"),
    )
    .or_else(|_| fs::read_to_string("../core/src/session_manager_handle.rs"))
    .expect("session_manager_handle.rs must exist");
    assert!(
        trait_src.contains("set_thinking_level_default"),
        "SessionManagerHandle must declare set_thinking_level_default"
    );

    // @step Given the FspecBackend trait
    // @step Then it declares set_thinking_level_default on both transports
    let backend_src = fs::read_to_string(
        Path::new("src").join("transport").join("mod.rs"),
    )
    .or_else(|_| fs::read_to_string("src/transport/mod.rs"))
    .expect("transport/mod.rs must exist");
    assert!(
        backend_src.contains("set_thinking_level_default"),
        "FspecBackend trait must declare set_thinking_level_default"
    );

    // RPC-027 rule [8] + architecture note [4]: App::dispatch must
    // route Action::SetThinkingLevelDefault through a handler that
    // calls backend.set_thinking_level_default. Without this wiring
    // the dialog emits an action that is silently dropped.
    // @step Then dispatch_rpc022.rs routes Action::SetThinkingLevelDefault to backend.set_thinking_level_default
    let dispatch_src = fs::read_to_string(
        Path::new("src").join("app").join("dispatch_rpc022.rs"),
    )
    .or_else(|_| fs::read_to_string("src/app/dispatch_rpc022.rs"))
    .expect("dispatch_rpc022.rs must exist");
    assert!(
        dispatch_src.contains("Action::SetThinkingLevelDefault"),
        "dispatch_rpc022.rs must route Action::SetThinkingLevelDefault"
    );
    assert!(
        dispatch_src.contains("backend.set_thinking_level_default("),
        "dispatch_rpc022.rs must call backend.set_thinking_level_default"
    );
}
