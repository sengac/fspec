//! BUG-377 — which session an agent pane's composer belongs to, without defaulting a missing session context.
//!
//! Feature: spec/features/agent-panes-report-missing-session-context.feature

use codelet_rpc_types::SessionId;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Widget};

use crate::store::{AgentViewStore, SessionContext};

pub(super) enum PaneComposer<'a> {
    /// No session is open: a fresh composer with nothing running.
    NoSession,
    /// The pane's session is open.
    Open(&'a SessionContext),
    /// The pane names a session that is no longer open.
    Missing(&'a SessionId),
}

pub(super) fn pane_composer<'a>(
    store: &'a AgentViewStore,
    sid: Option<&'a SessionId>,
) -> PaneComposer<'a> {
    match sid {
        None => PaneComposer::NoSession,
        Some(id) => match store.session_context_for(id) {
            Some(context) => PaneComposer::Open(context),
            None => PaneComposer::Missing(id),
        },
    }
}

/// Paint "session <id> is not open" in the input row and log the broken pane.
pub(super) fn paint_missing_session(area: Rect, buf: &mut Buffer, id: &SessionId) {
    tracing::error!(session = %id, "agent pane names a session that is not open");
    let row = Rect {
        height: area.height.min(1),
        ..area
    };
    Paragraph::new(format!("session {id} is not open"))
        .style(Style::default().fg(Color::Red))
        .render(row, buf);
}
