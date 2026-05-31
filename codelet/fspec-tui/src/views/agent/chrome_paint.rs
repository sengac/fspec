//! RPC-093 helper — `AgentView` chrome assembly extracted from
//! `views/agent.rs` to keep the orchestrator under its 300-LoC ceiling.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::store::AgentViewStore;

use super::footer::SessionFooter;
use super::header::SessionHeader;
use super::role_banner::RoleBanner;

/// Pre-split chrome rectangles for `render_with_store`. The split
/// itself stays in `views/agent.rs` (pinned by source_shape_rpc013).
pub struct ChromeAreas {
    pub header: Rect,
    pub role: Rect,
    pub scrollback: Rect,
    pub footer: Rect,
    pub input: Rect,
}

/// Build + paint the SessionHeader + optional RoleBanner.
pub fn paint_header_and_role(
    areas: &ChromeAreas,
    buf: &mut Buffer,
    store: &AgentViewStore,
    sid: Option<&codelet_rpc_types::SessionId>,
    is_loading: bool,
) {
    let model = sid.and_then(|s| store.model_info_for(s));
    let thinking = sid
        .and_then(|s| store.thinking_level_for(s).copied())
        .unwrap_or(codelet_rpc_types::ThinkingLevel::Off);
    let tokens = sid
        .and_then(|s| store.token_state_for(s).copied())
        .unwrap_or_default();
    let bound = sid.and_then(|s| store.work_unit_context_for(s));
    let work_unit_id = bound.map(|c| c.id.as_str()).or_else(|| store.current_work_unit_id());
    let work_unit_status = bound
        .map(|c| c.status.as_str())
        .or_else(|| store.current_work_unit_status());
    let is_debug_enabled = sid.and_then(|s| store.debug_enabled_for(s)).unwrap_or(false);
    let subordinate_label = sid.and_then(|s| {
        super::header_build::format_subordinate_label(store.supervisors_for(s))
    });
    SessionHeader {
        session_index: store.session_index(),
        model,
        thinking,
        tokens,
        work_unit_id,
        work_unit_status,
        is_isolated: false,
        is_debug_enabled,
        is_select_mode: false,
        tokens_per_second: None,
        reasoning_tokens: 0,
        compaction_reduction: None,
        is_loading,
        subordinate_label: subordinate_label.as_deref(),
    }
    .render(areas.header, buf);

    if areas.role.height > 0 {
        if let Some(role_text) = sid.and_then(|s| store.role_for(s)) {
            RoleBanner { role_text }.render(areas.role, buf);
        }
    }
}

/// Paint the SessionFooter.
pub fn paint_footer(
    areas: &ChromeAreas,
    buf: &mut Buffer,
    store: &AgentViewStore,
    sid: Option<&codelet_rpc_types::SessionId>,
) {
    let compaction_progress = sid.and_then(|s| store.compaction_progress_for(s));
    let supervisor_pending_count = sid
        .map(|s| store.supervisor_pending_count_for(s))
        .unwrap_or(0);
    SessionFooter {
        workspace: store.workspace(),
        compaction_progress,
        supervisor_pending_count,
    }
    .render(areas.footer, buf);
}
