//! PROV-107 — core model_selector behaviour tests (nav/select/filter/refresh).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: Navigation moves through provider headers (TS parity, PROV-104)
#[test]
fn down_arrow_lands_on_provider_header() {
    // @step Given the model selector shows a provider header followed by model rows
    let mut v = loaded_view();
    // loaded_view layout: 0=openai header, 1=gpt-4o, 2=o3-mini,
    // 3=anthropic header, 4=claude-sonnet. Start anchored on idx 1.
    // @step And the cursor is on the last model row above a provider header
    v.handle_key(key(KeyCode::Down)); // idx 1 -> 2 (o3-mini)
    let before = v.selected_index();
    assert_eq!(before, 2);

    // @step When I press the down arrow
    v.handle_key(key(KeyCode::Down));

    // @step Then the cursor moves to the next row (the provider header),
    // matching the TS navigateDown clamp over the full flat list with no
    // header-skipping; Right/Enter then expands it.
    let after = v.selected_index();
    assert_eq!(after, 3, "clamped move lands on the anthropic header row");
    assert!(
        !v.rows.get(after).map(|r| r.selectable).unwrap_or(true),
        "row 3 is a non-selectable provider header"
    );
}

/// Scenario: Selecting a model with an active session commits the choice
#[test]
fn enter_with_session_emits_model_selected() {
    // @step Given the model selector is open with an active session
    let mut v = loaded_view();
    // @step And the cursor is on the model row "claude-sonnet [R] [V] [200k]"
    // Navigate to the last selectable row.
    v.handle_key(key(KeyCode::End));

    // @step When I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then a model selection is emitted for the current session, provider and model
    match out {
        ModelSelectorEvent::Emit(Action::ModelSelected(Some(sid), pkey, mid)) => {
            assert_eq!(sid.value, "s-1");
            assert!(!pkey.is_empty());
            assert!(!mid.is_empty());
        }
        other => panic!("expected Emit(ModelSelected(Some(..))), got {other:?}"),
    }
    // @step And the model selector view closes
    // @step And the session header badge updates to the selected model
    // (close + badge refresh are driven by Navigator::apply_action +
    //  App dispatch of ModelSelected — asserted in navigator tests.)
}

/// Scenario: Selecting a model with no active session still emits the selection
///
/// PROV-117 reversal: the prior behavior (Enter == no-op without a session)
/// diverged from the TS implementation, whose Enter handler has NO
/// session-existence guard (ModelSelectorScreen.tsx:203-210). Selection is
/// always emitted; only the downstream backend write is gated on a session
/// (modelSelectionService.selectModel `if (sessionId)`). The selector closes
/// either way. See also `tests_enter_expand::
/// enter_on_model_row_with_no_session_still_emits_selection`.
#[test]
fn enter_without_session_still_emits_selection() {
    // @step Given the model selector is open with no current session
    let mut v = ModelSelectorView::new();
    v.set_session(None);
    v.set_providers(vec![provider("openai", &["gpt-4o"])]);
    expand_all(&mut v);
    // @step And the cursor is on a selectable model row
    v.handle_key(key(KeyCode::Home));

    // @step When I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then a model selection is emitted with no session id
    match out {
        ModelSelectorEvent::Emit(Action::ModelSelected(None, pkey, mid)) => {
            assert!(!pkey.is_empty());
            assert!(!mid.is_empty());
        }
        other => panic!("expected Emit(ModelSelected(None, ..)), got {other:?}"),
    }
    // @step And the view closes
    // (close is driven by Navigator::apply_action on ModelSelected.)
}

/// Scenario: Open the model selector full-screen via the slash command
#[test]
fn title_text_reports_model_count() {
    // @step Given I am in the Agent view
    // @step When I run the "/model" slash command
    let v = loaded_view();
    // @step Then the model selector replaces the screen as a full-screen view
    // @step And the title reads "Select Model (N models)"
    assert_eq!(v.title_text(), "Select Model (3 models)");
    // @step And the provider list is requested asynchronously
    // (the list_providers spawn is asserted in the dispatch layer.)
}

