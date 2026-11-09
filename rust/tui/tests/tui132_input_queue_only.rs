//! TUI-132 (ST-4): the shared terminal crate keeps only what has a caller.
//!
//! Feature: spec/features/every-tui-action-is-reachable-from-the-ui.feature
//!
//! The cli crate's interactive mode uses only `InputQueue`; the status
//! line, event stream and terminal restore helpers had no caller in the
//! workspace.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

fn src(file: &str) -> Option<String> {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file)).ok()
}

/// Scenario: The shared terminal crate exports only the input queue
#[test]
fn the_shared_terminal_crate_exports_only_the_input_queue() {
    // @step Given the codelet-tui crate
    let lib = src("lib.rs").expect("read src/lib.rs");

    // @step Then its library exports InputQueue
    assert!(lib.contains("pub use input_queue::InputQueue;"));
    let _queue = codelet_tui::InputQueue::new();

    // @step And it declares no StatusDisplay, create_event_stream, TuiEvent, restore_terminal or set_panic_hook
    for gone in [
        "StatusDisplay",
        "create_event_stream",
        "TuiEvent",
        "restore_terminal",
        "set_panic_hook",
    ] {
        assert!(!lib.contains(gone), "lib.rs still exports {gone}");
    }
    for file in ["status.rs", "events.rs", "terminal.rs"] {
        assert!(src(file).is_none(), "src/{file} still exists");
    }
}
