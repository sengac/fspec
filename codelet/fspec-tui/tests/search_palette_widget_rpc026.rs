//! RPC-026 — Widget tests for SearchPalette.
//!
//! Feature: spec/features/rpc026-search-palette.feature
//!
//! Exercises the standalone SearchPalette widget surface — set_query,
//! set_matches, handle_key (typing / navigation / Enter / Esc), and the
//! rendered placeholder / list bodies. App-level wiring lives in
//! `app_dispatch_resume_search_rpc026.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::search_palette::{SearchPalette, SearchPaletteOutcome};
use codelet_rpc_types::{HistoryMatch, SessionId};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn history_match(text: &str) -> HistoryMatch {
    HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: text.to_string(),
        timestamp_iso: "2026-05-17T00:00:00Z".to_string(),
    }
}

fn render_to_string(p: &SearchPalette) -> String {
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    p.render(buf.area, &mut buf);
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Scenario: A new SearchPalette has empty query and no matches
#[test]
fn new_palette_is_empty() {
    // @step Given a fresh SearchPalette
    let p = SearchPalette::new();
    // @step Then search_palette.query() equals ""
    assert_eq!(p.query(), "");
    // @step And search_palette.match_count() equals 0
    assert_eq!(p.match_count(), 0);
    // @step And search_palette.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
    // @step And search_palette.selected() returns None
    assert!(p.selected().is_none());
}

/// Scenario: set_query updates the filter text and resets selection to the first row
#[test]
fn set_query_updates_filter_and_resets_selection() {
    // @step Given a fresh SearchPalette
    let mut p = SearchPalette::new();
    // @step When set_query("git") is called
    p.set_query("git");
    // @step Then search_palette.query() equals "git"
    assert_eq!(p.query(), "git");
    // @step And search_palette.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
}

/// Scenario: set_matches populates the typeahead rows and clamps selection
#[test]
fn set_matches_populates_rows() {
    // @step Given a SearchPalette where set_query("git") has been called
    let mut p = SearchPalette::new();
    p.set_query("git");
    // @step When set_matches is called with three HistoryMatch values [text="git status", text="git push", text="git diff"]
    p.set_matches(vec![
        history_match("git status"),
        history_match("git push"),
        history_match("git diff"),
    ]);
    // @step Then search_palette.match_count() equals 3
    assert_eq!(p.match_count(), 3);
    // @step And search_palette.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
    // @step And search_palette.selected() returns Some(HistoryMatch with text "git status")
    assert_eq!(p.selected().expect("selected").text, "git status");
}

/// Scenario: set_matches with fewer rows than the current selection clamps the index
#[test]
fn set_matches_clamps_selection_when_shorter() {
    // @step Given a SearchPalette with three matches and selected_index == 2
    let mut p = SearchPalette::new();
    p.set_matches(vec![
        history_match("a"),
        history_match("b"),
        history_match("c"),
    ]);
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 2);
    // @step When set_matches is called with one match [text="git status"]
    p.set_matches(vec![history_match("git status")]);
    // @step Then search_palette.match_count() equals 1
    assert_eq!(p.match_count(), 1);
    // @step And search_palette.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
}

/// Scenario: Down arrow advances selection and wraps around at the end
#[test]
fn down_arrow_advances_and_wraps() {
    // @step Given a SearchPalette populated with three matches
    let mut p = SearchPalette::new();
    p.set_matches(vec![
        history_match("a"),
        history_match("b"),
        history_match("c"),
    ]);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then search_palette.selected_index() equals 1
    assert_eq!(p.selected_index(), 1);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then search_palette.selected_index() equals 2
    assert_eq!(p.selected_index(), 2);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then search_palette.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
}

/// Scenario: Up arrow walks backward and wraps to the last row
#[test]
fn up_arrow_walks_backward_and_wraps() {
    // @step Given a SearchPalette populated with three matches
    let mut p = SearchPalette::new();
    p.set_matches(vec![
        history_match("a"),
        history_match("b"),
        history_match("c"),
    ]);
    // @step When the user presses Up
    p.handle_key(KeyCode::Up, KeyModifiers::NONE);
    // @step Then search_palette.selected_index() equals 2
    assert_eq!(p.selected_index(), 2);
}

/// Scenario: Typing a printable character appends it to the query and emits FilterChanged
#[test]
fn typing_char_appends_to_query_and_emits_filter_changed() {
    // @step Given a fresh SearchPalette
    let mut p = SearchPalette::new();
    // @step When the user presses 'g'
    let outcome = p.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);
    // @step Then handle_key returns SearchPaletteOutcome::FilterChanged("g")
    match outcome {
        SearchPaletteOutcome::FilterChanged(ref q) => assert_eq!(q, "g"),
        other => panic!("expected FilterChanged('g'), got {other:?}"),
    }
    // @step And search_palette.query() equals "g"
    assert_eq!(p.query(), "g");
}

