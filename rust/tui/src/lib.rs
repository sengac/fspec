//! TUI (Terminal User Interface) - part of CLI Interface bounded context
//!
//! The input queue the cli crate's interactive mode reads typed input from
//! while a turn streams.

pub mod input_queue;

pub use input_queue::InputQueue;
