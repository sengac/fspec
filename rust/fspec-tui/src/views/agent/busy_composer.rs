//! **TUI-115** — the composer while the focused session is busy.
//!
//! Feature: spec/features/running-composer-steer-and-queue-hints.feature
//!
//! While the session is Running or Compacting the input area stacks, top
//! to bottom: the activity row (spinner and status), one row per queued
//! input (`Queued #n: text`), the editable composer, and a hint row that
//! names what Enter and Alt+Enter do. The composer is never replaced by
//! the spinner, so a draft is never edited blind.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::{SessionId, SessionStatus};

use crate::store::AgentViewStore;
use crate::terminal::sanitize::sanitize_for_terminal;

use super::input_transition::{paint_activity_row, render_idle_input};
use super::{multiline_input_render, AgentView};

/// Which busy turn the focused session is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BusyKind {
    Running,
    Compacting,
}

impl BusyKind {
    /// The busy kind of `sid`'s last observed status; `None` when idle.
    pub(super) fn of(store: &AgentViewStore, sid: Option<&SessionId>) -> Option<Self> {
        match sid.and_then(|session| store.session_status_for(session)) {
            Some(SessionStatus::Running) => Some(Self::Running),
            Some(SessionStatus::Compacting) => Some(Self::Compacting),
            _ => None,
        }
    }

    /// The hint row under the composer.
    pub(super) fn hint(self) -> &'static str {
        match self {
            Self::Running => "Enter steer · Alt+Enter queue · Esc interrupt",
            Self::Compacting => "Enter or Alt+Enter queue · runs after compaction",
        }
    }
}

/// Queued inputs of `sid`, each as its one-line `Queued #n: text` row.
fn queued_rows(store: &AgentViewStore, sid: Option<&SessionId>) -> Vec<String> {
    match sid.and_then(|session| store.session_context_for(session)) {
        Some(context) => context
            .busy_inputs
            .queued()
            .iter()
            .map(|queued| {
                let one_line = sanitize_for_terminal(&queued.text.replace('\n', " "));
                format!("Queued #{}: {one_line}", queued.n)
            })
            .collect(),
        // A session without an agent-view context has queued nothing.
        None => Vec::new(),
    }
}

/// Hands out consecutive rows of the busy stack, clipped to its bottom.
struct RowStack {
    area: Rect,
    y: u16,
}

impl RowStack {
    fn next(&mut self, height: u16) -> Rect {
        let bottom = self.area.y.saturating_add(self.area.height);
        let rect = Rect::new(
            self.area.x,
            self.y,
            self.area.width,
            height.min(bottom.saturating_sub(self.y)),
        );
        self.y = self.y.saturating_add(rect.height);
        rect
    }

    /// Rows painted so far, from the stack's top.
    fn used(&self) -> u16 {
        self.y.saturating_sub(self.area.y)
    }
}

fn paint_dim_row(area: Rect, buf: &mut Buffer, text: String) {
    let style = Style::default().add_modifier(Modifier::DIM);
    Paragraph::new(Line::from(Span::styled(text, style))).render(area, buf);
}

impl AgentView {
    /// Input-area rows while busy: activity + queued + composer + hint.
    pub(super) fn busy_input_height(
        &self,
        store: &AgentViewStore,
        sid: Option<&SessionId>,
        area_width: u16,
    ) -> u16 {
        let composer = self
            .input
            .visible_rows_for_width(multiline_input_render::input_body_width(area_width));
        let queued = queued_rows(store, sid)
            .iter()
            .fold(0u16, |rows, _| rows.saturating_add(1));
        composer.saturating_add(queued).saturating_add(2)
    }

    /// Paint the busy stack into `padded` (the input area minus side
    /// pads). The hardware-cursor anchor is shifted onto the composer.
    pub(super) fn paint_busy_input_area(
        &mut self,
        input_area: Rect,
        padded: Rect,
        buf: &mut Buffer,
        store: &AgentViewStore,
        sid: Option<&SessionId>,
        kind: BusyKind,
    ) {
        let queued = queued_rows(store, sid);
        let mut rows = RowStack {
            area: padded,
            y: padded.y,
        };
        let activity = rows.next(1);
        if let Some(context) = sid.and_then(|session| store.session_context_for(session)) {
            paint_activity_row(
                activity,
                buf,
                &context.input_transition.state,
                context.provider_waiting.as_deref(),
            );
        }
        for text in queued {
            paint_dim_row(rows.next(1), buf, text);
        }
        let composer_top = rows.used();
        let composer = rows.next(padded.height.saturating_sub(composer_top).saturating_sub(1));
        let input_body_width = multiline_input_render::input_body_width(input_area.width);
        self.input.sync_viewport(input_body_width, composer.height);
        render_idle_input(composer, buf, &self.input);
        paint_dim_row(rows.next(1), buf, kind.hint().to_string());
        self.last_hitl_input_offset = Some(composer_top);
    }
}
