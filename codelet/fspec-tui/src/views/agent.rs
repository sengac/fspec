//! AgentView — the always-on container. Owns presentation state for
//! the input + popup overlays; reads scrollback + token chrome from
//! the per-session [`crate::store::SessionContext`] via
//! [`AgentViewStore`].
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc013-agent-footer.feature
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc019-multiline-input.feature
//!   - spec/features/rpc019-scrollback.feature
//!   - spec/features/rpc020-slash-and-file-popups.feature
//!   - spec/features/rpc024-multi-session-cycling.feature
//!
//! 4-row vertical layout: Header(1) / Scrollback(flex) /
//! Input(visible_rows + 2 border) / Footer(1). RPC-020 popup overlays
//! (slash + file search) are painted on top of the base layout.
//! RPC-024 moved scrollback ownership onto SessionContext — AgentView
//! no longer owns a `scrollback` field; the render path borrows the
//! per-session ScrollbackList from `AgentViewStore`.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Widget};
use tokio::sync::mpsc::UnboundedSender;

use crate::components::Action;
use crate::store::AgentViewStore;

pub mod dispatch;
pub mod file_search_popup;
pub mod footer;
pub mod header;
pub mod multiline_input;
pub mod popup_body;
pub mod popups;
pub mod resume_picker;
pub mod scrollback;
pub mod search_palette;
pub mod slash_command_popup;
pub mod slash_commands;

pub use file_search_popup::{FilePopupOutcome, FileSearchPopup};
pub use footer::SessionFooter;
pub use header::SessionHeader;
pub use multiline_input::{InputEventOutcome, MultiLineInput};
pub use popups::{classify_buffer, splice_file_selection, PopupTrigger};
pub use resume_picker::{ResumePicker, ResumePickerOutcome};
pub use scrollback::{ScrollState, ScrollbackList};
pub use search_palette::{SearchPalette, SearchPaletteOutcome};
pub use slash_command_popup::{PopupOutcome, SlashCommandPopup};
pub use slash_commands::{SlashCommand, SlashCommandAction, SLASH_COMMANDS};

use ratatui::text::Line;

/// RPC-013 placeholder footer hints — kept here so the RPC-013
/// source-shape invariant continues to hold.
pub const PLACEHOLDER_FOOTER_HINTS: &str = "Enter=send  Ctrl+C=interrupt  ESC=back";

/// RPC-019 placeholder hint painted inside the input box when empty.
pub const INPUT_PLACEHOLDER_HINT: &str =
    "Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)";

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
    /// RPC-026: /resume session picker (Some when active).
    pub resume_popup: Option<ResumePicker>,
    /// RPC-026: /search history palette (Some when active).
    pub search_popup: Option<SearchPalette>,
}

impl AgentView {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            action_tx: Some(action_tx),
            ..Self::default()
        }
    }

    /// Chunk count for the currently focused session, or 0 if no
    /// session is open. RPC-024: scrollback lives on SessionContext.
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
        let y = area.y.saturating_add(1).saturating_add(row as u16);
        Some((x, y))
    }

    /// Push a raw line into the currently focused session's scrollback.
    /// No-op when no session is open. RPC-024: replaces the
    /// pre-refactor field-mutating version that wrote into
    /// `AgentView.scrollback`.
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
    /// No-op when no session is open.
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

    /// RPC-019 layout. RPC-020 adds popup overlay paint after the base
    /// widgets so the slash + file search popups float above the input.
    /// RPC-024 reads the scrollback from the focused SessionContext.
    pub fn render_with_store(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        store: &mut AgentViewStore,
    ) {
        let input_height = self.input.visible_rows().saturating_add(2);
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(input_height),
                Constraint::Length(1),
            ])
            .split(area);
        let (header_area, scrollback_area, input_area, footer_area) =
            (split[0], split[1], split[2], split[3]);
        self.last_input_area = Some(input_area);

        let sid = store.current_session().cloned();
        let model = sid.as_ref().and_then(|s| store.model_info_for(s));
        let thinking = sid
            .as_ref()
            .and_then(|s| store.thinking_level_for(s).copied())
            .unwrap_or(codelet_rpc_types::ThinkingLevel::Off);
        let tokens = sid
            .as_ref()
            .and_then(|s| store.token_state_for(s).copied())
            .unwrap_or_default();
        SessionHeader { session_index: store.session_index(), model, thinking, tokens }
            .render(header_area, buf);

        let title = match &sid {
            Some(s) => format!(" Agent — {} ", s.value),
            None => " Agent ".to_string(),
        };
        let scrollback_block = Block::default().borders(Borders::ALL).title(title);
        let inner_scrollback = scrollback_block.inner(scrollback_area);
        scrollback_block.render(scrollback_area, buf);
        self.last_scrollback_viewport = inner_scrollback.height;
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.scrollback.render_count_visited(inner_scrollback, buf);
        }

        let input_block = Block::default().borders(Borders::ALL);
        let inner_input = input_block.inner(input_area);
        input_block.render(input_area, buf);
        self.input
            .render_with_prompt(inner_input, buf, INPUT_PLACEHOLDER_HINT);

        SessionFooter { workspace: store.workspace() }.render(footer_area, buf);

        // RPC-020/RPC-026 overlay paint — resume / search popups paint
        // on top of slash / file when present. The dispatch routing
        // enforces mutual exclusivity; this paint order is the
        // belt-and-braces backstop.
        if let Some(p) = self.slash_popup.as_ref() {
            p.render(area, buf);
        } else if let Some(p) = self.file_popup.as_ref() {
            p.render(area, buf);
        }
        if let Some(p) = self.resume_popup.as_ref() {
            p.render(area, buf);
        } else if let Some(p) = self.search_popup.as_ref() {
            p.render(area, buf);
        }
    }
}
