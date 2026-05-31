//! AgentView — always-on container. RPC-029 layout:
//! Header(1) / RoleBanner(0|1) / Scrollback(flex) / SessionFooter(1) /
//! Input(visible_rows). RPC-026: resume_view/search_view early-return.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::scroll_viewport::WheelVelocity;
use crate::components::Action;
use crate::store::AgentViewStore;

use codelet_rpc_types::SessionStatus;

use input_transition::InputTransitionState;

pub mod chrome;
pub mod chrome_paint;
pub mod confirm_dialog;
pub mod dispatch;
pub mod dispatch_mode_views;
pub mod file_search_popup;
pub mod file_search_popup_rows;
pub mod footer;
pub mod header;
pub mod header_build;
pub mod input_transition;
pub mod merge_confirm_dialog;
pub mod mode_view_render;
pub mod mouse_dispatch;
pub mod multiline_input;
pub mod popups;
pub mod rendered_chunk;
pub mod resume_session_view;
pub mod role_banner;
pub mod scrollback;
pub mod scrollback_paint;
pub mod search_history_view;
pub mod search_history_view_render;
pub mod slash_command_popup;
pub mod slash_command_popup_rows;
pub mod slash_commands;
pub mod spinner;
pub mod text_wrap;
pub mod transition_driver;

pub use confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
pub use file_search_popup::{FilePopupOutcome, FileSearchPopup};
pub use footer::SessionFooter;
pub use header::SessionHeader;
pub use merge_confirm_dialog::{
    MergeConfirmDialog, MergeConfirmDialogOutcome, MERGE_CONFIRM_DIALOG_ID,
};
pub use multiline_input::{InputEventOutcome, MultiLineInput};
pub use popups::{classify_buffer, splice_file_selection, PopupTrigger};
pub use rendered_chunk::{ChunkKind, ChunkSource, RenderedChunk};
pub use resume_session_view::{ResumeSessionView, ResumeSessionViewOutcome};
pub use role_banner::RoleBanner;
pub use scrollback::{ScrollState, ScrollbackList};
pub use search_history_view::{SearchHistoryView, SearchHistoryViewOutcome};
pub use slash_command_popup::{PopupOutcome, SlashCommandPopup};
pub use slash_commands::{SlashCommand, SlashCommandAction, SLASH_COMMANDS};

/// RPC-013 placeholder footer hints.
pub const PLACEHOLDER_FOOTER_HINTS: &str = "Enter=send  Ctrl+C=interrupt  ESC=back";

/// RPC-019 placeholder hint painted inside the input box when empty.
pub const INPUT_PLACEHOLDER_HINT: &str =
    "Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)";

/// RPC-029: paint `color` over every cell of `area`.
pub(crate) fn paint_row_bg(area: Rect, buf: &mut Buffer, color: Color) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            buf[(x, y)].set_bg(color);
        }
    }
}

/// AgentView — owns presentation state only.
#[derive(Default)]
pub struct AgentView {
    pub input: MultiLineInput,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub last_input_area: Option<Rect>,
    pub(crate) last_scrollback_viewport: u16,
    pub slash_popup: Option<SlashCommandPopup>,
    pub file_popup: Option<FileSearchPopup>,
    pub resume_view: Option<ResumeSessionView>,
    pub search_view: Option<SearchHistoryView>,
    pub(crate) last_render_area: Option<Rect>,
    pub(crate) last_scrollback_area: Option<Rect>,
    pub(crate) scrollback_wheel: WheelVelocity,
    pub(crate) spinner_started_at: Option<Instant>,
    pub(crate) last_is_compacting: bool,
    pub(crate) input_transition_state: InputTransitionState,
    pub(crate) last_spinner_line: Option<String>,
    pub(crate) animation_clock_ms: u64,
}

