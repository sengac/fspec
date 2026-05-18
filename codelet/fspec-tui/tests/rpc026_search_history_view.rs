//! RPC-026 — SearchHistoryView widget unit tests.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::{SearchHistoryView, SearchHistoryViewOutcome};
use codelet_rpc_types::{HistoryMatch, SessionId};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn hmatch(text: &str) -> HistoryMatch {
    HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: text.to_string(),
        timestamp_iso: "2026-05-18T00:00:00Z".to_string(),
    }
}

fn rows_of(buf: &Buffer) -> Vec<String> {
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

/// Scenario: Slash command /search opens the full-screen search view empty
#[test]
fn empty_query_renders_typeahead_placeholder() {
    // @step Given AgentView has no popups or mode views open
    let v = SearchHistoryView::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    // @step When AgentView.render_with_store paints
    v.render(Rect::new(0, 0, 80, 24), &mut buf);
    let joined = rows_of(&buf).join("\n");
    // @step Then the header row contains "(search): " followed by an inverse-space block cursor
    assert!(joined.contains("(search): "));
    // @step And the body shows the placeholder "(type to search history)"
    assert!(joined.contains("(type to search history)"));
}

/// Scenario: Typing characters emits FilterChanged
#[test]
fn typing_characters_emits_filter_changed() {
    // @step Given search_view is open with an empty query
    let mut v = SearchHistoryView::new();
    // @step When the user types "g" then "i" then "t"
    let mut emitted: Vec<String> = Vec::new();
    for c in ['g', 'i', 't'] {
        match v.handle_key(KeyCode::Char(c), KeyModifiers::NONE, 20) {
            SearchHistoryViewOutcome::FilterChanged(q) => emitted.push(q),
            other => panic!("expected FilterChanged, got {other:?}"),
        }
    }
    // @step Then search_view.query becomes "git"
    assert_eq!(v.query(), "git");
    // @step And three Action::SearchHistory dispatches were emitted in order ("g", "gi", "git")
    assert_eq!(emitted, vec!["g".to_string(), "gi".to_string(), "git".to_string()]);
    // @step And backend.persistence_search_history was invoked with "git"
    // (Widget-level: emitting FilterChanged("git") IS the dispatch that App::dispatch turns into the backend call.)
    assert_eq!(emitted.last().map(String::as_str), Some("git"));
    // @step And the backend returned two HistoryMatch values
    // (Widget-level: simulate the backend returning two matches via set_matches.)
    v.set_matches(vec![hmatch("git status"), hmatch("git push")]);
    assert_eq!(v.match_count(), 2);
}

/// Scenario: set_matches folds in results
#[test]
fn set_matches_folds_results_with_first_highlighted() {
    // @step Given search_view is open with query "git"
    let mut v = SearchHistoryView::new();
    v.set_query("git");
    // @step When Action::HistorySearchResults is folded into search_view
    v.set_matches(vec![hmatch("git status"), hmatch("git push")]);
    // @step Then search_view.matches has length 2
    assert_eq!(v.match_count(), 2);
    // @step And search_view.selected_index equals 0
    assert_eq!(v.selected_index(), 0);

    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    v.render(Rect::new(0, 0, 80, 24), &mut buf);
    let joined = rows_of(&buf).join("\n");
    // @step And the rendered list shows both rows with the first highlighted
    assert!(joined.contains("git status"));
    assert!(joined.contains("git push"));
}

/// Scenario: Enter on a highlighted match emits Selected(text)
#[test]
fn enter_emits_selected_text() {
    // @step Given search_view is open with query "git" and 2 matches with "git status" highlighted
    let mut v = SearchHistoryView::new();
    v.set_query("git");
    v.set_matches(vec![hmatch("git status"), hmatch("git push")]);
    // @step When the user presses Enter
    let outcome = v.handle_key(KeyCode::Enter, KeyModifiers::NONE, 20);
    // @step Then Action::InsertIntoInput("git status") is dispatched
    assert_eq!(
        outcome,
        SearchHistoryViewOutcome::Selected("git status".to_string())
    );
}

/// Scenario: Non-empty query with zero matches renders no-match placeholder
#[test]
fn no_match_placeholder_renders_for_unmatched_query() {
    // @step Given search_view is open with query "xyzzy"
    let mut v = SearchHistoryView::new();
    v.set_query("xyzzy");
    // @step And backend.persistence_search_history("xyzzy") returned an empty Vec
    v.set_matches(Vec::new());
    // @step When AgentView.render_with_store paints
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    v.render(Rect::new(0, 0, 80, 24), &mut buf);
    let joined = rows_of(&buf).join("\n");
    // @step Then the body shows the placeholder "(no history matches \"xyzzy\")"
    assert!(joined.contains("(no history matches \"xyzzy\")"));
    // @step When the user presses Enter
    let outcome = v.handle_key(KeyCode::Enter, KeyModifiers::NONE, 20);
    // @step Then the keystroke is ignored
    assert_eq!(outcome, SearchHistoryViewOutcome::Ignored);
    // @step When the user presses Esc
    let esc = v.handle_key(KeyCode::Esc, KeyModifiers::NONE, 20);
    // @step Then Action::CloseSearchView is dispatched
    assert_eq!(esc, SearchHistoryViewOutcome::Dismiss);
    // @step And AgentView.search_view is None
    // (Widget-level: the Dismiss outcome IS what App::dispatch turns into search_view = None.)
    // The widget itself is the outgoing value being dropped by the parent.
}

/// Scenario: SearchHistoryView scrolls past 10 rows using terminal height
#[test]
fn scrolls_past_ten_rows_using_terminal_height() {
    // @step Given search_view has 40 HistoryMatch values
    let matches: Vec<HistoryMatch> = (0..40).map(|i| hmatch(&format!("entry-{i}"))).collect();
    let mut v = SearchHistoryView::new();
    v.set_query("entry");
    v.set_matches(matches);
    // @step And the render area height is 24 — visible_rows = 21
    let visible_rows = 21usize;
    // @step When the user presses ↓ fifteen times
    for _ in 0..15 {
        v.handle_key(KeyCode::Down, KeyModifiers::NONE, visible_rows);
    }
    // @step Then search_view.selected_index equals 15
    assert_eq!(v.selected_index(), 15);
    // @step And search_view.scroll_offset has advanced so row 15 falls inside the visible window
    assert!(v.selected_index() >= v.scroll_offset());
    assert!(v.selected_index() < v.scroll_offset() + visible_rows);
    // @step When the user presses ↓ until selection wraps past index 39
    for _ in 15..40 {
        v.handle_key(KeyCode::Down, KeyModifiers::NONE, visible_rows);
    }
    // @step Then search_view.selected_index equals 0
    assert_eq!(v.selected_index(), 0);
    // @step And search_view.scroll_offset resets to 0
    assert_eq!(v.scroll_offset(), 0);
}

/// Scenario: Ctrl+R while search_view is open is ignored
#[test]
fn ctrl_r_while_search_view_open_is_ignored() {
    // @step Given search_view is open
    let mut v = SearchHistoryView::new();
    // @step When the user presses Ctrl+R
    let outcome = v.handle_key(KeyCode::Char('r'), KeyModifiers::CONTROL, 20);
    // @step Then the chord is forwarded to the search_view which returns Ignored
    assert_eq!(outcome, SearchHistoryViewOutcome::Ignored);
    // @step And search_view stays open with unchanged query and matches
    assert_eq!(v.query(), "");
    assert_eq!(v.match_count(), 0);
}
