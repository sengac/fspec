//! Feature: spec/features/replace-global-chunk-callback-background-session.feature
//!
//! Moved out of broadcast.rs (COV-075: one test file per feature).

use super::*;

#[test]
fn scenario_handle_output_emits_unconditionally_on_chunks_tx() {
    // @step Given the BackgroundSession code lives at rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    // @step And `BackgroundSession::handle_output(&self, chunk: StreamChunk)` is defined there
    let sig_marker = "pub fn handle_output(&self, chunk: StreamChunk)";
    assert!(moved.contains(sig_marker));
    // @step When I inspect the body of handle_output in the moved file
    let body_start = moved.find(sig_marker).expect("handle_output sig");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);
    // @step Then the body still pushes the chunk into output_buffer
    assert!(body_code.contains("output_buffer") && body_code.contains(".push("));
    // @step And the body still calls `supervisor_broadcast.send(chunk.clone())`
    assert!(body_code.contains("supervisor_broadcast.send(chunk.clone())"));
    // @step And the body unconditionally calls `self.chunks_tx.send((codelet_rpc_types::SessionId::from(self.id.to_string()), chunk.clone()))` without an `if let Some(tx)` wrapper
    assert!(body_code.contains("self.chunks_tx.send("));
    assert!(!body_code.contains("if let Some(tx) = &self.chunks_tx"));
    // @step And the BackgroundSession struct declares `chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>` as a non-Option, non-pub field
    let struct_start = moved
        .find("pub struct BackgroundSession {")
        .expect("BackgroundSession struct decl must exist");
    let struct_body = &moved[struct_start..];
    let struct_end = struct_body.find("\n}\n").expect("struct must terminate");
    let fields_block = &struct_body[..struct_end];
    let fields_code = strip_line_comments(fields_block);
    assert!(
        fields_code.contains("chunks_tx: broadcast::Sender<")
            || fields_code.contains("chunks_tx: tokio::sync::broadcast::Sender<")
    );
    assert!(!fields_code.contains("chunks_tx: Option<"));
}

#[test]
fn scenario_set_status_emits_on_status_changes_tx() {
    // @step Given the BackgroundSession code lives at rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    // @step And `BackgroundSession::set_status(&self, status: SessionStatus)` is defined there
    let sig_marker = "pub fn set_status(&self, status: SessionStatus)";
    assert!(moved.contains(sig_marker));
    // @step When I inspect the body of set_status in the moved file
    let body_start = moved.find(sig_marker).expect("set_status sig");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);
    // @step Then the body first calls `let old_status = self.status.swap(status as u8, Ordering::AcqRel)`
    assert!(body_code.contains("self.status.swap(status as u8, Ordering::AcqRel)"));
    // @step And under the `if old_status != status as u8` guard the body calls `let _ = self.status_changes_tx.send((codelet_rpc_types::SessionId::from(self.id.to_string()), status))`
    assert!(body_code.contains("if old_status != status as u8"));
    assert!(body_code.contains("self.status_changes_tx.send("));
    // @step And the body still calls `self.handle_output(StreamChunk::session_state_change(state))`
    assert!(body_code.contains("self.handle_output(StreamChunk::session_state_change("));
    // @step And the body still calls `codelet_tools::broadcast_metadata_update()`
    assert!(body_code.contains("codelet_tools::broadcast_metadata_update()"));
    // @step And the BackgroundSession struct declares a non-Option `status_changes_tx: broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::SessionStatus)>` field
    let struct_start = moved
        .find("pub struct BackgroundSession {")
        .expect("BackgroundSession struct decl must exist");
    let struct_body = &moved[struct_start..];
    let struct_end = struct_body.find("\n}\n").expect("struct must terminate");
    let fields_block = &struct_body[..struct_end];
    let fields_code = strip_line_comments(fields_block);
    assert!(
        fields_code.contains("status_changes_tx: broadcast::Sender<")
            || fields_code.contains("status_changes_tx: tokio::sync::broadcast::Sender<")
    );
    assert!(!fields_code.contains("status_changes_tx: Option<"));
}
