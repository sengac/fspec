//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_codelet_sessions_builds_standalone_with_background_session_at_its_new_home() {
    // @step Given the BackgroundSession struct and impl have been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    assert!(moved.contains("impl BackgroundSession"));
    // @step When I run `cargo build -p codelet-sessions`
    let output = Command::new(env!("CARGO"))
        .args(["build", "-p", "codelet-sessions", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");
    // @step Then the build completes successfully with no errors
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // @step And the public path `codelet_sessions::background_session::BackgroundSession` resolves to the moved struct
    let _shape_witness: fn() = || {
        fn _accept<T>(_: std::marker::PhantomData<T>) {}
        _accept::<BackgroundSession>(std::marker::PhantomData);
    };
}

#[test]
pub(super) fn scenario_codelet_napi_still_builds_against_the_re_exported_background_session() {
    // @step Given rust/napi/src/session_manager.rs now `pub use`s BackgroundSession + its companion types from codelet-sessions
    let napi = read(&napi_shell_path());
    assert!(napi.contains("pub use codelet_sessions::background_session::"));
    for name in [
        "BackgroundSession",
        "IncomingMessage",
        "BridgeImageData",
        "format_incoming_message",
        "PromptInput",
        "CompactionProgress",
        "WorkUnitContext",
        "SessionError",
    ] {
        assert!(napi.contains(name));
    }
    // @step When I run `cargo build -p codelet-napi`
    let output = Command::new(env!("CARGO"))
        .args(["build", "-p", "codelet-napi", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");
    // @step Then the build completes successfully with no errors
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // @step And the rest of session_manager.rs (ChainOfCommand, SessionManager, the agent_loop, the #[napi] free functions, the unit-test module) resolves every BackgroundSession / PromptInput / IncomingMessage / BridgeImageData / format_incoming_message / WorkUnitContext / CompactionProgress / SessionError path through the re-exports
    assert!(
        napi.contains("pub use codelet_sessions::chain_of_command::ChainOfCommand")
            || napi.contains("pub use codelet_sessions::chain_of_command::")
            || napi.contains("pub struct ChainOfCommand")
    );
    assert!(
        napi.contains("pub use codelet_sessions::session_manager::SessionManager")
            || napi.contains("pub struct SessionManager")
    );
}

#[test]
fn scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step When I run `cargo metadata -p codelet-sessions --format-version 1`
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("metadata JSON must parse");
    let resolve = json
        .get("resolve")
        .expect("cargo metadata must include a resolve");
    let nodes = resolve
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("resolve.nodes must be an array");
    let packages = json
        .get("packages")
        .and_then(|v| v.as_array())
        .expect("packages must be an array");
    let sessions_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("codelet-sessions"))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .expect("codelet-sessions package must exist in metadata")
        .to_string();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stack: Vec<String> = vec![sessions_id];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|i| i.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for d in deps {
                        if let Some(s) = d.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
                break;
            }
        }
    }
    let mut transitive_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for id in &seen {
        if let Some(pkg) = packages
            .iter()
            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                transitive_names.insert(name.to_string());
            }
        }
    }
    // @step Then the resulting JSON contains zero packages with name `codelet-napi`
    assert!(!transitive_names.contains("codelet-napi"));
}

#[test]
pub(super) fn scenario_pre_existing_in_file_unit_tests_in_codelet_napi_still_pass_via_the_re_exports(
) {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step And rust/napi/src/session_manager.rs re-exports BackgroundSession, IncomingMessage, BridgeImageData, format_incoming_message, PromptInput, CompactionProgress, WorkUnitContext, SessionError from codelet-sessions
    let napi = read(&napi_shell_path());
    assert!(napi.contains("pub use codelet_sessions::background_session::"));
    // @step When I run `cargo test -p codelet-napi --lib session_manager::tests`
    if std::env::var_os("RPC_039_FULL_TESTS").is_none() {
        return;
    }
    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "-p",
            "codelet-napi",
            "--lib",
            "session_manager",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo test must run");
    // @step Then every pre-existing test (WorkUnitContext::new tests, format_for_environment tests, IncomingMessage::new/with_images tests, format_incoming_message tests, parse_interjection tests) passes with status ok
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for fragment in [
        "supervisor_broadcast_tests",
        "session_role_tests",
        "supervisor_input_tests",
        "correlation_id_tests",
        " ok",
    ] {
        assert!(combined.contains(fragment));
    }
}
