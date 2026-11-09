//! BUG-355: scrollback helpers for tests/rpc437_session_cleared_tui_consumption.rs
//! (the support-module pattern: helpers here, every test in the one root
//! file).

use codelet_fspec_tui::{App, RenderedChunk};
use codelet_rpc_types::{SessionId, SessionStatus};
use codelet_test_helpers::OrFail;
use ratatui::text::Line;

/// Count the scrollback chunks held by the SessionContext for `id`.
pub fn session_chunk_count(
    app: &App,
    id: &SessionId,
) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(app
        .agent_view_store()
        .session_context_for(id)
        .or_fail("SessionContext present for the counted id")?
        .scrollback
        .chunk_count())
}

/// Whether the SessionContext for `id` holds an empty input draft.
pub fn input_draft_is_empty(
    app: &App,
    id: &SessionId,
) -> Result<bool, codelet_test_helpers::TestError> {
    Ok(app
        .agent_view_store()
        .session_context_for(id)
        .or_fail("SessionContext present for the checked id")?
        .input_draft
        .is_empty())
}

/// Whether `id`'s session status folded to Cleared (RPC-100 chrome).
pub fn is_status_cleared(app: &App, id: &SessionId) -> bool {
    app.agent_view_store().session_status_for(id) == Some(&SessionStatus::Cleared)
}

/// Push `count` raw scrollback chunks into the SessionContext for `id`.
pub fn seed_chunks(
    app: &mut App,
    id: &SessionId,
    count: usize,
) -> Result<(), codelet_test_helpers::TestError> {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .or_fail("SessionContext present for seeded id")?;
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            row_ends: Vec::new(),
            seq: i as u64,
            lines: vec![Line::from(format!("seed-{i}"))],
            source: None,
        });
    }
    Ok(())
}

/// Flatten every visible chunk's text in `id`'s scrollback into a
/// single String.
pub fn session_scrollback_text(
    app: &App,
    id: &SessionId,
) -> Result<String, codelet_test_helpers::TestError> {
    Ok(app
        .agent_view_store()
        .session_context_for(id)
        .or_fail("SessionContext present for the read id")?
        .scrollback
        .visible_window(1024)
        .iter()
        .flat_map(|chunk| chunk.lines.iter().map(ToString::to_string))
        .collect())
}
