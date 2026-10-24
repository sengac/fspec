//! Feature: spec/features/context-header-provenance.feature

use codelet_fspec_tui::{store::TokenState, views::agent::header::SessionHeader};
use codelet_rpc_types::{ContextProvenance, RequestContextSnapshot, StreamChunk, ThinkingLevel};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

fn render(tokens: TokenState) -> String {
    let area = Rect::new(0, 0, 160, 1);
    let mut buffer = Buffer::empty(area);
    SessionHeader {
        session_index: (1, 1),
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
fn scenario_header_distinguishes_estimated_and_unknown_context_from_measured_usage() {
    // @step Given estimated committed context followed by an unavailable request measurement
    let mut tokens = TokenState::default();
    let mut snapshot = RequestContextSnapshot {
        model: "model-a".into(),
        generation: 7,
        request: 1,
        physical_request: None,
        input_tokens: Some(200),
        output_tokens: Some(0),
        capacity: Some(2000),
        threshold: Some(1000),
        provenance: ContextProvenance::Estimated,
    };
    tokens.apply_chunk(&StreamChunk::ContextSnapshot {
        snapshot: snapshot.clone(),
    });
    // @step When the session header renders each context snapshot
    // @step Then estimated occupancy is labelled estimated
    assert!(
        render(tokens).contains("[20% estimated]"),
        "estimated context looks measured: {}",
        render(tokens)
    );
    snapshot.request = 2;
    snapshot.input_tokens = None;
    snapshot.provenance = ContextProvenance::Unknown;
    tokens.apply_chunk(&StreamChunk::ContextSnapshot { snapshot });
    // @step And unavailable occupancy is labelled unknown rather than zero percent
    assert!(
        render(tokens).contains("[context unknown]"),
        "unknown context looks empty: {}",
        render(tokens)
    );
    assert!(!render(tokens).contains("[0%]"));
}
