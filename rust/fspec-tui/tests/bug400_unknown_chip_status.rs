//! BUG-400 — an agent chip shows a session status the TUI has not observed
//! as unknown instead of painting it Idle.
//!
//! Feature: spec/features/agent-chip-unknown-session-status.feature

use codelet_fspec_tui::components::menu_bar::build_chips;
use codelet_fspec_tui::store::agent_view::{AgentViewStore, SessionContext};
use codelet_fspec_tui::views::board::menu_snapshot::chip_inputs;
use codelet_rpc_types::{SessionId, SessionStatus};
use codelet_test_helpers::TestResult;

fn store_with(session: &SessionId) -> AgentViewStore {
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(session.clone()));
    store
}

#[test]
fn an_unobserved_session_is_painted_with_the_unknown_glyph() -> TestResult {
    // @step Given an open session whose status the TUI has not observed
    let session = SessionId::new("s-unobserved");
    let store = store_with(&session);

    // @step When the agent chips are built for that session
    let inputs = chip_inputs(&store, std::slice::from_ref(&session));
    let chips = build_chips(&inputs, 0);

    // @step Then the chip input has no status
    let input = inputs
        .first()
        .ok_or("the open session must get a chip input")?;
    if input.status.is_some() {
        return Err(format!("an unobserved status must be None, got {:?}", input.status).into());
    }

    // @step And the chip shows the unknown glyph "?"
    let chip = chips.first().ok_or("the open session must get a chip")?;
    if chip.glyph != "?" {
        return Err(format!("expected the unknown glyph, got {:?}", chip.glyph).into());
    }
    Ok(())
}

#[test]
fn an_observed_status_is_painted_as_before() -> TestResult {
    // @step Given an open session whose status the TUI observed as Running
    let session = SessionId::new("s-running");
    let mut store = store_with(&session);
    store.set_session_status(session.clone(), SessionStatus::Running);

    // @step When the agent chips are built for that session
    let inputs = chip_inputs(&store, std::slice::from_ref(&session));
    let chips = build_chips(&inputs, 0);

    // @step Then the chip input status is Running
    let input = inputs
        .first()
        .ok_or("the open session must get a chip input")?;
    if input.status != Some(SessionStatus::Running) {
        return Err(format!("expected Running, got {:?}", input.status).into());
    }

    // @step And the chip shows the running spinner glyph
    let chip = chips.first().ok_or("the open session must get a chip")?;
    if chip.glyph == "?" || chip.glyph == "●" {
        return Err(format!("expected the spinner, got {:?}", chip.glyph).into());
    }
    Ok(())
}
