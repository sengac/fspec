//! BUG-154: Tests for set_owning_manager in create_session_with_id.
//!
//! Feature: spec/features/missing-set-owning-manager-in-create-session-with-id-breaks-agentmanager-communication.feature
//!
//! These tests verify that create_session_with_id calls set_owning_manager
//! before spawning the agent loop, matching the behavior in
//! create_session_from_manifest and create_isolated_session_with_id.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

/// Workspace root (one level above this crate's manifest dir).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn session_manager_path() -> PathBuf {
    workspace_root().join("sessions").join("src").join("session_manager.rs")
}

/// Find the body of a function in source code
fn find_function_body(content: &str, func_name: &str) -> String {
    let func_pattern = format!("pub async fn {}", func_name);
    if let Some(start) = content.find(&func_pattern) {
        let brace_start = content[start..].find('{').unwrap_or(0) + start;
        let rest = &content[brace_start..];
        let mut depth = 0;
        let mut end = brace_start;
        for (i, c) in rest.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        content[brace_start..end].to_string()
    } else {
        String::new()
    }
}

/// Feature: spec/features/missing-set-owning-manager-in-create-session-with-id-breaks-agentmanager-communication.feature
/// Scenario: create_session_with_id sets owning_manager before spawning agent loop
#[test]
fn create_session_with_id_sets_owning_manager_before_spawn() {
    // @step Given a SessionManager instance with no existing sessions
    let sm_content = read(&session_manager_path());
    assert!(
        sm_content.contains("pub async fn create_session_with_id"),
        "session_manager.rs must contain create_session_with_id"
    );

    // @step When create_session_with_id is called with a valid UUID and model string
    // @step Then the created session must have owning_manager set to the SessionManager's self_weak reference
    let func_body = find_function_body(&sm_content, "create_session_with_id");
    assert!(
        func_body.contains("set_owning_manager"),
        "create_session_with_id must call set_owning_manager on the created session"
    );

    // @step And the owning_manager must be set before spawn_agent_loop is called
    let set_owning_pos = func_body
        .find("set_owning_manager")
        .expect("set_owning_manager must be called");
    let spawn_agent_pos = func_body
        .find("spawn_agent_loop")
        .expect("spawn_agent_loop must be called");
    assert!(
        set_owning_pos < spawn_agent_pos,
        "set_owning_manager (pos {}) must be called BEFORE spawn_agent_loop (pos {})",
        set_owning_pos,
        spawn_agent_pos
    );
}

/// Feature: spec/features/missing-set-owning-manager-in-create-session-with-id-breaks-agentmanager-communication.feature
/// Scenario: AgentManager handler receives non-None owning_manager from create_session_with_id
#[test]
fn agent_manager_handler_receives_non_none_owning_manager() {
    // @step Given a session created via create_session_with_id
    let sm_content = read(&session_manager_path());
    let func_body = find_function_body(&sm_content, "create_session_with_id");

    // @step When the agent loop registers the AgentManager handler via register_agent_manager_handler
    // @step Then the handler must capture a non-None owning_manager reference
    // The owning_manager is set via self.self_weak.get().cloned().unwrap_or_default()
    // which returns a Weak<SessionManager> that can be upgraded to Arc<SessionManager>.
    assert!(
        func_body.contains("set_owning_manager"),
        "create_session_with_id must call set_owning_manager"
    );
    assert!(
        func_body.contains("self_weak"),
        "create_session_with_id must use self_weak for the owning_manager reference"
    );

    // @step And the handler must be able to look up sessions through the owning manager
    // The owning_manager Weak reference is set before spawn_agent_loop, so when the
    // agent loop registers the handler, it can resolve the owning manager.
    let set_owning_pos = func_body
        .find("set_owning_manager")
        .expect("set_owning_manager must be called");
    let spawn_agent_pos = func_body
        .find("spawn_agent_loop")
        .expect("spawn_agent_loop must be called");
    assert!(
        set_owning_pos < spawn_agent_pos,
        "set_owning_manager must precede spawn_agent_loop so the handler captures a non-None reference"
    );
}

/// Feature: spec/features/missing-set-owning-manager-in-create-session-with-id-breaks-agentmanager-communication.feature
/// Scenario: create_session_with_id and create_session_from_manifest set owning_manager consistently
#[test]
fn both_creation_paths_set_owning_manager_consistently() {
    // @step Given a SessionManager instance
    let sm_content = read(&session_manager_path());

    // @step When a session is created via create_session_with_id
    let with_id_body = find_function_body(&sm_content, "create_session_with_id");

    // @step And another session is created via create_session_from_manifest
    let from_manifest_body = find_function_body(&sm_content, "create_session_from_manifest");

    // @step Then both sessions must have owning_manager set to the same SessionManager instance
    // Both must call set_owning_manager with self.self_weak.get().cloned().unwrap_or_default()
    assert!(
        with_id_body.contains("set_owning_manager"),
        "create_session_with_id must call set_owning_manager"
    );
    assert!(
        from_manifest_body.contains("set_owning_manager"),
        "create_session_from_manifest must call set_owning_manager"
    );

    // Both must use the same self_weak pattern
    assert!(
        with_id_body.contains("self.self_weak.get().cloned().unwrap_or_default()"),
        "create_session_with_id must use self.self_weak.get().cloned().unwrap_or_default()"
    );
    assert!(
        from_manifest_body.contains("self.self_weak.get().cloned().unwrap_or_default()"),
        "create_session_from_manifest must use self.self_weak.get().cloned().unwrap_or_default()"
    );

    // Both must set owning_manager before spawn_agent_loop
    let with_id_set_pos = with_id_body
        .find("set_owning_manager")
        .expect("set_owning_manager in create_session_with_id");
    let with_id_spawn_pos = with_id_body
        .find("spawn_agent_loop")
        .expect("spawn_agent_loop in create_session_with_id");
    assert!(
        with_id_set_pos < with_id_spawn_pos,
        "create_session_with_id: set_owning_manager must precede spawn_agent_loop"
    );

    let manifest_set_pos = from_manifest_body
        .find("set_owning_manager")
        .expect("set_owning_manager in create_session_from_manifest");
    let manifest_spawn_pos = from_manifest_body
        .find("spawn_agent_loop")
        .expect("spawn_agent_loop in create_session_from_manifest");
    assert!(
        manifest_set_pos < manifest_spawn_pos,
        "create_session_from_manifest: set_owning_manager must precede spawn_agent_loop"
    );
}
