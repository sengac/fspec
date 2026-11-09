//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_the_moved_background_session_rs_has_no_napi_references() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step When I grep the moved file for the regex `napi::|use napi|#\[napi`
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
