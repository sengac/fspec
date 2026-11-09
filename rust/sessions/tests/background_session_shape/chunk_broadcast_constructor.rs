//! Feature: spec/features/replace-global-chunk-callback-background-session.feature
//!
//! Moved out of api.rs (COV-075: one test file per feature).

use super::*;

#[test]
fn scenario_background_session_new_accepts_broadcast_senders_as_trailing_parameters() {
    // @step Given the BackgroundSession constructor `BackgroundSession::new(...)` is the sole construction site for BackgroundSession
    let moved = read(&moved_file_path());
    // @step When I inspect the parameter list of `BackgroundSession::new` in rust/sessions/src/background_session.rs
    let impl_marker = moved
        .find("impl BackgroundSession {")
        .expect("`impl BackgroundSession {` must exist");
    let impl_body = &moved[impl_marker..];
    let new_offset_in_impl = impl_body
        .find("pub fn new(")
        .expect("BackgroundSession::new must exist inside its impl block");
    let new_sig_start = impl_marker + new_offset_in_impl;
    let after = &moved[new_sig_start..];
    let body_open = after
        .find(") -> Self")
        .or_else(|| after.find(") -> BackgroundSession"))
        .expect("BackgroundSession::new return marker must exist");
    let params = &after[..body_open];
    let params_code = strip_line_comments(params);
    // @step Then the signature ends with two new trailing parameters `chunks_tx: tokio::sync::broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)>` and `status_changes_tx: tokio::sync::broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::SessionStatus)>`
    assert!(params_code.contains("chunks_tx:") && params_code.contains("broadcast::Sender"));
    assert!(params_code.contains("status_changes_tx:"));
    // @step And the body initializes the `chunks_tx` struct field directly from the parameter (no `None` placeholder remains)
    let body_start = body_open;
    let after_body_start = &after[body_start..];
    let body_end = after_body_start
        .find("\n    }\n")
        .unwrap_or(after_body_start.len());
    let new_body = &after_body_start[..body_end];
    let new_body_code = strip_line_comments(new_body);
    assert!(!new_body_code.contains("chunks_tx: None"));
    assert!(new_body_code.contains("chunks_tx,") || new_body_code.contains("chunks_tx:"));
    // @step And the body initializes the `status_changes_tx` struct field directly from the parameter
    assert!(
        new_body_code.contains("status_changes_tx,")
            || new_body_code.contains("status_changes_tx:")
    );
}
