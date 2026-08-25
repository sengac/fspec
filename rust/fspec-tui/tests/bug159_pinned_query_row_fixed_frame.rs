//! BUG-159 — Board search dialog: pinned query row + fixed frame rect.
//!
//! Feature: spec/features/board-search-dialog-pinned-query-row-and-fixed-frame.feature
//!
//! ACDD TESTING phase: these tests assert the BUG-159 behaviour — the
//! WorkUnitSearchDialog paints the live query on a dedicated row pinned
//! under the title (visible at all times while typing), and its frame is a
//! FIXED rect (fixed_dialog_rect + render_dialog_at) that does not
//! re-center as the match list grows or shrinks. `body_content_rows`
//! gains a `has_query_row` parameter so the body viewport math accounts
//! for the reserved query row, and dialogs without a query row render
//! unchanged (content still starts at the first content row).
//!
//! They compile against the yet-to-be-implemented
//! `body_content_rows(rect_height, footer_h, has_query_row)` surface and
//! the yet-to-be-painted query row, so this file is RED until the
//! BUG-159 implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::dialog_theme::{render_dialog_at, Accent, DialogRow, FspecDialog};
use codelet_fspec_tui::components::dialog_theme_rows::{
    body_content_rows, build_dialog, fixed_dialog_rect,
};
use codelet_fspec_tui::{Component, WorkUnitSearchDialog};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Span;
use ratatui::Terminal;

