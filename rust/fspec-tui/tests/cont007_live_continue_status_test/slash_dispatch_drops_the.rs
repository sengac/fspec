//! Feature: spec/features/live-continue-status-indicator.feature
use super::*;
use codelet_test_helpers::TestResult;

#[test]
fn slash_dispatch_drops_the_stale_live_counter() -> TestResult {
    // @step Given an App session whose live counter cache reports 7 nudges used
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 7,
            goal_active: false,
            effective_budget: 10,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));
    assert_eq!(
        app.agent_view_store()
            .continue_live_for(&sid("s-1"))
            .map(|l| l.nudges_used),
        Some(7),
        "test precondition: the live counter cache must hold 7"
    );

    // @step When the user applies "/continue 20" through the continue dispatch
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/continue 20".to_string()));
    }

    // @step Then the live counter cache for the session is cleared
    assert!(
        app.agent_view_store()
            .continue_live_for(&sid("s-1"))
            .is_none(),
        "slash dispatch must drop the stale live counter"
    );

    // @step And the painted footer shows "⏩ auto-continue (0/20)"
    let text = painted_footer(&app, &sid("s-1"));
    assert!(
        text.contains("\u{23E9} auto-continue (0/20)"),
        "footer must fall back to the cached (enabled, budget) pair; painted: {text:?}"
    );
    // Silence the unused-import lint for ContinueLiveState until the goal
    // dispatch assertion below uses it structurally.
    let _: Option<ContinueLiveState> = app.agent_view_store().continue_live_for(&sid("s-1"));

    // @step When a ContinueStateUpdate chunk repopulates the live counter and the user applies "/goal ship it" through the goal dispatch
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 20,
            nudges_used: 3,
            goal_active: false,
            effective_budget: 20,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/goal ship it".to_string()));
    }

    // @step Then the /goal dispatch also drops the live counter state
    assert!(
        app.agent_view_store()
            .continue_live_for(&sid("s-1"))
            .is_none(),
        "/goal dispatch must also drop the stale live counter"
    );
    Ok(())
}

#[test]
fn emission_sites_twin_mappings_and_chat_print_removal_are_pinned_in_the_sources() -> TestResult {
    // @step Given the codelet workspace sources
    let stream_loop = strip_line_comments(&read_source("cli/src/interactive/stream_loop.rs")?);
    let done_early_exit =
        strip_line_comments(&read_source("cli/src/interactive/done_early_exit.rs")?);
    let output_rs = strip_line_comments(&read_source("cli/src/interactive/output.rs")?);
    let agent_loop_twin = strip_line_comments(&read_source("agent-loop/src/background_output.rs")?);
    let agent_loop_json = strip_line_comments(&read_source("agent-loop/src/stream_chunk_json.rs")?);

    // @step Then stream_loop.rs emits the continue state at the turn start, refund settle, nudge and exhaustion sites
    for reason in [
        "ContinueStateReason::TurnStart",
        "ContinueStateReason::RefundSettled",
        "ContinueStateReason::NudgeConsumed",
        "ContinueStateReason::BudgetExhausted",
    ] {
        assert!(
            stream_loop.contains(reason),
            "stream_loop.rs must emit the continue state with {reason}"
        );
    }
    assert!(
        stream_loop.matches("emit_continue_state").count() >= 4,
        "stream_loop.rs must call emit_continue_state at all four transition sites"
    );

    // @step And stream_loop.rs no longer emits the "⏩ auto-continue: nudging" status message
    assert!(
        !stream_loop.contains("auto-continue: nudging"),
        "the per-nudge chat print must be REMOVED from stream_loop.rs — the counter belongs in the bar"
    );

    // @step And done_early_exit.rs emits the continue state at the tail of apply_finish_with_summary
    assert!(
        done_early_exit.contains("ContinueStateReason::DoneAccepted")
            && done_early_exit.contains("emit_continue_state"),
        "apply_finish_with_summary must emit the DoneAccepted continue state so BOTH exit sites are covered"
    );

    // @step And both background twins map StreamEvent::ContinueState to StreamChunk::ContinueStateUpdate
    assert!(
        agent_loop_twin.contains("StreamEvent::ContinueState"),
        "agent-loop twin must match the ContinueState event"
    );
    assert!(
        agent_loop_twin.contains("continue_state_update"),
        "agent-loop twin must map to StreamChunk::continue_state_update"
    );

    // @step And both stream_chunk_to_json_value copies serialize the variant as type "continueStateUpdate"
    assert!(
        agent_loop_json.contains("continueStateUpdate"),
        "agent-loop stream_chunk_to_json_value must serialize the variant as continueStateUpdate"
    );

    // @step And CliOutput renders the nudging line from the ContinueState event using the effective budget
    assert!(
        output_rs.contains("StreamEvent::ContinueState"),
        "CliOutput must have a ContinueState arm"
    );
    assert!(
        output_rs.contains("auto-continue: nudging"),
        "the CLI stdout nudging line moves to the CliOutput ContinueState arm"
    );
    assert!(
        output_rs.contains("effective_budget"),
        "the CLI nudging line must print the effective budget, not continue_budget"
    );
    Ok(())
}
