//! Shared mode detection for dual-front-door prompts (CLI vs native harness).
//!
//! `FSPEC_CAPTURE_MODE=1` is set by the harness entry points (combined/daemon/client)
//! when this process hosts native agent sessions with the AstGrep rig tool.
//! One-shot CLI subcommands never set it.

/// Returns `true` when running inside the native harness (TUI/daemon) agent loop.
pub fn in_capture_mode() -> bool {
    std::env::var("FSPEC_CAPTURE_MODE").is_ok_and(|v| v == "1")
}
