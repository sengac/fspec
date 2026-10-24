#![doc = "Feature: spec/features/background-response-attempt-rollback.feature"]

use codelet_fspec_tui::store::agent_view::SessionContext;
use codelet_fspec_tui::views::agent::{SelectionMode, TurnDir};
use codelet_rpc_types::{
    NotificationSeverity, SessionId, StreamChunk, ToolCallInfo, ToolResultInfo,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn boundary(name: &str) -> Result<StreamChunk, serde_json::Error> {
    serde_json::from_value(serde_json::json!(name))
}

fn bodies(ctx: &SessionContext) -> Vec<String> {
    ctx.scrollback
        .chunks()
        .iter()
        .filter_map(|chunk| chunk.source.as_ref().map(|source| source.text.clone()))
        .collect()
}

/// @step Given earlier assistant text and reasoning and a completed tool card
/// @step When a later response streams text and reasoning
/// @step Then its deltas are visible before completion
/// @step When that response is aborted and a replacement completes
/// @step Then only the abandoned text and reasoning disappear
/// @step And the earlier output tool card and reconnect notification remain
#[test]
fn ui_retracts_only_the_latest_attempt_while_preserving_streaming() -> TestResult {
    let mut ctx = SessionContext::new(SessionId::new("rollback"));
    ctx.record_chunk(&boundary("ResponseAttemptStarted")?);
    ctx.record_chunk(&StreamChunk::text("earlier answer".into()));
    ctx.record_chunk(&StreamChunk::thinking("earlier reasoning".into()));
    ctx.record_chunk(&StreamChunk::tool_call(ToolCallInfo {
        id: "tool".into(),
        name: "Read".into(),
        input: "{}".into(),
    }));
    ctx.record_chunk(&StreamChunk::tool_result(ToolResultInfo {
        tool_call_id: "tool".into(),
        content: "completed tool result".into(),
        is_error: false,
    }));
    let earlier = bodies(&ctx);
    ctx.record_chunk(&boundary("ResponseAttemptStarted")?);
    ctx.record_chunk(&StreamChunk::text("abandoned text".into()));
    ctx.record_chunk(&StreamChunk::thinking("abandoned reasoning".into()));
    assert!(bodies(&ctx).iter().any(|body| body == "abandoned text"));
    assert!(bodies(&ctx)
        .iter()
        .any(|body| body == "abandoned reasoning"));
    ctx.scrollback.enter_item_mode();
    ctx.scrollback.navigate_turn(TurnDir::Up);
    assert!(ctx.scrollback.selected_seq().is_some());
    assert_eq!(
        ctx.scrollback
            .selected_index()
            .and_then(|index| ctx.scrollback.chunks()[index].source.as_ref())
            .map(|source| source.text.as_str()),
        Some("abandoned reasoning")
    );
    ctx.record_chunk(&StreamChunk::user_notification(
        "Reconnecting".into(),
        NotificationSeverity::Info,
    ));
    ctx.record_chunk(&boundary("ResponseAttemptAborted")?);
    assert!(ctx.scrollback.selected_seq().is_none());
    ctx.record_chunk(&boundary("ResponseAttemptStarted")?);
    ctx.record_chunk(&StreamChunk::thinking("replacement reasoning".into()));
    ctx.record_chunk(&StreamChunk::text("replacement answer".into()));
    ctx.record_chunk(&StreamChunk::Done);
    let after = bodies(&ctx);
    for body in earlier.iter().filter(|body| !body.is_empty()) {
        assert!(after.contains(body), "earlier output missing: {after:?}");
    }
    assert!(after.contains(&"Reconnecting".into()));
    assert!(after.contains(&"replacement answer".into()));
    assert!(after.contains(&"replacement reasoning".into()));
    assert!(!after.iter().any(|body| body.contains("abandoned")));
    assert!(ctx.in_flight_assistant.is_none());
    assert!(ctx.in_flight_thinking.is_none());
    let rendered = ctx
        .scrollback
        .chunks()
        .iter()
        .flat_map(|chunk| &chunk.lines)
        .flat_map(|line| &line.spans)
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(!rendered.contains("abandoned"));
    Ok(())
}

/// @step Given earlier completed output and a live response attempt with text and reasoning
/// @step When another user prompt is queued before the response attempt is aborted
/// @step And a replacement response completes
/// @step Then the abandoned text and reasoning disappear
/// @step And the queued prompt earlier output and replacement output remain visible
#[test]
fn queued_input_preserves_rollback_of_the_active_response_attempt() {
    let mut ctx = SessionContext::new(SessionId::new("queued-input-rollback"));
    ctx.record_chunk(&StreamChunk::text("earlier output".into()));
    ctx.record_chunk(&StreamChunk::Done);
    ctx.record_chunk(&StreamChunk::ResponseAttemptStarted);
    ctx.record_chunk(&StreamChunk::text("abandoned text".into()));
    ctx.record_chunk(&StreamChunk::thinking("abandoned reasoning".into()));
    ctx.record_chunk(&StreamChunk::user_input("queued prompt".into()));
    ctx.record_chunk(&StreamChunk::text("abandoned after queue".into()));
    ctx.record_chunk(&StreamChunk::thinking(
        "abandoned thought after queue".into(),
    ));

    ctx.record_chunk(&StreamChunk::ResponseAttemptAborted);
    ctx.record_chunk(&StreamChunk::ResponseAttemptStarted);
    ctx.record_chunk(&StreamChunk::thinking("replacement reasoning".into()));
    ctx.record_chunk(&StreamChunk::text("replacement output".into()));
    ctx.record_chunk(&StreamChunk::Done);

    let after = bodies(&ctx);
    assert!(
        !after.iter().any(|body| body.contains("abandoned")),
        "queued input must not prevent active-attempt rollback: {after:?}"
    );
    for expected in [
        "earlier output",
        "queued prompt",
        "replacement reasoning",
        "replacement output",
    ] {
        assert!(after.iter().any(|body| body == expected));
    }
    assert!(ctx.in_flight_assistant.is_none());
    assert!(ctx.in_flight_thinking.is_none());
}

/// @step Given turn selection is focused on an abandoned response attempt
/// @step When the response attempt is aborted
/// @step And surviving or replacement output is available
/// @step Then either arrow key selects an available turn without toggling selection mode
/// @step And further arrow navigation moves between available turns
#[test]
fn turn_navigation_recovers_after_the_selected_attempt_is_retracted() {
    for direction in [TurnDir::Up, TurnDir::Down] {
        for has_survivors in [false, true] {
            let mut ctx = SessionContext::new(SessionId::new("rollback-navigation"));
            if has_survivors {
                ctx.record_chunk(&StreamChunk::text("earlier answer".into()));
                ctx.record_chunk(&StreamChunk::thinking("earlier reasoning".into()));
                ctx.record_chunk(&StreamChunk::Done);
            }
            ctx.record_chunk(&StreamChunk::ResponseAttemptStarted);
            ctx.record_chunk(&StreamChunk::text("abandoned".into()));
            ctx.scrollback.enter_item_mode();
            let abandoned = ctx.scrollback.selected_seq();

            ctx.record_chunk(&StreamChunk::ResponseAttemptAborted);

            assert_eq!(ctx.scrollback.selection_mode(), SelectionMode::Item);
            assert_eq!(ctx.scrollback.selected_seq(), None);
            if !has_survivors {
                ctx.scrollback.navigate_turn(direction);
                assert_eq!(ctx.scrollback.selected_seq(), None);
                ctx.record_chunk(&StreamChunk::ResponseAttemptStarted);
                ctx.record_chunk(&StreamChunk::text("replacement".into()));
                ctx.record_chunk(&StreamChunk::thinking("replacement reasoning".into()));
            }

            ctx.scrollback.navigate_turn(direction);

            assert_eq!(
                ctx.scrollback.selected_seq(),
                ctx.scrollback.chunks().last().map(|chunk| chunk.seq),
                "navigation must recover a selection after rollback"
            );
            assert_ne!(ctx.scrollback.selected_seq(), abandoned);
            let recovered = ctx.scrollback.selected_seq();
            ctx.scrollback.navigate_turn(TurnDir::Up);
            assert_ne!(ctx.scrollback.selected_seq(), recovered);
            ctx.scrollback.navigate_turn(TurnDir::Down);
            assert_eq!(ctx.scrollback.selected_seq(), recovered);
        }
    }
}

/// @step Given assistant text and reasoning already occupy streaming blocks
/// @step When another response attempt appends to those blocks and is aborted
/// @step Then the original block contents and streaming cursors are restored
/// @step And an abort without another start changes nothing
#[test]
fn start_checkpoints_existing_streaming_blocks_without_clearing_them() -> TestResult {
    let mut ctx = SessionContext::new(SessionId::new("rollback"));
    ctx.record_chunk(&StreamChunk::text("earlier 🦀".into()));
    ctx.record_chunk(&StreamChunk::thinking("earlier thought".into()));
    let original = bodies(&ctx);
    let cursors = (ctx.in_flight_assistant, ctx.in_flight_thinking);
    ctx.record_chunk(&boundary("ResponseAttemptStarted")?);
    ctx.record_chunk(&StreamChunk::text(" abandoned".into()));
    ctx.record_chunk(&StreamChunk::thinking(" abandoned".into()));
    assert!(bodies(&ctx).iter().all(|body| body.ends_with(" abandoned")));
    ctx.record_chunk(&boundary("ResponseAttemptAborted")?);
    assert_eq!(bodies(&ctx), original);
    assert_eq!((ctx.in_flight_assistant, ctx.in_flight_thinking), cursors);
    ctx.record_chunk(&StreamChunk::text(" kept".into()));
    let after = bodies(&ctx);
    ctx.record_chunk(&boundary("ResponseAttemptAborted")?);
    assert_eq!(bodies(&ctx), after);
    Ok(())
}
