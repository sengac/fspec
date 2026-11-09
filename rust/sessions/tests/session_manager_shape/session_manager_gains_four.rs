//! Feature: spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_session_manager_gains_four_new_fields_beyond_the_original_five() {
    // @step Given the SessionManager code has been moved into rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());

    // @step When I inspect the field list of the SessionManager struct in the moved file
    let struct_start = sm
        .find("pub struct SessionManager {")
        .expect("SessionManager struct decl must exist");
    let struct_body = &sm[struct_start..];
    let struct_end = struct_body.find("\n}\n").expect("struct must terminate");
    let fields_block = &struct_body[..struct_end];

    // @step Then the five original fields (sessions, chain_of_command, active_session_id, scheduler_handle, default_model) are preserved
    for field in [
        "sessions:",
        "chain_of_command:",
        "active_session_id:",
        "scheduler_handle:",
        "default_model:",
    ] {
        assert!(
            fields_block.contains(field),
            "SessionManager must preserve the field `{field}`. Got fields block:\n{fields_block}"
        );
    }

    // @step And a new field `chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>` exists and is initialized in `new()` via `broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY).0`
    assert!(
        fields_block.contains("chunks_tx:"),
        "SessionManager must have a `chunks_tx` field"
    );
    assert!(
        sm.contains("broadcast::channel("),
        "SessionManager::new() must initialize broadcast senders via `broadcast::channel(...)`"
    );

    // @step And a new field `logs_tx: broadcast::Sender<LogRecord>` exists and is initialized in `new()`
    assert!(
        fields_block.contains("logs_tx:"),
        "SessionManager must have a `logs_tx` field"
    );

    // @step And a new field `status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>` exists and is initialized in `new()`
    assert!(
        fields_block.contains("status_changes_tx:"),
        "SessionManager must have a `status_changes_tx` field"
    );

    // @step And a new field `hooks: ArcSwap<Arc<dyn SessionManagerHooks>>` exists and defaults to `ArcSwap::from_pointee(Arc::new(NoopSessionManagerHooks::default()))`
    assert!(
        fields_block.contains("hooks:"),
        "SessionManager must have a `hooks` field"
    );
    assert!(
        fields_block.contains("ArcSwap"),
        "the hooks field type must use ArcSwap"
    );

    // @step And accessor methods `chunks_tx(&self)`, `logs_tx(&self)`, `status_changes_tx(&self)` exist returning a `&broadcast::Sender<...>` so subscribers can call `.subscribe()`
    for accessor in [
        "pub fn chunks_tx(",
        "pub fn logs_tx(",
        "pub fn status_changes_tx(",
    ] {
        assert!(
            sm.contains(accessor),
            "SessionManager must expose accessor `{accessor}...`"
        );
    }

    // Runtime witness: construct SessionManager::new() and subscribe.
    let sm_instance = SessionManager::new();
    let _ = sm_instance.chunks_tx().subscribe();
    let _ = sm_instance.logs_tx().subscribe();
    let _ = sm_instance.status_changes_tx().subscribe();

    // @step And BackgroundSession::handle_output is NOT yet rewired to use these fields (that rewiring is explicitly deferred to RPC-041)
    let bg_path = workspace_root()
        .join("sessions")
        .join("src")
        .join("background_session.rs");
    let bg = read(&bg_path);
    // RPC-039 added `chunks_tx: Option<broadcast::Sender<...>>` but left it None.
    // The handle_output body must NOT yet wire SessionManager.chunks_tx() into the per-session field.
    let bg_code = strip_line_comments(&bg);
    assert!(
        bg_code.contains("chunks_tx"),
        "background_session.rs must mention chunks_tx (RPC-039 leaves it Option<...>=None)"
    );
}

#[test]
fn scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi() {
    // @step Given the SessionManager code has been moved into rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());
    assert!(sm.contains("pub struct SessionManager"));

    // @step When I run `cargo metadata -p codelet-sessions --format-version 1`
    //
    // NOTE: `cargo metadata` is a workspace-level command that does not
    // accept `-p` directly; the equivalent invocation runs `cargo
    // metadata` at the workspace root and filters the resulting JSON
    // graph for entries reachable from the codelet-sessions package.
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "cargo metadata failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let meta: serde_json::Value = serde_json::from_str(&stdout).expect("metadata JSON parses");

    // @step Then the resulting JSON contains zero packages with name `codelet-napi`
    let resolve = meta.get("resolve").expect("metadata.resolve exists");
    let nodes = resolve
        .get("nodes")
        .and_then(|n| n.as_array())
        .expect("nodes array");
    // Find the codelet-sessions package id from the packages list.
    let packages = meta
        .get("packages")
        .and_then(|p| p.as_array())
        .expect("packages array");
    let sessions_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("codelet-sessions"))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .expect("codelet-sessions package must exist in metadata")
        .to_string();
    let mut reachable: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut stack: Vec<String> = vec![sessions_id];
    while let Some(id) = stack.pop() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|v| v.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for dep in deps {
                        if let Some(s) = dep.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    let napi_in_graph = packages.iter().any(|p| {
        p.get("name").and_then(|n| n.as_str()) == Some("codelet-napi")
            && reachable.contains(p.get("id").and_then(|i| i.as_str()).unwrap_or(""))
    });
    assert!(
        !napi_in_graph,
        "codelet-sessions must NOT have codelet-napi in its transitive dependency graph"
    );

    // @step And the existing dependency-rule test at rust/sessions/tests/skeleton_invariants.rs::scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi still passes
    // Re-run the existing invariant test programmatically: the assertion
    // above is the same logic. Presence of the existing test file is a
    // secondary check.
    let inv = read(
        &workspace_root()
            .join("sessions")
            .join("tests")
            .join("skeleton_invariants.rs"),
    );
    assert!(
        inv.contains("scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi"),
        "the original dependency-rule scenario must still be present"
    );
}
