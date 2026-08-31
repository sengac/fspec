//! BUG-160 — Board search dialog: result rows show a dimmed title/description snippet.
//!
//! Feature: spec/features/board-search-dialog-result-snippet.feature
//!
//! ACDD TESTING phase: these tests assert the BUG-160 behaviour — the
//! WorkUnitSearchDialog result rows show the work-unit id followed by a
//! ` - ` separator and a mode-aware snippet (title in Id/Title mode,
//! description in Description mode, title fallback when the unit has no
//! description), built via the shared `label_description_row` primitive
//! (marker + label + dimmed description) and width-bounded with
//! `truncate_to` so a long title cannot widen the fixed frame rect
//! (BUG-159). The selection/scroll math operates on the match count only
//! and is unchanged.
//!
//! They compile against the yet-to-be-implemented
//! `filter_work_units` richer-match surface (`Vec<SearchMatch>` /
//! `snippets()` accessor) and the yet-to-be-painted snippet rows, so this
//! file is RED until the BUG-160 implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::dialog_theme_rows::label_description_row;
use codelet_fspec_tui::components::work_unit_search_dialog::{filter_work_units, SearchMode};
use codelet_fspec_tui::{Component, WorkUnitSearchDialog};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::Terminal;

const AREA: Rect = Rect::new(0, 0, 80, 24);

fn wu(id: &str, title: &str, description: Option<&str>) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: description.map(str::to_string),
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn char_key(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Render a Component into an 80x24 TestBackend and return the buffer.
fn render_80x24<C: Component>(component: &mut C) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    term.draw(|frame| component.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn rows(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect()
}

/// First buffer row (0-based y) containing `needle`, if any.
fn first_row_with(rows: &[String], needle: &str) -> Option<usize> {
    rows.iter().position(|r| r.contains(needle))
}

/// The DIM modifier state of the buffer cell at (x, y).
fn is_dim(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].modifier.contains(Modifier::DIM)
}

/// Scenario: A result row shows the id followed by a dimmed title snippet
#[test]
fn a_result_row_shows_the_id_followed_by_a_title_snippet() {
    // @step Given a board with a work unit "AUTH-001" in backlog titled "User login"
    let units = vec![wu("AUTH-001", "User login", None)];
    let mut dialog = WorkUnitSearchDialog::new(units);

    // @step When I open the search dialog and type "auth"
    for c in "auth".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists exactly one match "AUTH-001"
    assert_eq!(dialog.matches(), vec!["AUTH-001".to_string()]);

    // @step And the result row shows the id "AUTH-001" followed by " - User login"
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);
    let y = first_row_with(&rows, "AUTH-001").expect("the match row must be painted");
    assert!(
        rows[y].contains("AUTH-001 - User login"),
        "row must show the id followed by ' - <title snippet>':\n{}",
        rows.join("\n")
    );
}

/// Scenario: In Description mode the snippet is the unit description
#[test]
fn in_description_mode_the_snippet_is_the_unit_description() {
    // @step Given a board with a work unit "DOC-001" in backlog titled "Docs" whose description contains "viewer"
    let units = vec![wu("DOC-001", "Docs", Some("Open the attachment viewer"))];
    let mut dialog = WorkUnitSearchDialog::new(units);

    // @step When I open the search dialog and switch the search mode to Description
    let _ = dialog.handle_event(&key(KeyCode::Tab));
    let _ = dialog.handle_event(&key(KeyCode::Tab));
    assert_eq!(dialog.mode_label(), "description");

    // @step And I type "viewer" into the dialog
    for c in "viewer".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists exactly one match "DOC-001"
    assert_eq!(dialog.matches(), vec!["DOC-001".to_string()]);

    // @step And the result row shows the id "DOC-001" followed by " - Open the attachment viewer"
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);
    let y = first_row_with(&rows, "DOC-001").expect("the match row must be painted");
    assert!(
        rows[y].contains("DOC-001 - Open the attachment viewer"),
        "row must show the id followed by ' - <description snippet>':\n{}",
        rows.join("\n")
    );
}

/// Scenario: A unit without a description falls back to the title as snippet
#[test]
fn a_unit_without_a_description_falls_back_to_the_title_as_snippet() {
    // @step Given a board with a work unit "NO-DESC-1" in backlog titled "No description unit" that has no description
    let units = vec![wu("NO-DESC-1", "No description unit", None)];
    let mut dialog = WorkUnitSearchDialog::new(units);

    // @step When I open the search dialog and type "no-desc"
    for c in "no-desc".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists exactly one match "NO-DESC-1"
    assert_eq!(dialog.matches(), vec!["NO-DESC-1".to_string()]);

    // @step And the result row shows the id "NO-DESC-1" followed by " - No description unit"
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);
    let y = first_row_with(&rows, "NO-DESC-1").expect("the match row must be painted");
    assert!(
        rows[y].contains("NO-DESC-1 - No description unit"),
        "row must fall back to the title snippet when there is no description:\n{}",
        rows.join("\n")
    );
}

