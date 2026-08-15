//! CLI Interface bounded context
//!
//! Shared streaming engine, session management, and compaction helpers.
//! Consumed by `codelet-agent-loop`, `codelet-sessions`, `codelet-napi`,
//! and `codelet-fspec-tui`.

pub mod compaction_dag; // DAG instructions, detection, extraction, force-injection (CMPCT-016)
pub mod compaction_threshold; // CLI-020: Autocompact buffer for compaction threshold
pub mod context; // Context management - token tracking
pub mod error_display; // Error message formatting for user display
pub mod interactive;
pub mod interactive_helpers; // Compaction helpers for interactive mode (CLI-010)
pub mod large_write_intent; // CLI-019: Large write intent detection and chunking guidance
pub mod session; // Session management
pub mod terminal; // Terminal display utilities (ANSI codes, etc.)
