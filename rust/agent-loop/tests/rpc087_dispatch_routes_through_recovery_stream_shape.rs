//! Feature: spec/features/agent-loop-error-classification-recovery-wiring-shape.feature
//!
//! RPC-087 sibling regression-shape test on the agent-loop side: pin
//! that every per-turn provider dispatch in
//! `rust/agent-loop/src/dispatch.rs` funnels through
//! `codelet_cli::interactive::run_agent_stream_with_images` — the
//! single entry point that wires every error classifier + recovery
//! helper. Counting exactly one call site proves no provider arm has
//! diverged onto a non-recovery code path.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above rust/agent-loop")
        .to_path_buf()
}

// ====================================================================
// Scenario: dispatch.rs funnels every provider arm through the
// recovery-wired streaming engine
// ====================================================================
#[test]
fn dispatch_contains_exactly_one_run_agent_stream_with_images_call_site() {
    // @step Given the source file rust/agent-loop/src/dispatch.rs
    let path = workspace_root().join("rust/agent-loop/src/dispatch.rs");
    let body = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    // @step When I read the file as a string
    // (already read above)

    // @step Then it contains exactly one occurrence of the substring "codelet_cli::interactive::run_agent_stream_with_images("
    let needle = "codelet_cli::interactive::run_agent_stream_with_images(";
    let count = body.matches(needle).count();
    assert_eq!(
        count, 1,
        "dispatch.rs must contain exactly one call site for {needle} \
         (found {count}); every provider arm must funnel through the \
         recovery-wired streaming engine via run_with_provider!"
    );
}
