//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_the_moved_background_session_rs_has_no_napi_references() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step When I grep the moved file for the regex `napi::|use napi|#[napi`
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in moved.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("napi::") || line.contains("use napi") || line.contains("#[napi") {
            violations.push(format!("{lineno}: {line}"));
        }
    }
    // @step Then I find zero matches in rust/sessions/src/background_session.rs
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn scenario_the_moved_background_session_rs_has_no_crate_persistence_or_crate_types_imports() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    // @step When I grep the moved file for the regex `crate::persistence|crate::types`
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in moved.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("crate::persistence") || line.contains("crate::types") {
            violations.push(format!("{lineno}: {line}"));
        }
    }
    // @step Then I find zero matches in rust/sessions/src/background_session.rs
    assert!(violations.is_empty(), "{}", violations.join("\n"));
    // @step And every persistence import resolves to codelet_core::persistence
    let moved_code = strip_line_comments(&moved);
    let persistence_symbols = [
        "load_session",
        "append_message_with_metadata",
        "update_session_tokens",
        "MessageEnvelope",
        "MessagePayload",
        "UserMessage",
        "UserContent",
        "AssistantMessage",
        "AssistantContent",
    ];
    let mentions_any_persistence_symbol =
        persistence_symbols.iter().any(|s| moved_code.contains(s));
    if mentions_any_persistence_symbol {
        assert!(moved_code.contains("codelet_core::persistence"));
    }
    // @step And every FspecResult reference resolves to codelet_rpc_types::FspecResult
    assert!(moved.contains("FspecResult"));
    assert!(moved.contains("codelet_rpc_types"));
}

#[test]
pub(super) fn scenario_global_chunk_callback_static_struct_and_unsafe_impls_are_removed() {
    // @step Given the napi-side rust/napi/src/session_manager.rs file previously declared `static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback>`, a `struct GlobalChunkCallback { callback: ThreadsafeFunction<GlobalChunkCallbackArgs> }`, and `unsafe impl Send for GlobalChunkCallback {}` / `unsafe impl Sync for GlobalChunkCallback {}`
    let napi = read(&napi_shell_path());
    let napi_code = strip_line_comments(&napi);
    // @step When I grep rust/napi/src/session_manager.rs for the regex `GLOBAL_CHUNK_CALLBACK|struct GlobalChunkCallback|unsafe impl (Send|Sync) for GlobalChunkCallback`
    let mut violations: Vec<String> = Vec::new();
    for (idx, line) in napi.lines().enumerate() {
        let code = match line.find("//") {
            Some(pos) => &line[..pos],
            None => line,
        };
        if code.contains("GLOBAL_CHUNK_CALLBACK")
            || code.contains("struct GlobalChunkCallback {")
            || (code.contains("unsafe impl") && code.contains("for GlobalChunkCallback "))
            || (code.contains("unsafe impl") && code.contains("for GlobalChunkCallback{"))
        {
            violations.push(format!("{}: {}", idx + 1, line));
        }
    }
    // @step Then I find zero matches in the file
    assert!(violations.is_empty(), "{}", violations.join("\n"));
    // @step And the `GlobalChunkCallbackArgs` `#[napi(object)]` struct still exists because it is the TS-facing wire shape
    assert!(napi_code.contains("pub struct GlobalChunkCallbackArgs"));
    assert!(napi.contains("#[napi(object)]"));
}
