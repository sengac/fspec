//! RPC-064 — `/search` slash command end-to-end (UI view) integration tests.
//!
//! Feature: spec/features/search-history-debounce-and-polish.feature
//!
//! These tests drive the App + SearchHistoryView through the full
//! `/search` lifecycle and pin the four polish behaviours added by
//! RPC-064 on top of the RPC-026 base:
//!
//!   1. Debounce: typing inside a 150ms window fires ONE backend call
//!      with the final query (rapid-keystroke coalescing).
//!   2. Stale-discard: an `Action::HistorySearchResults { query, .. }`
//!      whose `query` no longer matches the live `search_view.query()`
//!      is dropped instead of folded into the visible matches.
//!   3. Result highlighting: occurrences of the live query inside each
//!      result row are rendered with `Modifier::BOLD` (case-insensitive
//!      match, original casing preserved).
//!   4. j/k navigation: lowercase `j`/`k` (no modifiers) move the
//!      selection ±1 (matching the arrow-key wrap semantics) WITHOUT
//!      appending to the query buffer.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, SearchHistoryView, SearchHistoryViewOutcome};
use codelet_rpc_types::{HistoryMatch, SessionId};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn hmatch(text: &str) -> HistoryMatch {
    HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: text.to_string(),
        timestamp_iso: "2026-05-25T00:00:00Z".to_string(),
    }
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Drain the App's action bus AND every pending tokio task spawned
/// inside `App::dispatch`. Mirrors helpers in
/// `pending_input_durability_rpc052.rs` and `keyboard_cascade_rpc051.rs`.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Spin until `predicate()` returns true or 1s elapses.
async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Picking /search from the palette opens the SearchHistoryView
//   empty with no backend call
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn picking_search_from_palette_opens_empty_view_no_backend_call() {
    // @step Given AgentView has no popups or mode views open
    let (mut app, mock) = fresh_app();
    assert!(app.navigator().agent.search_view.is_none());
    assert!(app.navigator().agent.slash_popup.is_none());

    // @step When the user picks "/search" from the slash command palette
    app.dispatch(Action::OpenSearchView);

    // @step Then AgentView.slash_popup is None
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And AgentView.search_view is Some(SearchHistoryView with empty query)
    let view = app
        .navigator()
        .agent
        .search_view
        .as_ref()
        .expect("search_view installed");
    assert_eq!(view.query(), "");
    // @step And backend.persistence_search_history has not been invoked
    drain_pending(&mut app).await;
    assert_eq!(mock.search_history_calls(), 0);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing Ctrl+R opens the SearchHistoryView empty with no
//   backend call
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_r_opens_empty_view_no_backend_call() {
    // @step Given AgentView has no popups or mode views open
    // @step And no search_view, resume_view, slash_popup, or file_popup is active
    let (mut app, mock) = fresh_app();
    assert!(app.navigator().agent.search_view.is_none());
    assert!(app.navigator().agent.resume_view.is_none());
    assert!(app.navigator().agent.slash_popup.is_none());
    assert!(app.navigator().agent.file_popup.is_none());

    // @step When the user presses Ctrl+R
    // (This is the action emitted by the Ctrl+R chord handler in
    //  views/agent/dispatch.rs::handle_event — we dispatch it directly
    //  to keep the test focused on App routing.)
    app.dispatch(Action::OpenSearchView);

    // @step Then Action::OpenSearchView is dispatched
    // (Implicit in dispatching it above.)
    // @step And AgentView.search_view is Some(SearchHistoryView with empty query)
    let view = app
        .navigator()
        .agent
        .search_view
        .as_ref()
        .expect("search_view installed");
    assert_eq!(view.query(), "");
    // @step And backend.persistence_search_history has not been invoked
    drain_pending(&mut app).await;
    assert_eq!(mock.search_history_calls(), 0);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Rapid typing within 150ms fires a single debounced backend
//   call with the final query
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rapid_typing_within_debounce_window_fires_single_backend_call() {
    // @step Given search_view is open with an empty query
    let (mut app, mock) = fresh_app();
    mock.set_history_search_results(vec![hmatch("git status")]);
    app.dispatch(Action::OpenSearchView);

    // @step When the user types "g" then "i" then "t" within 150ms of each other
    // Drive each keystroke through the actual `SearchHistoryView` widget
    // (`handle_key`) so the view's `query` advances on each char, and
    // dispatch the resulting `FilterChanged` outcome as
    // `Action::SearchHistory(q)` — matching the production
    // `AgentView::handle_search_view_key` wiring. All three keystrokes
    // happen synchronously inside the 150ms debounce window, so only
    // the final query "git" should reach the backend.
    for ch in ['g', 'i', 't'] {
        let outcome = {
            let view = app
                .navigator_mut()
                .agent
                .search_view
                .as_mut()
                .expect("search_view open");
            view.handle_key(KeyCode::Char(ch), KeyModifiers::NONE, 20)
        };
        let SearchHistoryViewOutcome::FilterChanged(q) = outcome else {
            panic!("expected FilterChanged from widget, got {outcome:?}");
        };
        app.dispatch(Action::SearchHistory(q));
    }

    // Wait for the debounce to flush and the backend call to land.
    wait_until(
        || mock.search_history_calls() >= 1,
        "backend.persistence_search_history to be invoked at least once",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then only one backend.persistence_search_history call has fired
    assert_eq!(
        mock.search_history_calls(),
        1,
        "rapid debounced typing must coalesce into exactly one backend call"
    );
    // @step And the last_history_query observed by the mock backend equals "git"
    assert_eq!(mock.last_history_query(), Some("git".to_string()));
    // @step And the view's query equals "git"
    let view = app
        .navigator()
        .agent
        .search_view
        .as_ref()
        .expect("search_view still open");
    assert_eq!(view.query(), "git");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Typing slower than the debounce window fires one backend
//   call per keystroke
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typing_slower_than_debounce_window_fires_one_call_per_keystroke() {
    // @step Given search_view is open with an empty query
    let (mut app, mock) = fresh_app();
    mock.set_history_search_results(vec![hmatch("git status")]);
    app.dispatch(Action::OpenSearchView);

    // @step When the user types "g", waits longer than 150ms, types "i", waits longer than 150ms, then types "t"
    // Drive each keystroke through the widget so the view's `query`
    // advances, then dispatch the resulting `Action::SearchHistory(q)`.
    // Wait for the debounce to flush AND for the backend's
    // `last_history_query` to reflect THIS keystroke's query before
    // typing the next char — that pins the in-order "g" → "gi" → "git"
    // sequence at the backend boundary.
    let mut typed = String::new();
    for (idx, ch) in ['g', 'i', 't'].iter().enumerate() {
        typed.push(*ch);
        let outcome = {
            let view = app
                .navigator_mut()
                .agent
                .search_view
                .as_mut()
                .expect("search_view open");
            view.handle_key(KeyCode::Char(*ch), KeyModifiers::NONE, 20)
        };
        let SearchHistoryViewOutcome::FilterChanged(q) = outcome else {
            panic!("expected FilterChanged from widget, got {outcome:?}");
        };
        assert_eq!(q, typed);
        app.dispatch(Action::SearchHistory(q.clone()));
        let expected_calls = idx + 1;
        let q_for_wait = q.clone();
        let mock_for_wait = mock.clone();
        wait_until(
            move || {
                mock_for_wait.search_history_calls() >= expected_calls
                    && mock_for_wait.last_history_query().as_deref() == Some(q_for_wait.as_str())
            },
            "debounced call to flush AND backend.last_history_query to advance to this keystroke's query",
        )
        .await;
        // Assert the in-order outcome at THIS step (so we pin the
        // sequence rather than just the final value).
        assert_eq!(mock.search_history_calls(), expected_calls);
        assert_eq!(mock.last_history_query(), Some(q));
    }
    drain_pending(&mut app).await;

    // @step Then backend.persistence_search_history has fired three times
    assert_eq!(mock.search_history_calls(), 3);
    // @step And the queries sent in order are "g", "gi", "git"
    // Pinned step-by-step inside the loop above (each iteration asserts
    // `last_history_query` equals the cumulative typed prefix before
    // advancing). Final assertion confirms the most-recent flush.
    assert_eq!(mock.last_history_query(), Some("git".to_string()));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Stale results from an older query are discarded
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_results_from_older_query_are_discarded() {
    // @step Given search_view is open with the current query "git"
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenSearchView);
    {
        let view = app
            .navigator_mut()
            .agent
            .search_view
            .as_mut()
            .expect("search_view");
        view.set_query("git");
    }

    // @step And an in-flight backend response is pending for the older query "g"
    // (Conceptually — we simulate by dispatching the response action
    //  with the OLDER query attached.)

    // @step When the older response Action::HistorySearchResults { query = "g", matches = [HistoryMatch("git log")] } arrives
    app.dispatch(Action::HistorySearchResults {
        query: "g".to_string(),
        matches: vec![hmatch("git log")],
    });

    // @step Then search_view.matches remains unchanged from its previous state
    let view = app
        .navigator()
        .agent
        .search_view
        .as_ref()
        .expect("search_view still open");
    assert_eq!(
        view.match_count(),
        0,
        "stale results for query \"g\" must not be folded when the view's current query is \"git\""
    );
    // @step And the view does NOT fold the stale "g" response into the visible matches
    assert!(view.matches().iter().all(|m| m.text != "git log"));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Fresh results matching the current query are folded
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_results_matching_current_query_are_folded() {
    // @step Given search_view is open with the current query "git"
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenSearchView);
    {
        let view = app
            .navigator_mut()
            .agent
            .search_view
            .as_mut()
            .expect("search_view");
        view.set_query("git");
    }

    // @step When Action::HistorySearchResults { query = "git", matches = [...] } is dispatched
    app.dispatch(Action::HistorySearchResults {
        query: "git".to_string(),
        matches: vec![hmatch("git status"), hmatch("git push")],
    });

    // @step Then search_view.matches has length 2
    let view = app
        .navigator()
        .agent
        .search_view
        .as_ref()
        .expect("search_view");
    assert_eq!(view.match_count(), 2);
    // @step And search_view.selected_index equals 0
    assert_eq!(view.selected_index(), 0);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Result row highlights the matching substring in bold
// ─────────────────────────────────────────────────────────────────────

/// Walk the rendered buffer looking for the leftmost cell on any
/// row whose symbol equals `needle_first_char`, AND whose subsequent
/// cells (in column order) spell out `row_substring`. Returns the
/// (col, row) of the matched starting cell.
///
/// Columns are counted in BUFFER CELLS (not string bytes) so a
/// multi-byte unicode marker like `▸` still consumes exactly one
/// column. This matters because the body row prefix " ▸ " contains
/// a 3-byte character that would otherwise misalign byte-based
/// `String::find` lookups.
fn find_first_col_of(
    buf: &Buffer,
    row_substring: &str,
    needle_first_char: char,
) -> Option<(u16, u16)> {
    let needle_buf: String = needle_first_char.to_string();
    for y in 0..buf.area.height {
        for x_start in 0..buf.area.width {
            let sym = buf[(x_start, y)].symbol();
            if sym != needle_buf {
                continue;
            }
            // Walk forward and re-assemble the expected substring
            // cell-by-cell so we only return matches where the row
            // actually paints `row_substring` from this column.
            let mut acc = String::new();
            let mut x_walk = x_start;
            while x_walk < buf.area.width && acc.len() < row_substring.len() {
                acc.push_str(buf[(x_walk, y)].symbol());
                x_walk += 1;
            }
            if acc.starts_with(row_substring) {
                return Some((x_start, y));
            }
        }
    }
    None
}

#[test]
fn result_row_highlights_matching_substring_in_bold() {
    // @step Given search_view is open with query "git"
    let mut view = SearchHistoryView::new();
    view.set_query("git");
    // @step And the loaded matches contain "git status now"
    view.set_matches(vec![hmatch("git status now")]);

    // @step When AgentView paints the search_view body
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // Locate the row containing "git status now" — the body renderer
    // prefixes the row with " ▸ " (or " ▸ ") for the selected row.
    let (g_col, g_row) = find_first_col_of(&buf, "git status now", 'g')
        .expect("must find the matched-row letter `g`");

    // @step Then the substring "git" inside the row is rendered with Modifier::BOLD
    for offset in 0..3u16 {
        let cell = &buf[(g_col + offset, g_row)];
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "expected cell at col={} row={} ({:?}) to have BOLD modifier set",
            g_col + offset,
            g_row,
            cell.symbol()
        );
    }
    // @step And the substring " status now" inside the row is rendered without Modifier::BOLD
    // Check the space + 's' immediately after "git".
    for offset in 3..6u16 {
        let cell = &buf[(g_col + offset, g_row)];
        assert!(
            !cell.modifier.contains(Modifier::BOLD),
            "expected cell at col={} row={} ({:?}) NOT to have BOLD modifier set",
            g_col + offset,
            g_row,
            cell.symbol()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Result row highlights every case-insensitive occurrence
//   in bold
// ─────────────────────────────────────────────────────────────────────

#[test]
fn result_row_highlights_every_case_insensitive_occurrence() {
    // @step Given search_view is open with the lowercase query "git"
    let mut view = SearchHistoryView::new();
    view.set_query("git");
    // @step And the loaded matches contain "GIT add then git push"
    view.set_matches(vec![hmatch("GIT add then git push")]);

    // @step When AgentView paints the search_view body
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // Find the start of the row text by looking for the first 'G'
    // of "GIT add then git push" on any rendered row.
    let (g_col, row) = find_first_col_of(&buf, "GIT add then git push", 'G')
        .expect("must find the matched-row letter `G`");

    // @step Then the rendered row preserves the original casing "GIT add then git push"
    // Walk 22 cells from g_col and re-assemble the substring.
    let mut painted = String::new();
    for offset in 0..21u16 {
        painted.push_str(buf[(g_col + offset, row)].symbol());
    }
    assert!(
        painted.starts_with("GIT add then git push"),
        "row at col={g_col} row={row} actually reads: {painted:?}"
    );

    // @step And both substrings "GIT" and "git" inside the row are rendered with Modifier::BOLD
    // "GIT" occupies offsets 0..3
    for offset in 0..3u16 {
        let cell = &buf[(g_col + offset, row)];
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "expected uppercase `GIT` cell at offset {offset} ({:?}) to have BOLD",
            cell.symbol()
        );
    }
    // "git" occupies offsets 13..16 ("GIT add then ".len() == 13)
    for offset in 13..16u16 {
        let cell = &buf[(g_col + offset, row)];
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "expected lowercase `git` cell at offset {offset} ({:?}) to have BOLD",
            cell.symbol()
        );
    }

    // @step And the substring " add then " between them is rendered without Modifier::BOLD
    for offset in 3..13u16 {
        let cell = &buf[(g_col + offset, row)];
        assert!(
            !cell.modifier.contains(Modifier::BOLD),
            "expected separator cell at offset {offset} ({:?}) NOT to have BOLD",
            cell.symbol()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing j moves selection down and k moves it up with
//   wrap-around
// ─────────────────────────────────────────────────────────────────────

#[test]
fn j_and_k_navigate_with_wrap_and_do_not_modify_query() {
    // @step Given search_view has two loaded matches and selected_index equals 0
    let mut view = SearchHistoryView::new();
    let initial_query = view.query().to_string();
    view.set_matches(vec![hmatch("git status"), hmatch("git push")]);
    assert_eq!(view.selected_index(), 0);

    // @step When the user presses "j"
    let outcome = view.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, 20);
    // @step Then selected_index equals 1
    assert_eq!(view.selected_index(), 1);
    assert_eq!(
        outcome,
        SearchHistoryViewOutcome::Continued,
        "j key must be Continued (selection moved, no filter change)"
    );

    // @step When the user presses "k"
    let outcome = view.handle_key(KeyCode::Char('k'), KeyModifiers::NONE, 20);
    // @step Then selected_index equals 0
    assert_eq!(view.selected_index(), 0);
    assert_eq!(outcome, SearchHistoryViewOutcome::Continued);

    // @step When the user presses "j" twice
    view.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, 20);
    view.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, 20);
    // @step Then selected_index equals 0
    assert_eq!(
        view.selected_index(),
        0,
        "j wraps from the last row back to the first"
    );

    // @step And the j/k keystrokes did NOT modify the query buffer
    assert_eq!(view.query(), initial_query);
    assert_eq!(view.query(), "");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Enter on a highlighted match inserts the text and closes
//   the view
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_inserts_match_text_and_closes_search_view() {
    // @step Given search_view is open with query "git"
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenSearchView);
    {
        let view = app
            .navigator_mut()
            .agent
            .search_view
            .as_mut()
            .expect("search_view");
        view.set_query("git");
        // @step And two matches are loaded with "git status" at selected_index 0
        view.set_matches(vec![hmatch("git status"), hmatch("git push")]);
    }

    // @step When the user presses Enter
    // Drive `KeyCode::Enter` through the widget so the `Selected(text)`
    // outcome is what feeds `Action::InsertIntoInput(text)` — exactly
    // the wiring inside `AgentView::handle_search_view_key`.
    let outcome = {
        let view = app
            .navigator_mut()
            .agent
            .search_view
            .as_mut()
            .expect("search_view");
        view.handle_key(KeyCode::Enter, KeyModifiers::NONE, 20)
    };
    assert_eq!(
        outcome,
        SearchHistoryViewOutcome::Selected("git status".to_string()),
        "Enter on the highlighted match must produce Selected(text)"
    );
    app.dispatch(Action::InsertIntoInput("git status".to_string()));

    // @step Then Action::InsertIntoInput("git status") is dispatched (above)
    // @step And AgentView.search_view is None
    assert!(app.navigator().agent.search_view.is_none());
    // @step And AgentView.input.value() equals "git status"
    assert_eq!(app.navigator().agent.input.value(), "git status");
    // @step And the input was NOT auto-submitted
    // (No InputSubmitted dispatched — the input still holds the text
    //  but no send_input was spawned. Verified by zero subsequent
    //  pending tasks after drain.)
    drain_pending(&mut app).await;
    assert!(app.next_pending_task().is_none());
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Esc closes the view without inserting and leaves the input
//   unchanged
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_closes_view_without_inserting_and_leaves_input_unchanged() {
    // @step Given the input contained "draft text" before /search was opened
    let (mut app, _mock) = fresh_app();
    app.navigator_mut().agent.input.set_value("draft text");
    // @step And search_view is open with query "git" and a highlighted match "git status"
    // Ctrl+R / OpenSearchView preserves the live input draft (RPC-064:
    // `handle_open_search_view` no longer calls `input.reset()`). The
    // slash-palette path's `input.reset()` lives upstream in
    // `AgentView::handle_popup_key` so the typed `/search` text is
    // still cleared on palette pick.
    app.dispatch(Action::OpenSearchView);
    {
        let view = app
            .navigator_mut()
            .agent
            .search_view
            .as_mut()
            .expect("search_view");
        view.set_query("git");
        view.set_matches(vec![hmatch("git status")]);
    }

    // @step When the user presses Esc
    // Drive the actual `KeyCode::Esc` through the widget so the
    // `Dismiss` outcome is what triggers the `CloseSearchView` action,
    // matching `AgentView::handle_search_view_key`'s wiring.
    let outcome = {
        let view = app
            .navigator_mut()
            .agent
            .search_view
            .as_mut()
            .expect("search_view");
        view.handle_key(KeyCode::Esc, KeyModifiers::NONE, 20)
    };
    assert_eq!(outcome, SearchHistoryViewOutcome::Dismiss);
    app.dispatch(Action::CloseSearchView);

    // @step Then Action::CloseSearchView is dispatched (above)
    // @step And AgentView.search_view is None
    assert!(app.navigator().agent.search_view.is_none());
    // @step And AgentView.input.value() equals "draft text"
    assert_eq!(app.navigator().agent.input.value(), "draft text");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Empty query state renders the placeholder and fires no
//   backend calls
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_query_state_renders_placeholder_and_fires_no_backend_calls() {
    // @step Given the user just opened /search and has not typed anything
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::OpenSearchView);

    // @step When AgentView paints the search_view body
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    {
        let view = app
            .navigator()
            .agent
            .search_view
            .as_ref()
            .expect("search_view");
        view.render(area, &mut buf);
    }
    let mut painted = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            painted.push_str(buf[(x, y)].symbol());
        }
        painted.push('\n');
    }
    // @step Then the body contains the placeholder "(type to search history)"
    assert!(
        painted.contains("(type to search history)"),
        "expected the empty-query placeholder, got:\n{painted}"
    );
    // @step And backend.persistence_search_history has not been invoked
    drain_pending(&mut app).await;
    assert_eq!(mock.search_history_calls(), 0);

    // @step When the user presses Esc
    app.dispatch(Action::CloseSearchView);
    // @step Then AgentView.search_view is None
    assert!(app.navigator().agent.search_view.is_none());
    // @step And backend.persistence_search_history has still not been invoked
    drain_pending(&mut app).await;
    assert_eq!(mock.search_history_calls(), 0);
}
