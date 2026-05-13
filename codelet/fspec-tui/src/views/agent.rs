//! AgentView — slim port of the previous `AgentReplView`.
//!
//! Feature: spec/features/rpc012-board-agent-navigation.feature
//! Card: RPC-012 (replaces RPC-009 `AgentReplView`).
//!
//! Reads `current_session` from `AgentViewStore` (passed in via
//! `render_with_store`) rather than owning an `active_session` field.
//! Preserves the RPC-009 single-line `tui_input::Input` + `Vec<RenderedChunk>`
//! scrollback shape — multi-line input, slash commands, model picker,
//! file search, history navigation, isolation banner, etc. are all
//! deferred to downstream slices.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use tokio::sync::mpsc::UnboundedSender;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::components::{Action, EventResult};
use crate::store::AgentViewStore;

/// Pre-rendered chunk row keyed by chunk seq. Migrated from
/// `views/agent_repl.rs` unchanged.
#[derive(Debug, Clone)]
pub struct RenderedChunk {
    pub seq: u64,
    pub lines: Vec<Line<'static>>,
}

/// AgentView — single-session input + scrollback. Owns ONLY presentation
/// state (the input buffer, scrollback Vec, scroll offsets); the session
/// id and work-unit linkage are owned by `AgentViewStore`.
pub struct AgentView {
    pub scrollback: Vec<RenderedChunk>,
    pub input: Input,
    pub scroll_offset: u16,
    pub stick_to_bottom: bool,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub next_seq: u64,
    pub last_input_area: Option<Rect>,
}

impl Default for AgentView {
    fn default() -> Self {
        Self {
            scrollback: Vec::new(),
            input: Input::default(),
            scroll_offset: 0,
            stick_to_bottom: true,
            action_tx: None,
            next_seq: 0,
            last_input_area: None,
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
        self.scrollback.len()
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        let area = self.last_input_area?;
        let x = area
            .x
            .saturating_add(1)
            .saturating_add(self.input.visual_cursor() as u16);
        let y = area.y.saturating_add(1);
        Some((x, y))
    }

    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        self.scrollback.push(RenderedChunk {
            seq,
            lines: vec![Line::from(Span::raw(line.into()))],
        });
        if self.stick_to_bottom {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }

    fn emit(&self, action: Action) {
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

    /// Record a chunk in the scrollback. Called by `App::dispatch` on
    /// `Action::ChunkReceived` after the chunks subscriber has already
    /// filtered by `AgentViewStore.current_session`.
    pub fn record_chunk(&mut self, chunk: &codelet_rpc_types::StreamChunk) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        let lines = Self::chunk_to_lines(chunk);
        self.scrollback.push(RenderedChunk { seq, lines });
        if self.stick_to_bottom {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }

    /// Handle a keyboard event. ESC emits `Action::BackToBoard` so the
    /// Navigator can flip back to BoardView. Everything else mirrors
    /// the RPC-009 AgentReplView behaviour.
    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };

        // ESC → back to BoardView.
        if key.code == KeyCode::Esc {
            self.emit(Action::BackToBoard);
            return EventResult::consumed();
        }

        if key.code == KeyCode::Enter {
            let value = self.input.value().to_string();
            if value.is_empty() {
                return EventResult::ignored();
            }
            self.emit(Action::InputSubmitted(value));
            self.input.reset();
            return EventResult::consumed();
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.emit(Action::Interrupt);
            return EventResult::consumed();
        }
        if key.code == KeyCode::PageUp {
            self.stick_to_bottom = false;
            self.scroll_offset = self.scroll_offset.saturating_sub(5);
            return EventResult::consumed();
        }
        if key.code == KeyCode::PageDown || key.code == KeyCode::End {
            let total_lines: u16 = self
                .scrollback
                .iter()
                .map(|rc| rc.lines.len() as u16)
                .sum();
            self.scroll_offset = self.scroll_offset.saturating_add(5);
            if self.scroll_offset >= total_lines.saturating_sub(1) {
                self.stick_to_bottom = true;
            }
            return EventResult::consumed();
        }
        let _ = self.input.handle_event(event);
        EventResult::consumed()
    }

    /// Render the view against the supplied store snapshot. The store
    /// is &-borrow only — mutation flows through `App::dispatch`.
    pub fn render_with_store(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        store: &AgentViewStore,
    ) {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);
        let scrollback_area = split[0];
        let input_area = split[1];
        self.last_input_area = Some(input_area);

        let title = match store.current_session() {
            Some(sid) => format!(" Agent — {} ", sid.value),
            None => " Agent ".to_string(),
        };
        let scrollback_block = Block::default().borders(Borders::ALL).title(title);
        let inner_scrollback = scrollback_block.inner(scrollback_area);
        scrollback_block.render(scrollback_area, buf);

        let mut all_lines: Vec<Line<'static>> = Vec::new();
        for rc in &self.scrollback {
            for l in &rc.lines {
                all_lines.push(l.clone());
            }
        }
        let total_lines = all_lines.len() as u16;
        let scrollback_height = inner_scrollback.height;
        let y_offset = if self.stick_to_bottom {
            total_lines.saturating_sub(scrollback_height)
        } else {
            self.scroll_offset
                .min(total_lines.saturating_sub(scrollback_height))
        };
        let scrollback = Paragraph::new(Text::from(all_lines))
            .scroll((y_offset, 0))
            .wrap(Wrap { trim: false });
        scrollback.render(inner_scrollback, buf);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Input (Enter=send, Ctrl+C=interrupt, ESC=back)")
            .border_style(Style::default().bold());
        let input_widget = Paragraph::new(self.input.value().to_string())
            .block(input_block)
            .style(Style::default().bold());
        input_widget.render(input_area, buf);
    }

}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::store::AgentViewStore;
    use codelet_rpc_types::SessionId;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use tokio::sync::mpsc::unbounded_channel;

    fn fresh() -> (AgentView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = unbounded_channel();
        (AgentView::new(tx), rx)
    }

    #[test]
    fn esc_emits_back_to_board() {
        let (mut view, mut rx) = fresh();
        let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let result = view.handle_event(&event);
        assert!(matches!(result, EventResult::Consumed(None)));
        let action = rx.try_recv().expect("Action::BackToBoard on bus");
        assert!(matches!(action, Action::BackToBoard));
    }

    #[test]
    fn enter_on_non_empty_input_emits_input_submitted() {
        let (mut view, mut rx) = fresh();
        view.input = view.input.clone().with_value("hi".to_string());
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let _ = view.handle_event(&event);
        let action = rx.try_recv().expect("Action::InputSubmitted on bus");
        match action {
            Action::InputSubmitted(s) => assert_eq!(s, "hi"),
            other => panic!("expected InputSubmitted, got {other:?}"),
        }
        assert_eq!(view.input.value(), "");
    }

    #[test]
    fn render_with_store_paints_agent_title_with_session_id() {
        let (mut view, _rx) = fresh();
        let mut store = AgentViewStore::default();
        store.set_current_session(Some(SessionId::new("s-1")));
        let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
        term.draw(|frame| {
            view.render_with_store(frame.area(), frame.buffer_mut(), &store);
        })
        .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                joined.push_str(buf[(x, y)].symbol());
            }
            joined.push('\n');
        }
        assert!(joined.contains("Agent"));
        assert!(joined.contains("s-1"));
    }
}
