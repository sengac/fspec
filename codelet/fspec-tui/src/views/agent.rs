//! AgentView — the always-on container. Owns presentation state for
//! the input + scrollback widgets + popup overlays; reads everything
//! else from [`AgentViewStore`].
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc013-agent-footer.feature
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc019-multiline-input.feature
//!   - spec/features/rpc019-scrollback.feature
//!   - spec/features/rpc020-slash-and-file-popups.feature
//!
//! 4-row vertical layout: Header(1) / Scrollback(flex) /
//! Input(visible_rows + 2 border) / Footer(1). RPC-020 popup overlays
//! (slash + file search) are painted on top of the base layout.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
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
pub mod scrollback;
pub mod slash_command_popup;
pub mod slash_commands;

pub use file_search_popup::{FilePopupOutcome, FileSearchPopup};
pub use footer::SessionFooter;
pub use header::SessionHeader;
pub use multiline_input::{InputEventOutcome, MultiLineInput};
pub use popups::{classify_buffer, splice_file_selection, PopupTrigger};
pub use scrollback::{ScrollState, ScrollbackList};
pub use slash_command_popup::{PopupOutcome, SlashCommandPopup};
pub use slash_commands::{SlashCommand, SlashCommandAction, SLASH_COMMANDS};

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
/// linkage live in `AgentViewStore`.
pub struct AgentView {
    pub scrollback: ScrollbackList,
    pub input: MultiLineInput,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub next_seq: u64,
    pub last_input_area: Option<Rect>,
    /// RPC-020: slash command palette (Some when active).
    pub slash_popup: Option<SlashCommandPopup>,
    /// RPC-020: `@file` search popup (Some when active).
    pub file_popup: Option<FileSearchPopup>,
}

impl Default for AgentView {
    fn default() -> Self {
        Self {
            scrollback: ScrollbackList::new(),
            input: MultiLineInput::new(),
            action_tx: None,
            next_seq: 0,
            last_input_area: None,
            slash_popup: None,
            file_popup: None,
        }
    }
}

impl AgentView {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            action_tx: Some(action_tx),
            ..Self::default()
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.scrollback.chunk_count()
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

    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        self.scrollback.push(RenderedChunk {
            seq,
            lines: vec![Line::from(Span::raw(line.into()))],
        });
    }

    /// RPC-020: clear scrollback + input + popups (used by /clear).
    pub fn reset_scrollback(&mut self) {
        self.scrollback.reset();
        self.next_seq = 0;
        self.input.reset();
        self.slash_popup = None;
        self.file_popup = None;
    }

    pub(crate) fn emit(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    fn chunk_to_lines(chunk: &codelet_rpc_types::StreamChunk) -> Vec<Line<'static>> {
        use codelet_rpc_types::StreamChunk as SC;
        let body: String = match chunk {
            SC::Text { text, .. } => format!("assistant> {text}"),
            SC::Thinking { thinking, .. } => format!("(thinking) {thinking}"),
            SC::UserNotification { message, .. } => format!("[notice] {message}"),
            SC::Error { error } => format!("[error] {error}"),
            SC::Done => "[done]".to_string(),
            other => format!("{other:?}"),
        };
        vec![Line::from(Span::raw(body))]
    }

    pub fn record_chunk(&mut self, chunk: &codelet_rpc_types::StreamChunk) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        let lines = Self::chunk_to_lines(chunk);
        self.scrollback.push(RenderedChunk { seq, lines });
    }

    /// RPC-020: forward file-search results into the open popup.
    pub fn set_file_search_results(&mut self, matches: Vec<String>) {
        if let Some(p) = self.file_popup.as_mut() {
            p.set_matches(matches);
        }
    }

    /// RPC-019 layout. RPC-020 adds popup overlay paint after the base
    /// widgets so the slash + file search popups float above the input.
    pub fn render_with_store(&mut self, area: Rect, buf: &mut Buffer, store: &AgentViewStore) {
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

        let sid = store.current_session();
        let model = sid.and_then(|s| store.model_info_for(s));
        let thinking = sid
            .and_then(|s| store.thinking_level_for(s).copied())
            .unwrap_or(codelet_rpc_types::ThinkingLevel::Off);
        let tokens = sid
            .and_then(|s| store.token_state_for(s).copied())
            .unwrap_or_default();
        SessionHeader { session_index: store.session_index(), model, thinking, tokens }
            .render(header_area, buf);

        let title = match sid {
            Some(s) => format!(" Agent — {} ", s.value),
            None => " Agent ".to_string(),
        };
        let scrollback_block = Block::default().borders(Borders::ALL).title(title);
        let inner_scrollback = scrollback_block.inner(scrollback_area);
        scrollback_block.render(scrollback_area, buf);
        self.scrollback.render_count_visited(inner_scrollback, buf);

        let input_block = Block::default().borders(Borders::ALL);
        let inner_input = input_block.inner(input_area);
        input_block.render(input_area, buf);
        self.input
            .render_with_prompt(inner_input, buf, INPUT_PLACEHOLDER_HINT);

        SessionFooter { workspace: store.workspace() }.render(footer_area, buf);

        // RPC-020 overlay paint — slash popup wins over file popup if
        // both are somehow live (the sync logic prevents this).
        if let Some(p) = self.slash_popup.as_ref() {
            p.render(area, buf);
        } else if let Some(p) = self.file_popup.as_ref() {
            p.render(area, buf);
        }
    }
}
