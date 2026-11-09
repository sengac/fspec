use super::{extract_brace_block_after, read_workspace_source};

pub fn engine_compaction_arm() -> String {
    let exports = read_workspace_source("rust/agent-loop/src/lib.rs");
    assert!(exports.contains("pub use agent_loop::agent_loop;"));
    assert!(exports
        .contains("pub use background_output::{BackgroundOutput, BackgroundProgressEmitter};"));
    let engine = read_workspace_source("rust/agent-loop/src/agent_loop.rs");
    assert!(
        engine.contains("use crate::background_output::{BackgroundOutput, LoopDetectionWiring};")
    );
    assert!(engine.contains("let output = BackgroundOutput::with_provider("));

    let shared = read_workspace_source("rust/agent-loop/src/background_output.rs");
    extract_brace_block_after(&shared, "StreamEvent::CompactionStarted =>").to_owned()
}
