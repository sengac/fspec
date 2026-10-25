//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

#[test]
pub(super) fn scenario_send_input_is_rewritten_to_a_non_napi_result_type() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    // @step When I inspect the `send_input` method signature in the moved file
    let sig_marker = "pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>";
    // @step Then the signature is `pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>`
    assert!(moved.contains(sig_marker));
    // @step And the error construction site uses `format!(\"Failed to send input: {}\", e)` (not `napi::Error::from_reason(...)`)
    let body_start = moved
        .find(sig_marker)
        .expect("send_input signature must exist");
    let after_sig = &moved[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_sig.len());
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);
    assert!(
        body_code.contains("format!(\"Failed to send input: {}\", e)")
            || body_code.contains("format!(\"Failed to send input: {e}\")")
    );
    assert!(!body_code.contains("Error::from_reason"));
    assert!(!body_code.contains("napi::"));
    // @step And the napi-side free function session_send_input maps the new String error back to napi::Error::from_reason at the wire boundary so the TypeScript Promise<void> signature is preserved
    let napi = read(&napi_shell_path());
    let napi_sig_start = napi
        .find("pub fn session_send_input")
        .expect("session_send_input must exist in napi");
    let sig_span = &napi[napi_sig_start..];
    let sig_brace = sig_span.find('{').unwrap_or(sig_span.len());
    let sig_ws = sig_span[..sig_brace]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        sig_ws.contains("session_id: String")
            && sig_ws.contains("input: String")
            && sig_ws.contains("thinking_config: Option<String>")
            && sig_ws.contains("-> Result<()>")
    );
    let after_napi_sig = &napi[napi_sig_start..];
    let napi_fn_end = after_napi_sig[1..]
        .find("\npub fn ")
        .map(|i| i + 1)
        .unwrap_or(after_napi_sig.len().min(2000));
    let napi_body = &after_napi_sig[..napi_fn_end];
    assert!(
        napi_body.contains("Error::from_reason") || napi_body.contains("napi::Error::from_reason")
    );
}

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

#[test]
fn scenario_codelet_sessions_tests_assert_background_session_is_reachable_from_its_new_home() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step And the integration test rust/sessions/tests/background_session_shape.rs has been added
    let this_test_path = workspace_root()
        .join("sessions")
        .join("tests")
        .join("background_session_shape.rs");
    assert!(this_test_path.exists());
    // @step When I run `cargo test -p codelet-sessions --tests`
    // @step Then the existing smoke test `crate_compiles` passes with status ok
    let smoke_path = workspace_root()
        .join("sessions")
        .join("tests")
        .join("smoke.rs");
    let smoke = read(&smoke_path);
    assert!(smoke.contains("fn crate_compiles"));
    // @step And the new shape test asserts the path `codelet_sessions::background_session::BackgroundSession` resolves at compile-time
    // @step And the shape test asserts the supporting types (PromptInput, CompactionProgress, WorkUnitContext, IncomingMessage, BridgeImageData, SessionError) are publicly reachable from the same module
    fn _accept<T>(_: std::marker::PhantomData<T>) {}
    let _supporting_witness: fn() = || {
        _accept::<PromptInput>(std::marker::PhantomData);
        _accept::<CompactionProgress>(std::marker::PhantomData);
        _accept::<WorkUnitContext>(std::marker::PhantomData);
        _accept::<IncomingMessage>(std::marker::PhantomData);
        _accept::<BridgeImageData>(std::marker::PhantomData);
        _accept::<SessionError>(std::marker::PhantomData);
        let _ = format_incoming_message;
    };
    // @step And the shape test asserts BackgroundSession::send_input returns `Result<(), String>` (not napi::Result)
    let _send_input_witness: fn(&BackgroundSession, String, Option<String>) -> Result<(), String> =
        BackgroundSession::send_input;
}
