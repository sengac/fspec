use super::{extract_brace_block_after, read_workspace_source};

pub fn napi_compaction_arm() -> String {
    let compatibility = read_workspace_source("rust/napi/src/agent_loop.rs");
    assert!(compatibility.contains("pub use codelet_agent_loop::{"));
    assert!(compatibility
        .contains("agent_loop, agent_loop_dispatch_supports_provider, BackgroundOutput"));
    assert!(!compatibility.contains("StreamEvent::CompactionStarted =>"));
    assert!(!compatibility.contains("pre_compaction_tokens"));

    let hooks = read_workspace_source("rust/napi/src/session_hooks.rs");
    assert!(hooks.contains("fn spawn_agent_loop("));
    let spawn = extract_brace_block_after(
        &hooks,
        "mcp_injection_rx: tokio::sync::mpsc::Receiver<codelet_tools::McpInjection>,\n    )",
    );
    assert!(
        spawn.contains("codelet_agent_loop::agent_loop(session, input_rx, mcp_injection_rx).await")
    );

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
