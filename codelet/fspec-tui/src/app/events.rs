//! `App::handle_event` + `App::handle_paste` + `App::render` + `App::run`
//! (RPC-008 rule [11], RPC-009 architecture note [9], RPC-012 navigator
//! routing).
//!
//! The crossterm event flow is:
//!   DisconnectDialog (Critical) → app-shortcuts (`?` / `q` / Ctrl+D)
//!     → Compositor → Navigator → store mutation via [`super::dispatch`].

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures::StreamExt;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::disconnect_dialog::DISCONNECT_DIALOG_ID;
use crate::components::help_dialog::HelpDialog;
use crate::components::{Action, EventResult, Priority};
use crate::terminal::TerminalGuard;
use crate::views::ViewMode;

use super::state::App;

/// Render-tick cadence — ~60fps cap per RPC-008 rule [11].
const RENDER_TICK: Duration = Duration::from_millis(16);

impl App {
    /// Process a single crossterm event. The DisconnectDialog (when
    /// topmost) intercepts everything; app-level shortcuts (`?`, `q`,
    /// Ctrl+D) fire next; then the Compositor; then the Navigator.
    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        let topmost_is_critical = matches!(
            self.compositor.topmost_priority(),
            Some(Priority::Critical)
        );
        let topmost_is_disconnect =
            self.compositor.topmost_id().as_deref() == Some(DISCONNECT_DIALOG_ID);

        if topmost_is_disconnect {
            return self.handle_disconnect_dialog_event(event);
        }

        if !topmost_is_critical {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Release {
                    if let Some(result) = self.handle_app_shortcut(key) {
                        return result;
                    }
                }
            }
        }

        let result = self.compositor.handle_event(event);
        if let EventResult::Consumed(Some(callback)) = result {
            callback(&mut self.compositor);
            self.should_render = true;
            return EventResult::consumed();
        }
        if result.is_consumed() {
            self.should_render = true;
            return result;
        }
        let nav_result = self.navigator.handle_event(event, &self.board_store);
        if nav_result.is_consumed() {
            self.should_render = true;
        }
        nav_result
    }

    fn handle_disconnect_dialog_event(&mut self, event: &Event) -> EventResult {
        // RPC-011 CR-1 rule [2]: DisconnectDialog is topmost and honours
        // ONLY `q` (quit) and `r` (manual reconnect).
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Release {
                if key.code == KeyCode::Char('q')
                    && key.modifiers == KeyModifiers::NONE
                {
                    self.should_quit = true;
                    let _ = self.compositor.remove(DISCONNECT_DIALOG_ID);
                    return EventResult::consumed();
                }
                if key.code == KeyCode::Char('r')
                    && key.modifiers == KeyModifiers::NONE
                {
                    let _ = self.action_tx.send(Action::ManualReconnect);
                    return EventResult::consumed();
                }
            }
        }
        EventResult::consumed()
    }

    fn handle_app_shortcut(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if key.code == KeyCode::Char('?') && key.modifiers == KeyModifiers::NONE {
            self.compositor.push(Box::new(HelpDialog::new()));
            self.should_render = true;
            return Some(EventResult::consumed());
        }
        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            self.should_quit = true;
            return Some(EventResult::consumed());
        }
        if key.code == KeyCode::Char('d')
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.should_quit = true;
            return Some(EventResult::consumed());
        }
        None
    }

    /// Forward a paste payload to the Compositor's stub paste handler.
    pub fn handle_paste(&mut self, text: &str) -> EventResult {
        let result = self.compositor.handle_paste(text);
        if let EventResult::Consumed(Some(callback)) = result {
            callback(&mut self.compositor);
            self.should_render = true;
            return EventResult::consumed();
        }
        self.should_render = true;
        result
    }

    /// Paint the Navigator (Board or Agent + footer) first, then the
    /// Compositor's modal stack on top.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.navigator
            .render_with_stores(area, buf, &self.board_store, &mut self.agent_view_store);
        self.compositor.render(area, buf);
        self.should_render = false;
    }

    /// Drive the run loop. Per RPC-008 rule [11]: `tokio::select!` over
    /// the crossterm `EventStream`, the action_rx channel, and a 16ms
    /// render-tick interval (~60fps cap).
    pub async fn run(mut self) -> Result<()> {
        let mut guard = TerminalGuard::init()?;
        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(RENDER_TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Initial draw.
        if self.should_render {
            guard.terminal().draw(|frame| {
                self.navigator.render_with_stores(
                    frame.area(),
                    frame.buffer_mut(),
                    &self.board_store,
                    &mut self.agent_view_store,
                );
                self.compositor.render(frame.area(), frame.buffer_mut());
                if let ViewMode::Agent = self.navigator.active_view {
                    if let Some((x, y)) = self.navigator.agent.cursor_position() {
                        frame.set_cursor_position((x, y));
                    }
                }
            })?;
            self.should_render = false;
        }

        while !self.should_quit {
            tokio::select! {
                Some(event) = events.next() => {
                    let event = event?;
                    match event {
                        Event::Paste(text) => {
                            let _ = self.handle_paste(&text);
                        }
                        Event::Resize(_, _) => {
                            self.should_render = true;
                        }
                        other => {
                            let _ = self.handle_event(&other);
                        }
                    }
                }
                Some(action) = self.action_rx.recv() => {
                    self.dispatch(action);
                }
                _ = tick.tick() => {
                    if self.should_render {
                        guard.terminal().draw(|frame| {
                            self.navigator.render_with_stores(
                                frame.area(),
                                frame.buffer_mut(),
                                &self.board_store,
                                &mut self.agent_view_store,
                            );
                            self.compositor.render(frame.area(), frame.buffer_mut());
                            if let ViewMode::Agent = self.navigator.active_view {
                                if let Some((x, y)) = self.navigator.agent.cursor_position() {
                                    frame.set_cursor_position((x, y));
                                }
                            }
                        })?;
                        self.should_render = false;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Helper: synthesise a Key Press event with no modifiers.
pub fn synth_key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
