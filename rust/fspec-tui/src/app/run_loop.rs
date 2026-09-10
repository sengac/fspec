//! `App::run` — the crossterm + action-bus + render-tick run loop
//! (RPC-008 rule [11]).
//!
//! Factored out of `app/events.rs` so both files stay under the
//! 300-LoC ceiling pinned by `source_shape_stores_rpc012`.

use anyhow::Result;
use crossterm::event::{Event, EventStream};
use futures::StreamExt;

use crate::terminal::TerminalGuard;
use crate::views::ViewMode;

use super::state::App;

/// Render-tick cadence — ~60fps cap per RPC-008 rule [11].
const RENDER_TICK: std::time::Duration = std::time::Duration::from_millis(16);

impl App {
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
            let session_status = self.current_session_status();
            guard.terminal().draw(|frame| {
                self.navigator.render_with_stores(
                    frame.area(),
                    frame.buffer_mut(),
                    &self.board_store,
                    &mut self.agent_view_store,
                );
                self.compositor.render(frame.area(), frame.buffer_mut());
                if let ViewMode::Agent = self.navigator.active_view {
                    if self.navigator.agent.is_cursor_visible(session_status) {
                        if let Some((x, y)) = self.navigator.agent.cursor_position() {
                            frame.set_cursor_position((x, y));
                        }
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
                    // COPY-006: drive long-press selection Begin from the tick.
                    self.navigator.agent.poll_selection_tick();
                    let is_busy = self.is_session_busy();
                    let is_animating = self.is_input_animating();
                    // TUI-106: a lazy mode-view cascade (Checkpoints /
                    // Changed Files) keeps the 16ms tick redrawing so the
                    // loading dialog's 80ms-cadence braille spinner
                    // animates on an otherwise-idle board.
                    let is_view_loading = self.is_view_loading();
                    // MUX-006: the mux focus flash keeps the 16ms tick
                    // redrawing during its 350ms window even when the
                    // session is idle.
                    let is_mux_flash_active = self.navigator.is_mux_flash_active();
                    if super::tick_should_draw(
                        self.should_render,
                        is_busy,
                        is_animating,
                        is_view_loading,
                        is_mux_flash_active,
                    ) {
                        let session_status = self.current_session_status();
                        guard.terminal().draw(|frame| {
                            self.navigator.render_with_stores(
                                frame.area(),
                                frame.buffer_mut(),
                                &self.board_store,
                                &mut self.agent_view_store,
                            );
                            self.compositor.render(frame.area(), frame.buffer_mut());
                            if let ViewMode::Agent = self.navigator.active_view {
                                if self.navigator.agent.is_cursor_visible(session_status) {
                                    if let Some((x, y)) = self.navigator.agent.cursor_position() {
                                        frame.set_cursor_position((x, y));
                                    }
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
