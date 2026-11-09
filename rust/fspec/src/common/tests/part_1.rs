use super::*;

/// RPC-044 regression: production `build_service` MUST construct a
/// real `codelet_sessions::SessionManager` and pass it as an
/// `Arc<dyn SessionManagerHandle>` into
/// `SharedFspecService::with_session_manager(watcher, session_manager)`
/// — the prior `SharedFspecService::new(watcher)` call is replaced.
/// Without this, the `fspec` binary cannot drive real agent sessions
/// through the NAPI-free `codelet-sessions` crate; the AgentView
/// would observe an empty fallback `chunks_rx` and see no output.
#[test]
#[serial]
fn build_service_wires_session_manager_into_shared_service() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    let src = codelet_test_helpers::source_composition::read_tree_or_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/common.rs"
    ))
    .expect("read common.rs");

    // @step When I open `rust/fspec/src/common.rs`
    // @step Then the file contains the literal substring `use codelet_sessions::SessionManager`
    assert!(
        src.contains("use codelet_sessions::SessionManager"),
        "common.rs must `use codelet_sessions::SessionManager` after RPC-044"
    );

    // @step And the file contains the literal substring `use codelet_core::SessionManagerHandle`
    assert!(
        src.contains("use codelet_core::SessionManagerHandle"),
        "common.rs must `use codelet_core::SessionManagerHandle` after RPC-044"
    );

    // @step And the `build_service` function constructs its session manager with `SessionManager::new()`, which returns the `Arc<SessionManager>` that holds its own self-reference (BUG-420)
    assert!(
        src.contains("Arc<dyn SessionManagerHandle>"),
        "common.rs must declare a `Arc<dyn SessionManagerHandle>` binding after RPC-044"
    );
    assert!(
        src.contains("SessionManager::new()"),
        "common.rs must construct the session manager via `SessionManager::new()` after RPC-044"
    );

    // @step And the `build_service` function calls `SharedFspecService::with_session_manager(watcher, session_manager)` instead of `SharedFspecService::new(watcher)`
    assert!(
        src.contains("SharedFspecService::with_session_manager(watcher, session_manager)"),
        "common.rs must wire the session_manager through `SharedFspecService::with_session_manager(watcher, session_manager)` after RPC-044"
    );
    let bs_start = src
        .find("pub fn build_service")
        .expect("build_service definition must exist");
    let bs_end = src[bs_start..]
        .find("\n}\n")
        .map(|i| bs_start + i)
        .unwrap_or(src.len());
    let body = &src[bs_start..bs_end];
    assert!(
        !body.contains("SharedFspecService::new(watcher)"),
        "build_service must NOT use the bare `SharedFspecService::new(watcher)` constructor after RPC-044"
    );

    // @step And the `set_data_directory` call still appears before the SessionManager construction
    let set_idx = src
        .find("codelet_common::set_data_directory")
        .expect("set_data_directory call must exist");
    let sm_idx = src
        .find("SessionManager::new()")
        .expect("SessionManager::new() call must exist in build_service after RPC-044");
    assert!(
        set_idx < sm_idx,
        "set_data_directory must be called BEFORE SessionManager::new() in build_service"
    );

    // @step When `build_service` is invoked against a temp workspace
    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    let service = build_service(workspace).expect("build_service");

    // @step Then `service.cwd()` returns `Some(temp_workspace_path)` as before
    assert_eq!(
        service.cwd(),
        Some(&workspace.to_path_buf()),
        "build_service must still call SharedFspecService::with_cwd(workspace) after RPC-044"
    );

    // @step And the literal substring `SharedFspecService::with_session_manager(watcher, session_manager)` is present in rust/fspec/src/common.rs
    assert!(
        src.contains("SharedFspecService::with_session_manager(watcher, session_manager)"),
        "common.rs must contain the literal `SharedFspecService::with_session_manager(watcher, session_manager)` substring (RPC-044)"
    );

    // @step And a chunk sent through `service.chunks_tx()` is received via `service.chunks_rx()` proving the SessionManager broadcast is live
    let tx = service.chunks_tx();
    let mut rx = service.chunks_rx();
    let session_id =
        codelet_rpc_types::SessionId::new("00000000-0000-0000-0000-000000000000".to_string());
    let chunk = codelet_rpc_types::StreamChunk::text("hello-rpc-044".to_string());
    tx.send((session_id.clone(), chunk.clone()))
        .expect("send chunk through service.chunks_tx()");
    let (got_id, got_chunk) = rx
        .try_recv()
        .expect("service.chunks_rx() must receive the chunk sent through service.chunks_tx()");
    assert_eq!(got_id, session_id);
    match got_chunk {
        codelet_rpc_types::StreamChunk::Text { text, .. } => assert_eq!(text, "hello-rpc-044"),
        other => panic!("unexpected chunk variant: {other:?}"),
    }
}

/// RPC-044 regression: rust/fspec/Cargo.toml MUST declare a
/// `codelet-sessions` dependency and MUST NOT declare a
/// `codelet-napi` dependency (the forbidden `fspec → napi` arrow).
#[test]
fn fspec_cargo_toml_declares_sessions_dep_and_not_napi() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    let cargo = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");

    // @step When I open `rust/fspec/Cargo.toml`
    // @step Then the `[dependencies]` table contains `codelet-sessions.workspace = true` or `codelet-sessions = { workspace = true }`
    let has_sessions_dep = cargo.contains("codelet-sessions.workspace = true")
        || cargo.contains("codelet-sessions = { workspace = true }");
    assert!(
        has_sessions_dep,
        "rust/fspec/Cargo.toml must declare codelet-sessions as a workspace dependency after RPC-044"
    );

    // @step And the file contains zero occurrences of the literal substring `codelet-napi` (outside comments)
    let stripped = strip_cargo_comments(&cargo);
    assert!(
        !stripped.contains("codelet-napi"),
        "rust/fspec/Cargo.toml MUST NOT declare `codelet-napi` (the forbidden `fspec → napi` arrow)"
    );
}