/// Scenario: Expanding and collapsing a provider group
#[test]
fn left_collapses_right_expands_focused_provider() {
    // @step Given the model selector shows an expanded provider group
    let mut v = loaded_view();
    assert!(v.is_expanded("openai"));
    v.handle_key(key(KeyCode::Home)); // focus first model (openai)

    // @step When I press the left arrow on the provider group
    v.handle_key(key(KeyCode::Left));
    // @step Then the group collapses and hides its model rows
    assert!(!v.is_expanded("openai"));

    // @step When I press the right arrow on the provider group
    v.handle_key(key(KeyCode::Right));
    // @step Then the group expands and shows its model rows
    assert!(v.is_expanded("openai"));
}

/// Scenario: Refreshing the model list
#[test]
fn r_key_emits_refresh_and_sets_refreshing() {
    // @step Given the model selector is open
    let mut v = loaded_view();
    assert!(!v.is_refreshing());

    // @step When I press "r"
    let out = v.handle_key(key(KeyCode::Char('r')));

    // @step Then the provider's models are refreshed
    assert!(matches!(
        out,
        ModelSelectorEvent::Emit(Action::RefreshModelSelector)
    ));
    // @step And the title shows "(refreshing...)" while the refresh is in flight
    assert!(v.is_refreshing());
    assert!(v.title_text().contains("(refreshing...)"));
    // @step And the list updates once the refreshed models arrive
    v.set_providers(vec![provider("openai", &["gpt-4o"])]);
    assert!(!v.is_refreshing());
}

/// Scenario: Close the model selector with Esc returns to Agent
#[test]
fn esc_emits_close() {
    // @step Given I am in the model selector mode-view
    let mut v = loaded_view();
    // @step When I press Esc
    let out = v.handle_key(key(KeyCode::Esc));
    // @step Then the model selector closes
    // @step And I am returned to the Agent view
    assert!(matches!(out, ModelSelectorEvent::Close));
}

/// Scenario: Filtering narrows the model list
#[test]
fn slash_enters_filter_then_typing_narrows() {
    // @step Given the model selector is showing all providers and models
    let mut v = loaded_view();
    assert_eq!(v.model_count(), 3);

    // @step When I press "/" and type filter text
    v.handle_key(key(KeyCode::Char('/')));
    v.handle_key(key(KeyCode::Char('o')));
    v.handle_key(key(KeyCode::Char('3')));

    // @step Then the list narrows to models matching the filter
    assert_eq!(v.model_count(), 1);

    // @step And clearing the filter restores the full list
    v.handle_key(key(KeyCode::Backspace));
    v.handle_key(key(KeyCode::Backspace));
    assert_eq!(v.model_count(), 3);
}

/// Scenario: Overflowing list shows scroll indicators and wheel navigates
#[test]
fn overflow_shows_indicators_and_wheel_advances_skipping_headers() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

    // @step Given the model list overflows the viewport
    let many: Vec<&str> = vec!["m0", "m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9"];
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![
        provider("openai", &many),
        provider("anthropic", &["a0", "a1"]),
    ]);
    expand_all(&mut v);

    // @step When the list is rendered
    // Render into a short area (8 rows total → ~4 list rows) so the
    // list overflows the viewport.
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 8)).expect("term");
    term.draw(|f| v.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }

    // @step Then a scrollbar column shows the scroll position beside the list
    // PROV-104 (TS parity): the scroll indicator lives in a dedicated
    // column (proportional thumb), NOT inline arrows that steal content
    // rows. The thumb track uses ■ / │ glyphs.
    assert!(
        joined.contains('■') || joined.contains('│'),
        "expected a dedicated scrollbar column: {joined}"
    );

    // @step When I scroll the mouse wheel down
    let before = v.selected_index();
    let ev = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    let _ = v.handle_mouse(ev);

    // @step Then the selection advances skipping provider headers
    let after = v.selected_index();
    assert_ne!(after, before, "wheel down must advance the selection");
    assert!(
        v.rows.get(after).map(|r| r.selectable).unwrap_or(false),
        "selection must land on a selectable model row, never a header"
    );
    let _ = MouseButton::Left; // keep import used across crossterm versions
}
