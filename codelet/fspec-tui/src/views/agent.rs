//! AgentView — the always-on container. Owns presentation state for
//! the input + popup overlays; reads scrollback + token chrome from
//! the per-session [`crate::store::SessionContext`] via
//! [`AgentViewStore`].
//!
//! Feature files: see `rpc012/rpc013/rpc018/rpc019/rpc020/rpc024/`
//! `rpc026/rpc029-*.feature`.
//!
//! RPC-029 layout (matches `src/tui/components/AgentView.tsx`):
//!   [SessionHeader(1) / RoleBanner(0|1) / Scrollback(flex) /
//!    SessionFooter(1) / Input(visible_rows)]
//!
//! The footer sits ABOVE the input row. Scrollback and input have no
//! borders. Header + footer paint a `#333333` row background and pad
//! horizontally by 1 column.
//!
//! RPC-026: when `resume_view` or `search_view` is `Some`, the render
//! path EARLY-RETURNS — those mode views paint into the entire area.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use ratatui::widgets::Widget;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::Action;
use crate::store::AgentViewStore;

pub mod chrome;
pub mod confirm_dialog;
pub mod dispatch;
pub mod file_search_popup;
pub mod file_search_popup_rows;
pub mod footer;
pub mod header;
pub mod header_build;
pub mod mode_view_render;
pub mod mouse_dispatch;
pub mod multiline_input;
pub mod popups;
pub mod resume_session_view;
pub mod role_banner;
pub mod scrollback;
pub mod search_history_view;
pub mod slash_command_popup;
pub mod slash_command_popup_rows;
pub mod slash_commands;

pub use confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
pub use file_search_popup::{FilePopupOutcome, FileSearchPopup};
pub use footer::SessionFooter;
pub use header::SessionHeader;
pub use multiline_input::{InputEventOutcome, MultiLineInput};
pub use popups::{classify_buffer, splice_file_selection, PopupTrigger};
pub use resume_session_view::{ResumeSessionView, ResumeSessionViewOutcome};
pub use role_banner::RoleBanner;
pub use scrollback::{ScrollState, ScrollbackList};
pub use search_history_view::{SearchHistoryView, SearchHistoryViewOutcome};
pub use slash_command_popup::{PopupOutcome, SlashCommandPopup};
pub use slash_commands::{SlashCommand, SlashCommandAction, SLASH_COMMANDS};

use ratatui::text::Line;

/// RPC-013 placeholder footer hints. RPC-029 stopped painting these
/// (the TS footer left side is empty), but the constant is kept here
/// so the RPC-013 source-shape invariant — which asserts `agent.rs`
/// contains the substrings `Enter=send`, `Ctrl+C=interrupt`,
/// `ESC=back` — continues to hold.
pub const PLACEHOLDER_FOOTER_HINTS: &str = "Enter=send  Ctrl+C=interrupt  ESC=back";

/// RPC-019 placeholder hint painted inside the input box when empty.
pub const INPUT_PLACEHOLDER_HINT: &str =
    "Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)";

/// RPC-029: paint `color` as the background of every cell of `area`.
/// Called by `SessionHeader::render` and `SessionFooter::render`
/// before their styled spans are painted, so ratatui's cell-merge
/// keeps the fg per-span while the bg stays uniform across the row.
pub(crate) fn paint_row_bg(area: Rect, buf: &mut Buffer, color: Color) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            buf[(x, y)].set_bg(color);
        }
    }
}

/// Pre-rendered chunk row keyed by chunk seq.
#[derive(Debug, Clone)]
pub struct RenderedChunk {
    pub seq: u64,
    pub lines: Vec<Line<'static>>,
}

/// AgentView — owns only presentation state. Session id + work-unit
/// linkage + scrollback live in `AgentViewStore` (RPC-024).
#[derive(Default)]
pub struct AgentView {
    pub input: MultiLineInput,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub last_input_area: Option<Rect>,
    /// Last observed scrollback viewport height (in rows). Updated each
    /// `render_with_store` call and consumed by App::dispatch when
    /// pushing chunks so stick-to-bottom math stays correct without
    /// requiring the render path to mutate the SessionContext.
    pub(crate) last_scrollback_viewport: u16,
    /// RPC-020: slash command palette (Some when active).
    pub slash_popup: Option<SlashCommandPopup>,
    /// RPC-020: `@file` search popup (Some when active).
    pub file_popup: Option<FileSearchPopup>,
    /// RPC-026: /resume full-screen mode view (Some when active).
    pub resume_view: Option<ResumeSessionView>,
    /// RPC-026: /search full-screen mode view (Some when active).
    pub search_view: Option<SearchHistoryView>,
    /// RPC-026: most-recent render area; used by dispatch.rs to size
    /// `handle_key`'s `visible_rows` argument for the mode views.
    pub(crate) last_render_area: Option<Rect>,
}

