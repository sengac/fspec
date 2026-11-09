//! BUG-377 — agent panes report a missing session context instead of painting an idle composer or an empty
//! ghost draft, and focusing a session index that is not open is refused.
//!
//! Feature: spec/features/agent-panes-report-missing-session-context.feature

use codelet_fspec_tui::store::{AgentViewStore, SessionContext};
use codelet_fspec_tui::views::agent::{
    pane_render::PaneSession, AgentView, INPUT_PLACEHOLDER_HINT,
};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::TestResult;
use ratatui::{buffer::Buffer, layout::Rect};
use tokio::sync::mpsc::unbounded_channel;

fn two_open_sessions() -> AgentViewStore {
    let mut store = AgentViewStore::default();
    for name in ["first", "second"] {
        store.append_session(SessionContext::new(SessionId::new(name)));
    }
    store.focus_session_index(0);
    store
}

/// Paint one pane naming `session` and return its rows.
fn paint_pane(store: &mut AgentViewStore, session: &str, focused: bool) -> Vec<String> {
    let (sender, _receiver) = unbounded_channel();
    let mut view = AgentView::new(sender);
    let area = Rect::new(0, 0, 100, 18);
    let mut buffer = Buffer::empty(area);
    view.render_session_pane(
        area,
        &mut buffer,
        store,
        PaneSession {
            session: Some(SessionId::new(session)),
            is_focused: focused,
            menu_row: false,
        },
    );
    (0..area.height)
        .map(|row| {
            (0..area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect()
        })
        .collect()
}

fn expect_missing_session_row(rows: &[String]) -> TestResult {
    if !rows
        .iter()
        .any(|row| row.contains("session gone is not open"))
    {
        return Err(format!("no 'session gone is not open' row in {rows:#?}").into());
    }
    if rows.iter().any(|row| row.contains(INPUT_PLACEHOLDER_HINT)) {
        return Err(format!("placeholder hint painted for a missing session: {rows:#?}").into());
    }
    Ok(())
}

#[test]
fn focusing_a_session_index_that_is_not_open_is_refused_and_keeps_the_current_focus() -> TestResult
{
    // @step Given two agent sessions are open and the first one is focused
    let mut store = two_open_sessions();
    // @step When the store is asked to focus session index 5
    let refused = match store.focus_open_session(5) {
        Ok(context) => return Err(format!("index 5 focused session {}", context.id).into()),
        Err(error) => error,
    };
    // @step Then it reports that no session is open at index 5 out of 2 open sessions
    assert_eq!((refused.index, refused.open), (5, 2));
    assert_eq!(
        refused.to_string(),
        "no agent session is open at index 5 (2 open)"
    );
    // @step And the first session stays focused
    assert_eq!(store.current_session(), Some(&SessionId::new("first")));
    Ok(())
}

#[test]
fn an_unfocused_pane_whose_session_is_no_longer_open_says_so() -> TestResult {
    // @step Given an unfocused agent pane names session "gone" that is not open
    let mut store = two_open_sessions();
    // @step When the pane is painted
    let rows = paint_pane(&mut store, "gone", false);
    // @step Then its input row reads "session gone is not open"
    // @step And the input placeholder hint is not shown
    expect_missing_session_row(&rows)
}

#[test]
fn the_focused_pane_whose_session_is_no_longer_open_says_so() -> TestResult {
    // @step Given the focused agent pane names session "gone" that is not open
    let mut store = two_open_sessions();
    // @step When the pane is painted
    let rows = paint_pane(&mut store, "gone", true);
    // @step Then its input row reads "session gone is not open"
    // @step And the input placeholder hint is not shown
    expect_missing_session_row(&rows)
}

#[test]
fn looking_up_the_slot_of_a_session_that_is_not_open_reports_it_as_missing() -> TestResult {
    // @step Given two agent sessions are open and the first one is focused
    let store = two_open_sessions();
    // @step When the store is asked for the slot of session "gone"
    let slot = store.session_index_for(&SessionId::new("gone"));
    // @step Then it reports that session "gone" is not open
    assert_eq!(format!("{slot:?}"), "None");
    // @step And the open sessions report slots 1 and 2 of 2
    for (name, slot) in [("first", "Some((1, 2))"), ("second", "Some((2, 2))")] {
        let found = store.session_index_for(&SessionId::new(name));
        assert_eq!(format!("{found:?}"), slot, "slot of {name}");
    }
    Ok(())
}

#[test]
fn the_agent_bar_shows_no_chip_for_a_session_that_is_not_open() -> TestResult {
    // @step Given two agent sessions are open and the first one is focused
    let store = two_open_sessions();
    // @step When the agent bar builds chips for sessions "first" and "gone"
    let active = [SessionId::new("first"), SessionId::new("gone")];
    let chips = codelet_fspec_tui::views::board::menu_snapshot::chip_inputs(&store, &active);
    // @step Then only session "first" gets a chip, numbered 1
    let slots: Vec<(usize, usize)> = chips.iter().map(|chip| chip.index).collect();
    assert_eq!(slots, vec![(1, 2)]);
    Ok(())
}