impl AgentView {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            action_tx: Some(action_tx),
            ..Self::default()
        }
    }

    pub fn chunk_count(&self, store: &AgentViewStore) -> usize {
        store
            .current_session_context()
            .map(|c| c.scrollback.chunk_count())
            .unwrap_or(0)
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        let area = self.last_input_area?;
        let (row, col) = self.input.cursor();
        let x = area
            .x
            .saturating_add(1)
            .saturating_add(2)
            .saturating_add(col as u16);
        let y = area.y.saturating_add(row as u16);
        Some((x, y))
    }

    /// RPC-093 rule [6]: busy iff session is Running or Compacting.
    pub fn is_busy(&self) -> bool {
        self.spinner_started_at.is_some()
    }

    /// RPC-093: true iff the input row is mid-finish-animation
    /// (`Hiding` or `Showing`). The run loop reads this to keep
    /// drawing every tick AFTER the session has gone Idle so the
    /// 5 char/17ms sweep advances instead of freezing at full
    /// captured text.
    pub fn is_input_animating(&self) -> bool {
        self.input_transition_state.is_animating()
    }

    /// RPC-093 rule [8]: cursor visible only when (a) status is not
    /// Running/Compacting AND (b) transition is Idle.
    pub fn is_cursor_visible_for(
        session_status: Option<SessionStatus>,
        transition: &InputTransitionState,
    ) -> bool {
        if matches!(
            session_status,
            Some(SessionStatus::Running) | Some(SessionStatus::Compacting)
        ) {
            return false;
        }
        transition.is_cursor_painted()
    }

    pub fn is_cursor_visible(&self, session_status: Option<SessionStatus>) -> bool {
        Self::is_cursor_visible_for(session_status, &self.input_transition_state)
    }

    pub fn push_line<S: Into<String>>(&mut self, store: &mut AgentViewStore, line: S) {
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.push_line(line);
        }
    }

    pub fn reset_scrollback(&mut self, store: &mut AgentViewStore) {
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.reset_scrollback();
        }
        self.input.reset();
        self.slash_popup = None;
        self.file_popup = None;
    }

    pub(crate) fn emit(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    pub fn record_chunk(
        &mut self,
        store: &mut AgentViewStore,
        chunk: &codelet_rpc_types::StreamChunk,
    ) {
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.record_chunk(chunk);
        }
    }

    pub fn set_file_search_results(&mut self, matches: Vec<String>) {
        if let Some(p) = self.file_popup.as_mut() {
            p.set_matches(matches);
        }
    }

    /// RPC-095 + RPC-093: per-frame animation tick. Returns
    /// `(session_status, is_loading)`.
    fn tick_animation(
        &mut self,
        store: &AgentViewStore,
        sid: Option<&codelet_rpc_types::SessionId>,
    ) -> (Option<SessionStatus>, bool) {
        let session_status = sid.and_then(|s| store.session_status_for(s).copied());
        let is_busy = matches!(
            session_status,
            Some(SessionStatus::Running) | Some(SessionStatus::Compacting)
        );
        if is_busy && self.spinner_started_at.is_none() {
            self.spinner_started_at = Some(Instant::now());
        } else if !is_busy {
            self.spinner_started_at = None;
        }
        let is_loading = matches!(session_status, Some(SessionStatus::Running));
        self.animation_clock_ms = self.animation_clock_ms.saturating_add(16);
        let elapsed_ms = self
            .spinner_started_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.input_transition_state = transition_driver::advance_transition(
            session_status,
            &self.input_transition_state,
            self.last_spinner_line.as_deref(),
            elapsed_ms,
            self.animation_clock_ms,
        );
        (session_status, is_loading)
    }

    /// RPC-029 layout. Mode views early-return; otherwise paints chrome + input + popups.
    pub fn render_with_store(&mut self, area: Rect, buf: &mut Buffer, store: &mut AgentViewStore) {
        self.last_render_area = Some(area);
        if let Some(v) = self.resume_view.as_ref() {
            v.render(area, buf);
            return;
        }
        if let Some(v) = self.search_view.as_ref() {
            v.render(area, buf);
            return;
        }
        let input_height = self.input.visible_rows();
        let sid = store.current_session().cloned();
        let role_height: u16 = sid.as_ref().and_then(|s| store.role_for(s)).map(|_| 1).unwrap_or(0);
        // RPC-029 layout: Header(1), RoleBanner(0|1), Scrollback flex Min(0), Footer Length(1), Input Length(input_height).
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(role_height),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(input_height),
            ])
            .split(area);
        let areas = chrome_paint::ChromeAreas {
            header: split[0],
            role: split[1],
            scrollback: split[2],
            footer: split[3],
            input: split[4],
        };
        self.last_input_area = Some(areas.input);

        let (session_status, is_loading) = self.tick_animation(store, sid.as_ref());
        self.last_is_compacting = matches!(session_status, Some(SessionStatus::Compacting));
        chrome_paint::paint_header_and_role(&areas, buf, store, sid.as_ref(), is_loading);

        self.last_scrollback_viewport = areas.scrollback.height;
        self.last_scrollback_area = Some(areas.scrollback);
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.scrollback.render_count_visited(areas.scrollback, buf);
        }
        chrome_paint::paint_footer(&areas, buf, store, sid.as_ref());

        // RPC-029: input has no border; paddingX=1.
        let pad = areas.input.width.min(1);
        let padded = Rect {
            x: areas.input.x + pad,
            y: areas.input.y,
            width: areas.input.width.saturating_sub(pad * 2),
            height: areas.input.height,
        };
        input_transition::paint_input_or_spinner(padded, buf, &self.input, &self.input_transition_state);
        if let Some(line) = transition_driver::cached_spinner_line(&self.input_transition_state) {
            self.last_spinner_line = Some(line);
        }
        if let Some(p) = self.slash_popup.as_ref() {
            p.render(area, buf);
        } else if let Some(p) = self.file_popup.as_ref() {
            p.render(area, buf);
        }
    }
}