impl AgentView {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            action_tx: Some(action_tx),
            ..Self::default()
        }
    }

    /// Chunk count for the currently focused session, or 0 if no
    /// session is open.
    pub fn chunk_count(&self, store: &AgentViewStore) -> usize {
        store
            .current_session_context()
            .map(|c| c.scrollback.chunk_count())
            .unwrap_or(0)
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        let area = self.last_input_area?;
        let (row, col) = self.input.cursor();
        // RPC-029: input area has no border; only paddingX=1. The
        // prompt "> " adds 2 cells before the textarea body.
        let x = area
            .x
            .saturating_add(1)
            .saturating_add(2)
            .saturating_add(col as u16);
        let y = area.y.saturating_add(row as u16);
        Some((x, y))
    }

    /// Push a raw line into the currently focused session's scrollback.
    pub fn push_line<S: Into<String>>(&mut self, store: &mut AgentViewStore, line: S) {
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.push_line(line);
        }
    }

    /// RPC-020: clear the currently focused session's scrollback +
    /// input + popups. No-op when no session is open.
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

    /// Record a chunk into the currently focused session's scrollback.
    pub fn record_chunk(
        &mut self,
        store: &mut AgentViewStore,
        chunk: &codelet_rpc_types::StreamChunk,
    ) {
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.record_chunk(chunk);
        }
    }

    /// RPC-020: forward file-search results into the open popup.
    pub fn set_file_search_results(&mut self, matches: Vec<String>) {
        if let Some(p) = self.file_popup.as_mut() {
            p.set_matches(matches);
        }
    }

    /// RPC-029 layout. RPC-026: when `resume_view` or `search_view`
    /// is `Some`, EARLY-RETURN after painting the mode view into the
    /// entire `area`. Otherwise paints the normal Header / RoleBanner
    /// / Scrollback / Footer / Input layout + the slash / file popup
    /// overlays.
    pub fn render_with_store(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        store: &mut AgentViewStore,
    ) {
        self.last_render_area = Some(area);
        if let Some(v) = self.resume_view.as_ref() {
            v.render(area, buf);
            return;
        }
        if let Some(v) = self.search_view.as_ref() {
            v.render(area, buf);
            return;
        }
        // RPC-029: input has no border; height == visible_rows.
        let input_height = self.input.visible_rows();

        let sid = store.current_session().cloned();
        let role_height: u16 = sid
            .as_ref()
            .and_then(|s| store.role_for(s))
            .map(|_| 1)
            .unwrap_or(0);
        // RPC-029 layout order: Header(1), RoleBanner(0|1),
        // Scrollback(flex), Footer(1), Input(visible_rows).
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
        let (header_area, role_area, scrollback_area, footer_area, input_area) =
            (split[0], split[1], split[2], split[3], split[4]);
        self.last_input_area = Some(input_area);

        let model = sid.as_ref().and_then(|s| store.model_info_for(s));
        let thinking = sid
            .as_ref()
            .and_then(|s| store.thinking_level_for(s).copied())
            .unwrap_or(codelet_rpc_types::ThinkingLevel::Off);
        let tokens = sid
            .as_ref()
            .and_then(|s| store.token_state_for(s).copied())
            .unwrap_or_default();
        // RPC-029: wire work-unit context from the store. Other badge
        // flags (is_isolated/is_debug/is_select_mode/...) default to
        // their no-op values until follow-up cards thread them through.
        SessionHeader {
            session_index: store.session_index(),
            model,
            thinking,
            tokens,
            work_unit_id: store.current_work_unit_id(),
            work_unit_status: store.current_work_unit_status(),
            is_isolated: false,
            is_debug_enabled: false,
            is_select_mode: false,
            tokens_per_second: None,
            reasoning_tokens: 0,
            compaction_reduction: None,
            is_loading: false,
        }
        .render(header_area, buf);

        if role_height > 0 {
            if let Some(role_text) = sid.as_ref().and_then(|s| store.role_for(s)) {
                RoleBanner { role_text }.render(role_area, buf);
            }
        }

        // RPC-029: render scrollback directly into its slot — no
        // surrounding Block, no title.
        self.last_scrollback_viewport = scrollback_area.height;
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.scrollback.render_count_visited(scrollback_area, buf);
        }

        // RPC-029: footer now sits ABOVE the input row.
        SessionFooter { workspace: store.workspace() }.render(footer_area, buf);

        // RPC-029: input has no border; paddingX=1 carved here.
        let pad = input_area.width.min(1);
        let padded = Rect {
            x: input_area.x + pad,
            y: input_area.y,
            width: input_area.width.saturating_sub(pad * 2),
            height: input_area.height,
        };
        self.input
            .render_with_prompt(padded, buf, INPUT_PLACEHOLDER_HINT);

        if let Some(p) = self.slash_popup.as_ref() {
            p.render(area, buf);
        } else if let Some(p) = self.file_popup.as_ref() {
            p.render(area, buf);
        }
    }
}
