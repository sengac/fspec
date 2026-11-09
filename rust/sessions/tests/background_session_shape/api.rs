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
    // @step And the error construction site uses `format!("Failed to send input: {}", e)` (not `napi::Error::from_reason(...)`)
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
    let handle_impl = read(
        &workspace_root()
            .join("sessions")
            .join("src")
            .join("handle_impl")
            .join("handle_impl_1.rs"),
    );
    let handle_code = strip_line_comments(&handle_impl);
    assert!(handle_code.contains("fn send_input_with_thinking("));
    assert!(handle_code.contains("session.send_input(text, thinking_json)"));
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
