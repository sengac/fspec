//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

#[test]
pub(super) fn scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback(
) {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    // @step When I inspect the `handle_output` method in the moved file
    let sig_marker = "pub fn handle_output(&self, chunk: StreamChunk)";
    assert!(moved.contains(sig_marker));
    let body_start = moved
        .find(sig_marker)
        .expect("handle_output signature must exist");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    // @step Then it still pushes the chunk into output_buffer
    assert!(body.contains("output_buffer") && body.contains(".push("));
    // @step And it still calls `supervisor_broadcast.send(chunk.clone())`
    assert!(body.contains("supervisor_broadcast.send(chunk.clone())"));
    // @step And it unconditionally calls the new chunks_tx broadcast (`let _ = self.chunks_tx.send((<session-id>, chunk.clone()));`) — RPC-041 removed the `if let Some(tx)` wrapper
    assert!(body.contains("self.chunks_tx"));
    assert!(body.contains("self.chunks_tx.send("));
    assert!(moved.contains("chunks_tx:"));
    // @step And there is zero reference to GLOBAL_CHUNK_CALLBACK in the moved file
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in moved.lines().enumerate() {
        let lineno = idx + 1;
        let code = match line.find("//") {
            Some(pos) => &line[..pos],
            None => line,
        };
        if code.contains("GLOBAL_CHUNK_CALLBACK") {
            violations.push(format!("{lineno}: {line}"));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
    // @step And the GLOBAL_CHUNK_CALLBACK global has been deleted from rust/napi/src/session_manager.rs (RPC-041 inverted the prior RPC-039 assertion)
    let manager = codelet_test_helpers::source_composition::read_tree_or_file(
        workspace_root()
            .join("sessions")
            .join("src")
            .join("session_manager.rs"),
    )
    .expect("session_manager source tree must be readable");
    let manager_code = strip_line_comments(&manager);
    assert!(!manager_code.contains("GLOBAL_CHUNK_CALLBACK"));
    assert!(!manager_code.contains("struct GlobalChunkCallback {"));
    assert!(
        !manager_code.contains("unsafe impl Send for GlobalChunkCallback")
            && !manager_code.contains("unsafe impl Sync for GlobalChunkCallback")
    );
}
