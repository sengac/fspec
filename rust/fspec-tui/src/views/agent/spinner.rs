//! TUI-106 — re-export shim for the shared braille spinner.
//!
//! The pure spinner (frames, 80 ms cadence, DIM painter + its unit
//! tests) now lives at `crate::components::spinner` so the lazy
//! mode-views (TUI-107/108) can reuse it without importing another
//! view module. This shim keeps every existing `super::spinner::`
//! call site (`input_transition.rs`, `transition_driver.rs`) compiling
//! unchanged with byte-identical behavior.

//! Mirrors `src/tui/components/ThinkingIndicator.tsx:19-22` — see
//! `crate::components::spinner` for the full byte-for-byte contract
//! (10 frames `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`, 80 ms/frame, DIM-styled).

pub use crate::components::spinner::{
    current_frame_glyph, paint_spinner_line, DOTS_FRAMES, DOTS_INTERVAL_MS,
};
