//! TUI (Terminal User Interface) - part of CLI Interface bounded context
//!
//! Infrastructure module for terminal rendering and input handling.
//! Used by the cli crate's interactive mode.
//! Based on OpenAI codex architecture: inline viewport with preserved scrollback.

pub mod events;
pub mod input_queue;
pub mod status;
pub mod terminal;

pub use events::{create_event_stream, TuiEvent};
pub use input_queue::InputQueue;
pub use status::StatusDisplay;
pub use terminal::{restore_terminal, set_panic_hook};
