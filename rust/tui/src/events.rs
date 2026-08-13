//! Event stream coordination
//!
//! Unified event stream combining terminal events and draw requests.
//! Based on codex event stream pattern with tokio::select!.

use crossterm::event::{Event, KeyEvent};
use futures::Stream;
use std::pin::Pin;
use tokio_stream::StreamExt;

/// TUI events (terminal + draw requests)
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// Keyboard input
    Key(KeyEvent),
    /// Terminal paste event
    Paste(String),
    /// Redraw request
    Draw,
    /// Resize event
    Resize,
}

/// Create unified event stream from crossterm events
pub fn create_event_stream() -> Pin<Box<dyn Stream<Item = TuiEvent> + Send>> {
    let mut crossterm_events = crossterm::event::EventStream::new();

    let event_stream = async_stream::stream! {
        loop {
            if let Some(Ok(event)) = crossterm_events.next().await {
                match event {
                    Event::Key(key_event) => {
                        yield TuiEvent::Key(key_event);
                    }
                    Event::Paste(pasted) => {
                        yield TuiEvent::Paste(pasted);
                    }
                    Event::Resize(_, _) => {
                        yield TuiEvent::Resize;
                    }
                    _ => {}
                }
            }
        }
    };

    Box::pin(event_stream)
}