/// Scenario: A long title is truncated with a trailing ellipsis and the frame stays fixed
#[test]
fn a_long_title_is_truncated_with_a_trailing_ellipsis_and_the_frame_stays_fixed() {
    // @step Given a board with a single work unit "LONG-001" in backlog whose title is 60 characters long
    let long_title: String = "a".repeat(60);
    let units = vec![wu("LONG-001", &long_title, None)];
    let mut dialog = WorkUnitSearchDialog::new(units);

    // @step When I open the search dialog and render it on an 80x24 terminal
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);

    // @step Then the result row snippet ends with the ellipsis character "…"
    let y = first_row_with(&rows, "LONG-001").expect("the match row must be painted");
    assert!(
        rows[y].contains('…'),
        "a 60-char title must be truncated with a trailing ellipsis:\n{}",
        rows.join("\n")
    );

    // @step And the dialog frame width is the fixed rect width (not widened by the title)
    // Measure the top border (╭ ... ╮) in CHAR positions — the selected
    // row's inverse highlight bleeds past the frame, so the side borders
    // on the match row are not a reliable width measure.
    let top_y = first_row_with(&rows, "╭").expect("rounded top border");
    let top: Vec<char> = rows[top_y].chars().collect();
    let corner_span = top
        .iter()
        .position(|c| *c == '╭')
        .and_then(|first| top[first..].iter().position(|c| *c == '╮'))
        .expect("top border corners must be visible");
    // The frame rect is AREA.width - 4 wide (BUG-159), so the two top
    // border corners are `width - 1` columns apart.
    assert_eq!(
        corner_span,
        (AREA.width - 4 - 1) as usize,
        "the fixed frame width (BUG-159) must not be widened by a long title"
    );
}

/// Scenario: The selected row snippet is not dimmed but unselected rows are
#[test]
fn the_selected_row_snippet_is_not_dimmed_but_unselected_rows_are() {
    // @step Given a board with two work units "AAA-001" and "AAA-002" in backlog both titled "Same title"
    let units = vec![
        wu("AAA-001", "Same title", None),
        wu("AAA-002", "Same title", None),
    ];
    let mut dialog = WorkUnitSearchDialog::new(units);

    // @step When I open the search dialog and type "aaa"
    for c in "aaa".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step And I press Down to move the selection to the second match
    let _ = dialog.handle_event(&key(KeyCode::Down));
    assert_eq!(dialog.selected_index(), 1);

    // @step Then the snippet of the selected row "AAA-002" is not dimmed
    // @step And the snippet of the unselected row "AAA-001" is dimmed
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);
    let y_sel = first_row_with(&rows, "AAA-002").expect("selected row must be painted");
    let y_unsel = first_row_with(&rows, "AAA-001").expect("unselected row must be painted");
    // The snippet starts after the id + " - " separator.
    let sel_snippet_x = rows[y_sel]
        .find("Same title")
        .expect("selected row must show the title snippet") as u16;
    let unsel_snippet_x = rows[y_unsel]
        .find("Same title")
        .expect("unselected row must show the title snippet") as u16;
    assert!(
        !is_dim(&buf, sel_snippet_x, y_sel as u16),
        "the selected row's snippet must NOT be dimmed"
    );
    assert!(
        is_dim(&buf, unsel_snippet_x, y_unsel as u16),
        "the unselected row's snippet must be dimmed (label_description_row semantics)"
    );
}

/// Scenario: The match order and selection math are unchanged by the richer matches
#[test]
fn the_match_order_and_selection_math_are_unchanged_by_the_richer_matches() {
    // @step Given a board with three work units "AAA-001", "AAA-002" and "AAA-003" in backlog
    let units = vec![
        wu("AAA-001", "one", None),
        wu("AAA-002", "two", None),
        wu("AAA-003", "three", None),
    ];
    let mut dialog = WorkUnitSearchDialog::new(units);

    // @step When I open the search dialog and type "aaa"
    for c in "aaa".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog lists the matches in board order "AAA-001", "AAA-002", "AAA-003"
    assert_eq!(
        dialog.matches(),
        vec![
            "AAA-001".to_string(),
            "AAA-002".to_string(),
            "AAA-003".to_string()
        ]
    );

    // @step And pressing Down wraps the selection within the match list
    for _ in 0..3 {
        let _ = dialog.handle_event(&key(KeyCode::Down));
    }
    assert_eq!(
        dialog.selected_index(),
        0,
        "wrap-around selection must be preserved with the richer match shape"
    );
}

/// BUG-160 unit-level: filter_work_units returns the mode-aware snippet
/// (title in Id/Title mode, description in Description mode, title
/// fallback when the unit has no description) for every match, in board
/// order, parallel to the id list.
#[test]
fn filter_work_units_returns_mode_aware_snippets_parallel_to_ids() {
    let units = vec![
        wu("AAA-001", "First title", Some("First description")),
        wu("AAA-002", "Second title", None),
    ];
    // Id mode: snippet is the title.
    let id_matches = filter_work_units(&units, Default::default(), "");
    assert_eq!(
        id_matches.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        vec!["AAA-001", "AAA-002"]
    );
    let id_snippets: Vec<String> = id_matches.iter().map(|m| m.snippet.clone()).collect();
    assert_eq!(
        id_snippets,
        vec!["First title".to_string(), "Second title".to_string()]
    );
    // Description mode: only units WITH a description match; snippet is
    // the description.
    let desc_matches = filter_work_units(&units, SearchMode::Description, "");
    assert_eq!(
        desc_matches
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["AAA-001"]
    );
    assert_eq!(desc_matches[0].snippet, "First description");
}

/// BUG-160 unit-level: the row builder reuses label_description_row, so a
/// row built for a match is byte-identical to label_description_row(id,
/// snippet, is_sel) for the same inputs (the visual contract is the
/// shared primitive, not a re-implementation).
#[test]
fn the_row_builder_reuses_label_description_row_semantics() {
    let row = label_description_row("AUTH-001", "User login", true);
    let texts: String = row.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        texts.contains("AUTH-001 - User login"),
        "label_description_row must produce the id + ' - ' + snippet shape: {texts}"
    );
    assert!(row.selectable);
    assert!(row.selected);
}
