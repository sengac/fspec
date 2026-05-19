//! RPC-027 — Tests for SlashCommandPopup + FileSearchPopup parity.
//!
//! Feature: spec/features/rpc027-slash-file-popups.feature
//! Covers Sections G (SlashCommandPopup) and H (FileSearchPopup).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use codelet_fspec_tui::views::agent::file_search_popup::FileSearchPopup;
use codelet_fspec_tui::views::agent::slash_command_popup::SlashCommandPopup;

fn render_slash_80x24(p: &SlashCommandPopup) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|f| p.render(f.area(), f.buffer_mut())).expect("draw");
    term.backend().buffer().clone()
}

fn render_file_80x24(p: &FileSearchPopup) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|f| p.render(f.area(), f.buffer_mut())).expect("draw");
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
// Section G — SlashCommandPopup
// ============================================================

/// Scenario: SlashCommandPopup renders with the cyan accent and "Slash Commands" inner title
#[test]
fn slash_command_popup_renders_with_cyan_accent_and_inner_title() {
    // @step Given a SlashCommandPopup with at least one matching command
    let popup = SlashCommandPopup::new();
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_slash_80x24(&popup);
    // @step Then the border cells use foreground color Color::Cyan
    assert_eq!(find_border_color(&buf), Color::Cyan);
    // @step And the body's first non-padding row contains the text "Slash Commands"
    let (x, y) = find_text(&buf, "Slash Commands").expect("title present");
    // @step And the title cells have foreground color Color::Cyan with BOLD modifier
    for i in 0..("Slash Commands".chars().count() as u16) {
        let cell = &buf[(x + i, y)];
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }
}

/// Scenario: SlashCommandPopup uses the two-character marker on every match row
#[test]
fn slash_command_popup_uses_two_character_marker_on_every_match_row() {
    // @step Given a SlashCommandPopup with three matching commands and selected_index = 1
    let mut popup = SlashCommandPopup::new();
    // Walk down once to make index 1 selected
    use crossterm::event::{KeyCode, KeyModifiers};
    let _ = popup.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(popup.selected_index(), 1);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_slash_80x24(&popup);

    // Locate "/" prefix of each row — first three /commands appear in top area
    let cmds: Vec<&codelet_fspec_tui::views::agent::slash_commands::SlashCommand> =
        popup.matches().iter().take(3).copied().collect();
    assert!(cmds.len() >= 3, "need ≥ 3 commands for this test");
    let labels: Vec<String> = cmds.iter().map(|c| format!("/{}", c.name())).collect();

    let (l0_x, l0_y) = find_text(&buf, &labels[0]).expect("row 0 present");
    let (l1_x, l1_y) = find_text(&buf, &labels[1]).expect("row 1 present");
    let (l2_x, l2_y) = find_text(&buf, &labels[2]).expect("row 2 present");

    // @step Then the row at index 0 begins with the two-character marker "  "
    assert_eq!(buf[(l0_x - 2, l0_y)].symbol(), " ");
    assert_eq!(buf[(l0_x - 1, l0_y)].symbol(), " ");
    // @step And the row at index 1 begins with the two-character marker "▸ "
    assert_eq!(buf[(l1_x - 2, l1_y)].symbol(), "▸");
    assert_eq!(buf[(l1_x - 1, l1_y)].symbol(), " ");
    // @step And the row at index 2 begins with the two-character marker "  "
    assert_eq!(buf[(l2_x - 2, l2_y)].symbol(), " ");
    assert_eq!(buf[(l2_x - 1, l2_y)].symbol(), " ");
}

/// Scenario: SlashCommandPopup highlights the selected match with the inverse cyan/black style
#[test]
fn slash_command_popup_highlights_selected_match_with_inverse_cyan_black() {
    // @step Given a SlashCommandPopup with three matching commands and selected_index = 1
    let mut popup = SlashCommandPopup::new();
    use crossterm::event::{KeyCode, KeyModifiers};
    let _ = popup.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_slash_80x24(&popup);
    // Locate the second command's name on the body
    let cmds: Vec<&codelet_fspec_tui::views::agent::slash_commands::SlashCommand> =
        popup.matches().iter().take(3).copied().collect();
    let label1 = format!("/{}", cmds[1].name());
    let (l1_x, l1_y) = find_text(&buf, &label1).expect("row 1 present");
    // @step Then the row at index 1 has background Color::Cyan and foreground Color::Black with BOLD modifier
    let cell = &buf[(l1_x, l1_y)];
    assert_eq!(cell.bg, Color::Cyan);
    assert_eq!(cell.fg, Color::Black);
    assert!(cell.modifier.contains(Modifier::BOLD));
    // @step And no other row carries the inverse highlight
    let label0 = format!("/{}", cmds[0].name());
    let (l0_x, l0_y) = find_text(&buf, &label0).expect("row 0 present");
    assert_ne!(buf[(l0_x, l0_y)].bg, Color::Cyan, "row 0 must not be highlighted");
}

