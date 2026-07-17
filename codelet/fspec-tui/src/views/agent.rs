//! AgentView — always-on container. RPC-029 layout: Header / RoleBanner /
//! Scrollback / Footer / Input. RPC-026: resume/search views early-return.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::scroll_viewport::WheelVelocity;
use crate::components::Action;
use crate::store::AgentViewStore;

use codelet_rpc_types::SessionStatus;

use input_transition::InputTransitionState;

pub mod animation;
pub mod chrome;
pub mod chrome_paint;
pub mod confirm_dialog;
mod cursor_anchor;
pub mod dispatch;
pub mod dispatch_mode_views;
mod dispatch_popups;
pub mod dispatch_select;
pub mod file_search_popup;
pub mod file_search_popup_rows;
pub mod footer;
pub mod header;
pub mod header_build;
mod hitl_keys;
pub mod hitl_prompt;
mod input_area;
pub mod input_transition;
pub mod merge_confirm_dialog;
pub mod mode_view_render;
pub mod mouse_dispatch;
pub mod multiline_input;
mod multiline_input_enter;
mod multiline_input_paste;
mod multiline_input_render;
mod multiline_input_select;
pub mod multiline_wrap;
mod pause_keys;
pub mod pause_prompt;
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
pub mod turn_modal;
mod turn_modal_select;

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
pub use scrollback::{ScrollState, ScrollbackList, SelectionMode, TurnDir};
pub use search_history_view::{SearchHistoryView, SearchHistoryViewOutcome};
pub use slash_command_popup::{PopupOutcome, SlashCommandPopup};
pub use slash_commands::{SlashCommand, SlashCommandAction, SLASH_COMMANDS};
pub use turn_modal::TurnContentModal;

/// RPC-013 placeholder footer hints.
pub const PLACEHOLDER_FOOTER_HINTS: &str = "Enter=send  Ctrl+C=interrupt  ESC=back";

/// RPC-019 placeholder hint. RPC-426: 'Ctrl+J' is the universal newline
/// binding (Emacs-style) — works on every terminal. 'Shift+Enter' is
/// best-effort (only on terminals with keyboard enhancement).
pub const INPUT_PLACEHOLDER_HINT: &str =
    "Type a message... 'Ctrl+J' newline, 'Shift+↑/↓' history, 'Shift+←/→' sessions, 'Tab' turns";

/// RPC-029: paint `color` over every cell of `area` (RPC-405: moved
/// to `chrome.rs`; re-exported so `super::paint_row_bg` callers work).
pub(crate) use chrome::paint_row_bg;

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
    /// RPC-381: turn-selection (SELECT) mode toggle. Mirrors the TS
    /// component-level `isTurnSelectMode`; when true, ↑/↓ navigate
    /// turn-to-turn, Enter is suppressed, Esc exits the mode locally,
    /// and the SessionHeader paints the `[SELECT]` badge.
    pub turn_select_mode: bool,
    /// RPC-382: turn content modal — `Some(seq)` ⇒ open for that turn.
    pub turn_modal_seq: Option<u64>,
    /// RPC-383: turn content modal scroll offset (first visible body row).
    pub turn_modal_offset: usize,
    /// COPY-008: modal text selection + cached body layout (semantics in `turn_modal_select`).
    pub turn_modal_selection: Option<crate::mouse::selection::Selection>,
    pub(crate) turn_modal_rows: Vec<String>,
    pub(crate) turn_modal_body_origin: Option<Rect>,
    pub(crate) last_render_area: Option<Rect>,
    pub(crate) last_scrollback_area: Option<Rect>,
    pub(crate) scrollback_wheel: WheelVelocity,
    pub(crate) spinner_started_at: Option<Instant>,
    pub(crate) last_is_compacting: bool,
    pub(crate) input_transition_state: InputTransitionState,
    pub(crate) last_spinner_line: Option<String>,
    pub(crate) animation_clock_ms: u64,
    /// RPC-406: `(session, kind)` of the pause prompt painted last frame.
    pub(crate) last_pause: Option<(codelet_rpc_types::SessionId, codelet_rpc_types::PauseKind)>,
    /// RPC-411: `(session, mode)` of the HITL prompt painted last frame.
    pub(crate) last_hitl: Option<(codelet_rpc_types::SessionId, hitl_keys::HitlKeyMode)>,
    /// RPC-412: freeform HITL header offset (rows to the "> " input line).
    pub(crate) last_hitl_input_offset: Option<u16>,
    pub(crate) recognizer: crate::mouse::gesture::SelectionRecognizer, // COPY-006
    pub(crate) text_selection_active: bool, // COPY-006: live scrollback selection.
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

    /// RPC-404/RPC-412 hardware cursor: geometry in
    /// [`MultiLineInput::hardware_cursor_in`], freeform header offset
    /// applied by [`cursor_anchor::anchored_input_area`].
    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        let area = self.last_input_area?;
        let anchored = cursor_anchor::anchored_input_area(area, self.last_hitl_input_offset);
        Some(self.input.hardware_cursor_in(anchored))
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

    /// Cursor gate — RPC-411 HITL-mode logic lives in `hitl_keys.rs`.
    pub fn is_cursor_visible(&self, session_status: Option<SessionStatus>) -> bool {
        self.is_cursor_visible_with_prompts(session_status)
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
        // RPC-405: height from the SAME area width the renderer derives
        // its body widths from. RPC-406: the pause prompt's wrapped
        // height wins while paused.
        let sid = store.current_session().cloned();
        let input_height = self.input_area_height(store, sid.as_ref(), area.width);
        let role_height: u16 = sid
            .as_ref()
            .and_then(|s| store.role_for(s))
            .map(|_| 1)
            .unwrap_or(0);
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
        chrome_paint::paint_header_and_role(
            &areas,
            buf,
            store,
            sid.as_ref(),
            is_loading,
            self.turn_select_mode,
        );

        self.last_scrollback_viewport = areas.scrollback.height;
        self.last_scrollback_area = Some(areas.scrollback);
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.scrollback.render_count_visited(areas.scrollback, buf);
        }
        chrome_paint::paint_footer(&areas, buf, store, sid.as_ref());

        // RPC-406: inline pause prompt OR spinner/transition/input
        // (impl in `input_area.rs` to keep this file under 300 LoC).
        self.paint_input_area(areas.input, buf, store, sid.as_ref());
        if let Some(p) = self.slash_popup.as_ref() {
            p.render(area, buf);
        } else if let Some(p) = self.file_popup.as_ref() {
            p.render(area, buf);
        }
        // RPC-382/383 + COPY-008: turn content modal overlay + selection.
        self.paint_turn_modal(area, buf, store);
    }
}