/// Scenario: Backspace removes the last character from the query and emits FilterChanged
#[test]
fn backspace_removes_last_char_and_emits_filter_changed() {
    // @step Given a SearchPalette where set_query("git") has been called
    let mut p = SearchPalette::new();
    p.set_query("git");
    // @step When the user presses Backspace
    let outcome = p.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    // @step Then handle_key returns SearchPaletteOutcome::FilterChanged("gi")
    match outcome {
        SearchPaletteOutcome::FilterChanged(ref q) => assert_eq!(q, "gi"),
        other => panic!("expected FilterChanged('gi'), got {other:?}"),
    }
    // @step And search_palette.query() equals "gi"
    assert_eq!(p.query(), "gi");
}

/// Scenario: Backspace on an empty query is a no-op
#[test]
fn backspace_on_empty_query_is_noop() {
    // @step Given a fresh SearchPalette with empty query
    let mut p = SearchPalette::new();
    // @step When the user presses Backspace
    let outcome = p.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    // @step Then handle_key returns SearchPaletteOutcome::Continued
    assert!(matches!(outcome, SearchPaletteOutcome::Continued));
    // @step And search_palette.query() equals ""
    assert_eq!(p.query(), "");
}

/// Scenario: Enter on a highlighted match emits Selected with the match text
#[test]
fn enter_emits_selected_with_match_text() {
    // @step Given a SearchPalette populated with [text="git status", text="git push"]
    let mut p = SearchPalette::new();
    p.set_matches(vec![history_match("git status"), history_match("git push")]);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step And the user presses Enter
    let outcome = p.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    // @step Then handle_key returns SearchPaletteOutcome::Selected("git push")
    match outcome {
        SearchPaletteOutcome::Selected(text) => assert_eq!(text, "git push"),
        other => panic!("expected Selected('git push'), got {other:?}"),
    }
}

/// Scenario: Enter on zero matches is ignored
#[test]
fn enter_on_zero_matches_is_ignored() {
    // @step Given a SearchPalette where set_query("xyzzy") has been called and matches is empty
    let mut p = SearchPalette::new();
    p.set_query("xyzzy");
    // @step When the user presses Enter
    let outcome = p.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    // @step Then handle_key returns SearchPaletteOutcome::Ignored
    assert!(matches!(outcome, SearchPaletteOutcome::Ignored));
}

/// Scenario: Esc on the popup returns Dismiss
#[test]
fn esc_returns_dismiss() {
    // @step Given a SearchPalette populated with one match
    let mut p = SearchPalette::new();
    p.set_matches(vec![history_match("git status")]);
    // @step When the user presses Esc
    let outcome = p.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    // @step Then handle_key returns SearchPaletteOutcome::Dismiss
    assert!(matches!(outcome, SearchPaletteOutcome::Dismiss));
}

/// Scenario: Modifier-prefixed keys are propagated so AgentView can route Shift+arrow chords
#[test]
fn shift_arrow_is_propagated_as_ignored() {
    // @step Given a SearchPalette populated with two matches
    let mut p = SearchPalette::new();
    p.set_matches(vec![history_match("a"), history_match("b")]);
    // @step When the user presses Shift+Down
    let outcome = p.handle_key(KeyCode::Down, KeyModifiers::SHIFT);
    // @step Then handle_key returns SearchPaletteOutcome::Ignored
    assert!(matches!(outcome, SearchPaletteOutcome::Ignored));
    // @step And search_palette.selected_index() is unchanged at 0
    assert_eq!(p.selected_index(), 0);
}

/// Scenario: Empty query renders the "(type to search history)" placeholder
#[test]
fn empty_query_renders_placeholder() {
    // @step Given a fresh SearchPalette with empty query
    let p = SearchPalette::new();
    // @step When the popup is rendered
    let painted = render_to_string(&p);
    // @step Then the rendered body contains the literal string "(type to search history)"
    assert!(
        painted.contains("(type to search history)"),
        "missing placeholder in:\n{painted}"
    );
}

/// Scenario: Non-empty query with zero matches renders the placeholder
#[test]
fn non_empty_query_with_zero_matches_renders_placeholder() {
    // @step Given a SearchPalette where set_query("xyzzy") has been called and matches is empty
    let mut p = SearchPalette::new();
    p.set_query("xyzzy");
    // @step When the popup is rendered
    let painted = render_to_string(&p);
    // @step Then the rendered body contains the literal string "(no history matches \"xyzzy\")"
    assert!(
        painted.contains("(no history matches \"xyzzy\")"),
        "missing placeholder in:\n{painted}"
    );
}

/// Scenario: Populated matches render one row per HistoryMatch with the navigation hint
#[test]
fn populated_matches_render_rows() {
    // @step Given a SearchPalette populated with [text="git status", text="git push"]
    let mut p = SearchPalette::new();
    p.set_matches(vec![history_match("git status"), history_match("git push")]);
    // @step When the popup is rendered
    let painted = render_to_string(&p);
    // @step Then the rendered body contains a row referencing "git status"
    assert!(painted.contains("git status"), "missing row:\n{painted}");
    // @step And the rendered body contains a row referencing "git push"
    assert!(painted.contains("git push"), "missing row:\n{painted}");
    // @step And the rendered body contains the navigation hint "↑↓ Navigate │ Enter Insert │ Esc Close"
    assert!(
        painted.contains("Navigate") && painted.contains("Insert") && painted.contains("Esc"),
        "missing nav hint:\n{painted}"
    );
}
