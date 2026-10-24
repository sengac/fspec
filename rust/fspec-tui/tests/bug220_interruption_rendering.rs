//! Feature: spec/features/interruption-reason-rendering.feature

use codelet_fspec_tui::store::agent_view::SessionContext;
use codelet_rpc_types::{SessionId, StreamChunk};
use serde_json::json;

#[test]
fn scenario_scrollback_distinguishes_user_and_detector_interruption_with_legacy_fallback(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given user and loopDetector interruption events and a legacy reasonless event
    for (reason, expected) in [
        (Some("user"), "⚠ Interrupted by user"),
        (Some("loopDetector"), "⚠ Interrupted by loop detector"),
        (None, "⚠ Interrupted"),
    ] {
        let chunk = if let Some(reason) = reason {
            serde_json::from_value(
                json!({"InterruptedWithReason":{"queuedInputs":[],"reason":reason}}),
            )?
        } else {
            StreamChunk::interrupted(Vec::new())
        };
        let mut context = SessionContext::new(SessionId::new("interrupted"));
        // @step When the events enter session scrollback
        context.record_chunk(&chunk);
        // @step Then the notices name explicit causes and keep the legacy notice generic
        let lines: Vec<_> = context
            .scrollback
            .visible_window(100)
            .into_iter()
            .filter_map(|chunk| chunk.source.as_ref().map(|source| source.text.clone()))
            .collect();
        assert_eq!(lines, [expected]);
    }
    Ok(())
}