fn wu(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: "title".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn char_key(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
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

/// Inner (border-stripped) text of buffer row `y`.
fn inner_row(buf: &Buffer, y: u16, rect: Rect) -> String {
    let start = rect.x + 2;
    let end = (rect.x + rect.width).saturating_sub(2);
    let mut s = String::new();
    for x in start..end {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

const AREA: Rect = Rect::new(0, 0, 80, 24);

/// Scenario: Typing a query paints the query text on a pinned row under the title
#[test]
fn typing_a_query_paints_the_query_text_on_a_pinned_row_under_the_title() {
    // @step Given the work-unit search dialog is open on an 80x24 terminal
    let units: Vec<WorkUnitInfo> = (1..=5).map(|i| wu(&format!("AUTH-{i:03}"))).collect();
    let mut dialog = WorkUnitSearchDialog::new(units);
    let rect = fixed_dialog_rect(AREA);

    // @step When I type "auth" into the dialog
    for c in "auth".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step Then the dialog renders the query "auth" on the row immediately below the title
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);
    // Spacious layout: title at rect.y+2, gap at rect.y+3, first content
    // row (the pinned query row) at rect.y+4.
    let query_y = rect.y + 4;
    assert!(
        rows[query_y as usize].contains("▸ auth"),
        "query row must echo the live query on the pinned row:\n{}",
        rows.join("\n")
    );
    assert!(
        inner_row(&buf, rect.y + 3, rect).trim().is_empty(),
        "the gap row between title and query row must stay blank:\n{}",
        rows.join("\n")
    );

    // @step And the query row is styled with the accent color and a trailing block cursor
    let x = rows[query_y as usize]
        .find('▸')
        .expect("query marker '▸' on the pinned row");
    assert_eq!(
        buf[(x as u16, query_y)].fg,
        Color::Cyan,
        "query row must use the dialog accent color"
    );
    assert!(
        rows[query_y as usize].contains('▏'),
        "trailing block cursor expected on the query row: {:?}",
        rows[query_y as usize]
    );
}

/// Scenario: The dialog frame is invariant as the match list grows
#[test]
fn the_dialog_frame_is_invariant_as_the_match_list_grows() {
    // @step Given a board with 20 work units whose ids all contain "a" and only one contains "ab"
    let mut units = vec![wu("AB-001")];
    for i in 1..=19 {
        units.push(wu(&format!("AA-{i:03}")));
    }
    let mut dialog = WorkUnitSearchDialog::new(units);
    let rect = fixed_dialog_rect(AREA);

    // @step When I open the search dialog and type "a"
    let _ = dialog.handle_event(&char_key('a'));
    assert_eq!(dialog.matches().len(), 20, "query 'a' must match all 20 units");
    let buf_a = render_80x24(&mut dialog);

    // @step Then the dialog frame top-left corner is at the fixed_dialog_rect position
    let top_a = first_row_with(&rows(&buf_a), "╭").expect("rounded top border");
    assert_eq!(
        top_a,
        rect.y as usize,
        "frame top border must sit at fixed_dialog_rect y"
    );

    // @step When I type "b" so the query is "ab" leaving only 1 match
    let _ = dialog.handle_event(&char_key('b'));
    assert_eq!(dialog.matches().len(), 1, "query 'ab' must match exactly one unit");
    let buf_b = render_80x24(&mut dialog);

    // @step Then the dialog frame top-left corner is unchanged
    let top_b = first_row_with(&rows(&buf_b), "╭").expect("rounded top border");
    assert_eq!(
        top_b, top_a,
        "frame must not re-center as the match list shrinks from 20 to 1"
    );
}

/// Scenario: The query row is visible when the body is at maximum height
#[test]
fn the_query_row_is_visible_when_the_body_is_at_maximum_height() {
    // @step Given the work-unit search dialog is open on an 80x24 terminal with 20 matches
    let units: Vec<WorkUnitInfo> = (1..=20).map(|i| wu(&format!("AA-{i:03}"))).collect();
    let mut dialog = WorkUnitSearchDialog::new(units);
    let _ = dialog.handle_event(&char_key('a'));
    assert_eq!(dialog.matches().len(), 20);
    let rect = fixed_dialog_rect(AREA);

    // @step When the match list fills the body viewport
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);

    // @step Then the query row is still painted on the pinned row under the title
    let query_y = rect.y + 4;
    assert!(
        rows[query_y as usize].contains("▸ a▏"),
        "query row must stay pinned under the title at maximum body height:\n{}",
        rows.join("\n")
    );
    assert!(
        rows[(query_y + 1) as usize].contains("AA-"),
        "match rows must start one row below the pinned query row:\n{}",
        rows.join("\n")
    );
}

/// Scenario: The body viewport accounts for the query row in compact layout
#[test]
fn the_body_viewport_accounts_for_the_query_row_in_compact_layout() {
    // @step Given a dialog rect of height 6 with a 1-line footer
    // @step When body_content_rows is called with has_query_row = true
    let with_query = body_content_rows(6, 1, true);

    // @step Then the result is one less than body_content_rows with has_query_row = false
    let without = body_content_rows(6, 1, false);
    assert_eq!(
        with_query,
        without - 1,
        "the reserved query row must consume exactly one content row (compact)"
    );
    // Same invariant in the spacious layout.
    assert_eq!(
        body_content_rows(20, 1, true),
        body_content_rows(20, 1, false) - 1,
        "the reserved query row must consume exactly one content row (spacious)"
    );
}

/// Scenario: Dialogs without a query row render unchanged
#[test]
fn dialogs_without_a_query_row_render_unchanged() {
    // @step Given a dialog built with build_dialog and no query_row set
    let row = DialogRow {
        spans: vec![Span::raw("row one")],
        selectable: true,
        selected: true,
    };
    let dialog: FspecDialog<'_> = build_dialog(Accent::Cyan, "Title", vec![row], "footer hint", 20);
    let rect = fixed_dialog_rect(AREA);
    let mut buf = Buffer::empty(AREA);

    // @step When the dialog is rendered at a fixed rect
    render_dialog_at(rect, &mut buf, &dialog);
    let rows = rows(&buf);

    // @step Then the rendered buffer is byte-identical to the pre-BUG-159 output
    // Observable invariant: the gap row stays blank and the first content
    // row (rect.y+4) carries the first row's text — no extra row, no shift.
    assert!(
        inner_row(&buf, rect.y + 3, rect).trim().is_empty(),
        "gap row must stay blank when no query row is set:\n{}",
        rows.join("\n")
    );
    assert!(
        rows[(rect.y + 4) as usize].contains("row one"),
        "first content row must stay at the pre-BUG-159 position:\n{}",
        rows.join("\n")
    );
    assert!(
        !rows.join("").contains('▏'),
        "no block cursor may appear when no query row is set"
    );
}

/// Scenario: An empty query still shows the pinned query row
#[test]
fn an_empty_query_still_shows_the_pinned_query_row() {
    // @step Given the work-unit search dialog is open with an empty query
    let mut dialog = WorkUnitSearchDialog::new(vec![wu("AUTH-001")]);
    let rect = fixed_dialog_rect(AREA);

    // @step When the dialog is rendered
    let buf = render_80x24(&mut dialog);
    let rows = rows(&buf);

    // @step Then the query row is painted with a block cursor on the pinned row under the title
    let query_y = rect.y + 4;
    assert!(
        rows[query_y as usize].contains("▸ ▏"),
        "an empty query must still occupy the pinned query row with a block cursor:\n{}",
        rows.join("\n")
    );
}
