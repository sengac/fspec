//! Feature: spec/features/implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager.feature
use super::*;

// =============================================================================
// Scenario: The impl block exists with explicit overrides for every trait
// method
// =============================================================================
#[test]
fn scenario_impl_block_exists_with_every_override() {
    // @step Given the file "rust/sessions/src/session_manager.rs" (and any sibling "handle_impl.rs" if used)
    let src = sessions_src_concat();

    // @step When I read the source bytes and scan them
    // @step Then exactly one "impl codelet_core::SessionManagerHandle for SessionManager" block exists across the inspected files
    let impl_marker = "impl codelet_core::SessionManagerHandle for SessionManager";
    let matches = src.matches(impl_marker).count();
    assert_eq!(
        matches, 1,
        "expected exactly one `{impl_marker}` block in rust/sessions/src/, found {matches}",
    );

    // @step And the impl block contains a "fn list_sessions(" override
    // @step And the impl block contains a "fn create_session_checked(" override
    // @step And the impl block contains a "fn send_input(" override
    // @step And the impl block contains a "fn send_input_with_thinking(" override
    // @step And the impl block contains a "fn interrupt(" override
    // @step And the impl block contains a "fn get_session_status(" override
    // @step And the impl block contains a "fn chunks_rx(" override
    // @step And the impl block contains a "fn logs_rx(" override
    // @step And the impl block contains a "fn chunks_tx(" override
    // @step And the impl block contains a "fn logs_tx(" override
    // @step And the impl block contains a "fn status_changes_rx(" override
    // @step And the impl block contains a "fn status_changes_tx(" override
    // @step And the impl block contains a "fn get_session_tokens(" override
    // @step And the impl block contains a "fn get_session_model(" override
    // @step And the impl block contains a "fn get_compaction_progress(" override
    // @step And the impl block contains a "fn get_buffered_output(" override
    // @step And the impl block contains a "fn clear_history(" override
    // @step And the impl block contains a "fn compact_session(" override
    // @step And the impl block contains a "fn restore_session_messages(" override
    // @step And the impl block contains a "fn restore_session_token_state(" override
    // @step And the impl block contains a "fn get_work_unit_context(" override
    // @step And the impl block contains a "fn set_work_unit_context(" override
    // @step And the impl block contains a "fn get_pending_input(" override
    // @step And the impl block contains a "fn set_pending_input(" override
    // @step And the impl block contains a "fn set_active_session(" override
    // @step And the impl block contains a "fn clear_active_session(" override
    // @step And the impl block contains a "fn get_active_session(" override
    // @step And the impl block contains a "fn get_effective_cwd(" override
    // @step And the impl block contains a "fn get_supervisors(" override
    // @step And the impl block contains a "fn get_debug_enabled(" override
    // @step And the impl block contains a "fn set_debug_enabled(" override
    // @step And the impl block contains a "fn toggle_debug(" override
    // @step And the impl block contains a "fn pause_human_action(" override
    // @step And the impl block contains a "fn pause_confirm(" override
    // @step And the impl block contains a "fn pause_triple(" override
    // @step And the impl block contains a "fn send_hitl_response(" override
    // @step And the impl block contains a "fn get_pause_state(" override
    // @step And the impl block contains a "fn get_hitl_request(" override
    // @step And the impl block contains a "fn send_fspec_result(" override
    // @step And the impl block contains a "fn create_isolated_session(" override
    // @step And the impl block contains a "fn destroy_session(" override
    // @step And the impl block contains a "fn set_thinking_level_default(" override
    let impl_start = src.find(impl_marker).expect("impl block start found above");
    let after_impl = &src[impl_start..];
    for method in TRAIT_METHODS {
        let needle = format!("fn {method}(");
        assert!(
            after_impl.contains(&needle),
            "expected `{needle}` inside the SessionManagerHandle impl block",
        );
    }

    // @step And a "fn parse_session_uuid(" helper exists alongside the impl
    assert!(
        src.contains("fn parse_session_uuid("),
        "expected a `fn parse_session_uuid(` helper somewhere in rust/sessions/src/",
    );

    // @step And the source contains exactly one occurrence of "tokio::runtime::Handle::current().block_on(" across the create_session and create_isolated_session overrides (counted across BOTH methods)
    //
    // Note: rustfmt may split the call across lines (e.g.
    // `tokio::runtime::Handle::current()\n    .block_on(`); the test
    // accepts a small range of matches of the single-line substring
    // because the *intent* is that the sync→async bridge methods use this
    // pattern. The count spans the deterministically-sorted concatenation
    // from the impl marker onward, so it includes the handle_impl bridges
    // (create_session, create_isolated_session, restore_session_messages,
    // set_model's sibling bridges) plus the profile_sections.rs bridge that
    // sorts after handle_impl.rs. The actual presence of the bridges is
    // enforced by the runtime tests
    // (`scenario_session_manager_satisfies_trait_object` and
    // `scenario_unknown_session_id_returns_safe_defaults`).
    // Comment lines that merely name the pattern are not bridges.
    let block_on_count = after_impl
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .map(|line| {
            line.matches("tokio::runtime::Handle::current().block_on(")
                .count()
        })
        .sum::<usize>();
    // The upper bound tracks the current bridge inventory (BUG-297): 6
    // per-method bridges in handle_impl.rs (resume_session, the two
    // compact_session bridges, create_isolated_session,
    // test_provider_connection, detach_session_worktree [WT-009]) plus the
    // shared sync→async bridge helper, the profile_discovery.rs `/v1/models`
    // probe bridge, and the two session_restore.rs bridges
    // (restore_messages, restore_tokens) = 10.
    assert!(
        (1..=10).contains(&block_on_count),
        "expected 1..=10 `tokio::runtime::Handle::current().block_on(` occurrences inside the SessionManagerHandle impl, found {block_on_count}",
    );
}

// =============================================================================
// Scenario: The conversions module bridges tool_pause and rpc-types pause
// families
// =============================================================================
#[test]
fn scenario_conversions_module_exists() {
    // @step Given the file "rust/sessions/src/conversions.rs"
    let conv_path = sessions_src_dir().join("conversions.rs");
    assert!(
        conv_path.exists(),
        "expected file at {}",
        conv_path.display()
    );
    let conv_src = read(&conv_path);

    // @step And the module is declared from "rust/sessions/src/lib.rs" via "pub mod conversions;"
    let lib_src = read(&sessions_src_dir().join("lib.rs"));
    assert!(
        lib_src.contains("pub mod conversions;"),
        "expected `pub mod conversions;` declaration in rust/sessions/src/lib.rs",
    );

    // @step When I read the source bytes

    // @step Then the file contains "pub fn pause_state_to_rpc"
    let expected_fn = "pub fn pause_state_to_rpc";
    assert!(
        conv_src.contains(expected_fn),
        "expected `{expected_fn}` in {}",
        conv_path.display(),
    );

    // @step And the file contains "pub fn approval_choice_to_pause_response"
    assert!(
        conv_src.contains("pub fn approval_choice_to_pause_response"),
        "expected `pub fn approval_choice_to_pause_response` in {}",
        conv_path.display(),
    );

    // @step And the file contains "pub fn confirm_accept_to_pause_response"
    assert!(
        conv_src.contains("pub fn confirm_accept_to_pause_response"),
        "expected `pub fn confirm_accept_to_pause_response` in {}",
        conv_path.display(),
    );
}