/// Scenario: SlashCommandPopup footer documents Tab/Enter Select
#[test]
fn slash_command_popup_footer_documents_tab_enter_select() {
    // @step Given a SlashCommandPopup with matches
    let popup = SlashCommandPopup::new();
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_slash_80x24(&popup);
    let text = buffer_text(&buf);
    // @step Then the footer contains "↑↓ Navigate │ Tab/Enter Select │ Esc Close"
    assert!(text.contains("↑↓ Navigate │ Tab/Enter Select │ Esc Close"));
    // @step And the footer carries Modifier::DIM
    let (fx, fy) = find_text(&buf, "↑↓ Navigate").expect("footer present");
    assert!(buf[(fx, fy)].modifier.contains(Modifier::DIM));
}

// ============================================================
// Section H — FileSearchPopup
// ============================================================

/// Scenario: FileSearchPopup renders with the cyan accent and "File Search" inner title
#[test]
fn file_search_popup_renders_with_cyan_accent_and_inner_title() {
    // @step Given a FileSearchPopup with at least one match
    let mut popup = FileSearchPopup::new(0, "rea");
    popup.set_matches(vec!["README.md".to_string()]);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_file_80x24(&popup);
    // @step Then the border cells use foreground color Color::Cyan
    assert_eq!(find_border_color(&buf), Color::Cyan);
    // @step And the body's first non-padding row contains the text "File Search"
    let (x, y) = find_text(&buf, "File Search").expect("title present");
    // @step And the title cells have foreground color Color::Cyan with BOLD modifier
    for i in 0..("File Search".chars().count() as u16) {
        let cell = &buf[(x + i, y)];
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }
}

/// Scenario: FileSearchPopup uses the two-character marker on every match row
#[test]
fn file_search_popup_uses_two_character_marker_on_every_match_row() {
    // @step Given a FileSearchPopup with three matches and selected_index = 0
    let mut popup = FileSearchPopup::new(0, "");
    popup.set_matches(vec![
        "a.md".to_string(),
        "b.md".to_string(),
        "c.md".to_string(),
    ]);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_file_80x24(&popup);
    // @step Then the row at index 0 begins with the two-character marker "▸ "
    let (a_x, a_y) = find_text(&buf, "a.md").expect("a.md present");
    assert_eq!(buf[(a_x - 2, a_y)].symbol(), "▸");
    assert_eq!(buf[(a_x - 1, a_y)].symbol(), " ");
    // @step And rows 1 and 2 begin with the two-character marker "  "
    let (b_x, b_y) = find_text(&buf, "b.md").expect("b.md present");
    assert_eq!(buf[(b_x - 2, b_y)].symbol(), " ");
    assert_eq!(buf[(b_x - 1, b_y)].symbol(), " ");
    let (c_x, c_y) = find_text(&buf, "c.md").expect("c.md present");
    assert_eq!(buf[(c_x - 2, c_y)].symbol(), " ");
    assert_eq!(buf[(c_x - 1, c_y)].symbol(), " ");
}

/// Scenario: FileSearchPopup empty-state literals render in plain text
#[test]
fn file_search_popup_empty_state_literal_renders_in_plain_text() {
    // @step Given a FileSearchPopup with no matches and an empty filter
    let popup = FileSearchPopup::new(0, "");
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_file_80x24(&popup);
    let text = buffer_text(&buf);
    // @step Then the body contains the literal "(type to search files)"
    assert!(text.contains("(type to search files)"));
    // @step And the literal carries no inverse highlight
    let (lx, ly) = find_text(&buf, "(type to search files)").expect("literal present");
    assert_ne!(buf[(lx, ly)].bg, Color::Cyan, "empty-state must not be highlighted");
    assert_ne!(buf[(lx, ly)].fg, Color::Black);
}

/// Scenario: FileSearchPopup no-match state renders with the filter quoted
#[test]
fn file_search_popup_no_match_state_renders_with_filter_quoted() {
    // @step Given a FileSearchPopup with filter "zzz" and zero matches
    let popup = FileSearchPopup::new(0, "zzz");
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_file_80x24(&popup);
    let text = buffer_text(&buf);
    // @step Then the body contains the literal "(no files match \"zzz\")"
    assert!(
        text.contains("(no files match \"zzz\")"),
        "expected no-match literal in: {text}"
    );
    // @step And the literal carries no inverse highlight
    let (lx, ly) = find_text(&buf, "(no files match").expect("literal present");
    assert_ne!(buf[(lx, ly)].bg, Color::Cyan);
    assert_ne!(buf[(lx, ly)].fg, Color::Black);
}
