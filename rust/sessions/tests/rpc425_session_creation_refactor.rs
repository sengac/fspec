//! RPC-425: Tests for session creation refactoring.
//!
//! Feature: spec/features/extract-shared-session-creation.feature
//!
//! These tests verify that the shared session creation helper exists and is used
//! by both create_session_with_id and create_session_from_manifest.

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

fn helper_path() -> PathBuf {
    workspace_root().join("sessions").join("src").join("session_creation_helper.rs")
}

/// Feature: spec/features/extract-shared-session-creation.feature
/// Scenario: create_session_with_id uses shared helper for session construction
#[test]
fn create_session_with_id_uses_shared_helper() {
    // @step Given a SessionManager with create_session_with_id
    let sm_content = read(&session_manager_path());
    assert!(
        sm_content.contains("pub async fn create_session_with_id"),
        "session_manager.rs must contain create_session_with_id"
    );

    // @step When a session is created via create_session_with_id
    // @step Then the shared helper creates the BackgroundSession
    assert!(
        sm_content.contains("create_background_session_inner"),
        "session_manager.rs must call create_background_session_inner helper"
    );

    // @step And the manifest is saved to disk before the helper is called
    assert!(
        sm_content.contains("save_session"),
        "create_session_with_id must save session manifest"
    );
}

/// Feature: spec/features/extract-shared-session-creation.feature
/// Scenario: create_session_from_manifest uses shared helper for session construction
#[test]
fn create_session_from_manifest_uses_shared_helper() {
    // @step Given a SessionManager with create_session_from_manifest
    let sm_content = read(&session_manager_path());
    assert!(
        sm_content.contains("pub async fn create_session_from_manifest"),
        "session_manager.rs must contain create_session_from_manifest"
    );

    // @step When a session is resumed via create_session_from_manifest
    // @step Then the shared helper creates the BackgroundSession
    assert!(
        sm_content.contains("create_background_session_inner"),
        "session_manager.rs must call create_background_session_inner helper"
    );

    // @step And the manifest is NOT saved to disk
    // Verify that create_session_from_manifest does NOT call save_session
    let from_manifest_section = find_function_body(&sm_content, "create_session_from_manifest");
    assert!(
        !from_manifest_section.contains("save_session"),
        "create_session_from_manifest must NOT save session manifest"
    );
}

/// Feature: spec/features/extract-shared-session-creation.feature
/// Scenario: Shared helper preserves all existing session setup behavior
#[test]
fn shared_helper_preserves_session_setup_behavior() {
    // @step Given the shared session creation helper
    let helper_content = read(&helper_path());
    let sm_content = read(&session_manager_path());

    // @step When it creates a session
    // @step Then credentials are resolved for the provider
    assert!(
        sm_content.contains("resolve_and_set_env_var"),
        "Session manager must resolve credentials before calling helper"
    );

    // @step And lifecycle hooks are loaded from the project path
    assert!(
        helper_content.contains("load_lifecycle_hooks"),
        "Helper must load lifecycle hooks"
    );

    // @step And pre-tool hooks are registered if lifecycle hooks exist
    assert!(
        helper_content.contains("register_pre_tool_hook"),
        "Helper must register pre-tool hooks"
    );

    // @step And MCP session is initialized
    assert!(
        helper_content.contains("init_mcp_session"),
        "Helper must initialize MCP session"
    );

    // @step And agent loop is spawned via hooks
    assert!(
        sm_content.contains("spawn_agent_loop"),
        "Session manager must spawn agent loop after helper returns"
    );

    // @step And session is inserted into in-memory map
    assert!(
        sm_content.contains(".insert(uuid, session)"),
        "Session manager must insert session into map"
    );

    // @step And session-created broadcast is sent
    assert!(
        sm_content.contains("session_created_tx.send"),
        "Session manager must send session-created broadcast"
    );

    // @step And isolation state change is broadcast
    assert!(
        sm_content.contains("isolation_state_change"),
        "Session manager must broadcast isolation state change"
    );

    // @step And footer poller is spawned
    assert!(
        sm_content.contains("spawn_footer_poller"),
        "Session manager must spawn footer poller"
    );

    // @step And metadata update is broadcast
    assert!(
        sm_content.contains("broadcast_metadata_update"),
        "Session manager must broadcast metadata update"
    );
}

/// Feature: spec/features/extract-shared-session-creation.feature
/// Scenario: Model limits and thinking level are set by shared helper
#[test]
fn model_limits_and_thinking_level_are_set() {
    // @step Given the shared session creation helper
    let helper_content = read(&helper_path());

    // @step When it creates a session
    // @step Then the persisted default thinking level is applied
    assert!(
        helper_content.contains("load_default_thinking_level"),
        "Helper must load default thinking level"
    );
    assert!(
        helper_content.contains("set_base_thinking_level"),
        "Helper must set base thinking level"
    );

    // @step And model limits (context window, max output tokens, compaction threshold) are set
    assert!(
        helper_content.contains("resolve_compaction_threshold"),
        "Helper must resolve compaction threshold"
    );
    assert!(
        helper_content.contains("set_model_limits"),
        "Helper must set model limits"
    );
}

/// Feature: spec/features/extract-shared-session-creation.feature
/// Scenario: Both call sites use the shared helper
#[test]
fn both_call_sites_use_shared_helper() {
    // @step Given the shared session creation helper exists
    let sm_content = read(&session_manager_path());
    let helper_content = read(&helper_path());

    // @step When we examine session_manager.rs
    // @step Then create_background_session_inner is called from create_session_with_id
    // Verify the helper function exists in the helper module
    assert!(
        helper_content.contains("pub async fn create_background_session_inner"),
        "session_creation_helper.rs must contain create_background_session_inner"
    );

    // @step And create_background_session_inner is called from create_session_from_manifest
    // Count how many times the helper is called in session_manager.rs
    let helper_calls = sm_content.matches("create_background_session_inner").count();
    // At least 2 call sites (create_session_with_id + create_session_from_manifest)
    assert!(
        helper_calls >= 2,
        "create_background_session_inner should be called from both create_session_with_id and create_session_from_manifest (found {} occurrences)",
        helper_calls
    );
}

/// Find the body of a function in source code
fn find_function_body(content: &str, func_name: &str) -> String {
    let func_pattern = format!("pub async fn {}", func_name);
    if let Some(start) = content.find(&func_pattern) {
        // Find the opening brace
        let brace_start = content[start..].find('{').unwrap_or(0) + start;
        // Find the matching closing brace (simple approach: count braces)
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
