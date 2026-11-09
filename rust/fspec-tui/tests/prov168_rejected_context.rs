//! Feature: spec/features/rejected-anthropic-context-provenance.feature

use codelet_fspec_tui::{
    store::{AgentViewStore, TokenState},
    views::agent::header::SessionHeader,
};
use codelet_rpc_types::{
    ContextProvenance, RequestContextSnapshot, SessionId, StreamChunk, ThinkingLevel,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

fn render(tokens: TokenState) -> String {
    let area = Rect::new(0, 0, 160, 1);
    let mut buffer = Buffer::empty(area);
    SessionHeader {
        session_index: Some((1, 1)),
        model: None,
        thinking: ThinkingLevel::Off,
        tokens,
        work_unit_id: None,
        work_unit_status: None,
        is_isolated: false,
        is_debug_enabled: false,
        is_select_mode: false,
        tokens_per_second: None,
        reasoning_tokens: 0,
        compaction_reduction: None,
        is_loading: false,
        subordinate_label: None,
    }
    .render(area, &mut buffer);
    buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

#[test]
fn rejection_before_usage_leaves_occupancy_unknown() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a new Anthropic session has known model capacity but no measured usage
    let session_id = SessionId::new("anthropic-rejected-before-usage");
    let mut store = AgentViewStore::default();
    let snapshot = RequestContextSnapshot {
        model: "claude-opus-5-5".into(),
        generation: 0,
        request: 1,
        physical_request: None,
        input_tokens: None,
        output_tokens: None,
        capacity: Some(200_000),
        threshold: Some(160_000),
        provenance: ContextProvenance::Unknown,
    };
    assert!(snapshot.context_fill()?.is_none());
    store.apply_chunk_to_token_state(&session_id, &StreamChunk::ContextSnapshot { snapshot });
    let before = *store
        .token_state_for(&session_id)
        .ok_or("initial context snapshot must create token state")?;

    // @step When the request is rejected before returning usage
    store.apply_chunk_to_token_state(
        &session_id,
        &StreamChunk::error(
            "HTTP 400 invalid_request_error: Claude Code 2.1.3 does not support this model; version 2.1.280 or newer is required (claude_code_version_too_old)".into(),
        ),
    );

    // @step Then the header keeps context unknown rather than inventing zero percent or measured usage
    let after = *store
        .token_state_for(&session_id)
        .ok_or("rejection must retain token state")?;
    assert_eq!(after, before);
    assert_eq!(after.committed_generation, Some(0));
    assert_eq!(after.context_request, Some(1));
    assert_eq!(after.input_tokens, 0);
    assert_eq!(after.output_tokens, 0);
    assert!(after.estimated);
    let header = render(after);
    assert!(header.contains("tokens: 0↓ 0↑"), "{header}");
    assert!(header.contains("[context unknown]"), "{header}");
    assert!(!header.contains("[0%"), "{header}");
    Ok(())
}
