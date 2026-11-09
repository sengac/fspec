//! Feature: spec/features/implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager.feature
use super::*;

// =============================================================================
// Scenario: SessionManager satisfies the codelet-core SessionManagerHandle
// trait object
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_session_manager_satisfies_trait_object() {
    // @step Given the codelet-sessions crate compiles
    // @step And a tokio multi-threaded runtime is active for the test
    // (Both established by the build + test harness above.)

    // @step When I construct a fresh "codelet_sessions::SessionManager" via "SessionManager::new()"
    let manager = SessionManager::new();

    // @step And I cast its Arc as "Arc<dyn codelet_core::SessionManagerHandle>"
    let handle: Arc<dyn SessionManagerHandle> = manager;

    // @step Then the cast compiles without error
    // (Reaching this line proves the cast compiled.)

    // @step And calling "handle.list_sessions()" on the trait object returns an empty "Vec<SessionInfo>"
    let sessions = handle
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    assert!(
        sessions.is_empty(),
        "expected empty session list, got {} entries",
        sessions.len()
    );
}

// =============================================================================
// Scenario: Every per-session method reports an unknown SessionId (BUG-445)
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_unknown_session_id_is_reported_by_every_method() {
    // @step Given a fresh "SessionManager" wrapped as "Arc<dyn SessionManagerHandle>"
    let handle = make_handle();

    // @step And a "SessionId::new(\"6f1d2c3b-0000-4000-8000-00000000dead\")" that is NOT registered in the manager
    let sid = SessionId::new("6f1d2c3b-0000-4000-8000-00000000dead");

    // @step When I call every per-session method with that "SessionId"
    // (Each `Then`/`And` below performs one call + assertion.)

    // @step Then "get_session_status", "get_session_tokens", "get_session_model", "get_compaction_progress", "get_buffered_output(.., 32)", "get_work_unit_context", "get_pending_input", "get_pause_state", "get_hitl_request" and "get_role" all return "Err(...)" containing the substring "Session not found" (BUG-445)
    let not_found = |getter: &str, outcome: Result<(), String>| {
        let error = outcome.expect_err(getter);
        assert!(error.contains("Session not found"), "{getter}: {error}");
    };
    not_found(
        "get_session_status",
        handle.get_session_status(&sid).map(drop),
    );
    not_found(
        "get_session_tokens",
        handle.get_session_tokens(&sid).map(drop),
    );
    not_found(
        "get_session_model",
        handle.get_session_model(&sid).map(drop),
    );
    not_found(
        "get_compaction_progress",
        handle.get_compaction_progress(&sid).map(drop),
    );
    not_found(
        "get_buffered_output",
        handle.get_buffered_output(&sid, 32).map(drop),
    );
    not_found(
        "get_work_unit_context",
        handle.get_work_unit_context(&sid).map(drop),
    );
    not_found(
        "get_pending_input",
        handle.get_pending_input(&sid).map(drop),
    );
    not_found("get_pause_state", handle.get_pause_state(&sid).map(drop));
    not_found("get_hitl_request", handle.get_hitl_request(&sid).map(drop));
    not_found("get_role", handle.get_role(&sid).map(drop));

    // @step And "get_effective_cwd" returns "Err(...)" for the unknown session (BUG-437: no start-workspace fallback)
    let cwd = handle.get_effective_cwd(&sid);
    assert!(
        cwd.is_err(),
        "expected an error for an unknown session, got {cwd:?}"
    );

    // @step And "get_supervisors" returns "Err(...)" containing the substring "Session not found" (BUG-464)
    not_found("get_supervisors", handle.get_supervisors(&sid).map(drop));

    // @step And "get_debug_enabled" returns "Err(...)" containing the substring "Session not found" (BUG-445)
    not_found(
        "get_debug_enabled",
        handle.get_debug_enabled(&sid).map(drop),
    );

    // @step And "clear_history", "compact_session", "toggle_debug", "pause_human_action", "pause_confirm", "pause_triple", "send_hitl_response", "send_fspec_result", "destroy_session", "restore_session_messages", "restore_session_token_state", "set_work_unit_context", "set_thinking_level", "set_thinking_level_default", "set_role", "set_model" all return "Err(...)" containing the substring "Session not found"
    fn assert_session_not_found<T: std::fmt::Debug>(method: &str, res: Result<T, String>) {
        match res {
            Ok(value) => panic!("{method}: expected Err, got Ok({value:?})"),
            Err(msg) => assert!(
                msg.contains("Session not found"),
                "{method}: expected error containing `Session not found`, got `{msg}`",
            ),
        }
    }

    assert_session_not_found("clear_history", handle.clear_history(&sid));
    assert_session_not_found("compact_session", handle.compact_session(&sid));
    assert_session_not_found(
        "toggle_debug",
        handle.toggle_debug(&sid, "/tmp/fspec-debug-test"),
    );
    assert_session_not_found(
        "pause_human_action",
        handle.pause_human_action(&sid, "request-1", codelet_rpc_types::HumanActionReply::Done),
    );
    assert_session_not_found(
        "pause_confirm",
        handle.pause_confirm(&sid, "request-1", true),
    );
    assert_session_not_found(
        "pause_triple",
        handle.pause_triple(&sid, "request-1", ApprovalChoice::Approve),
    );
    assert_session_not_found(
        "send_hitl_response",
        handle.send_hitl_response(
            &sid,
            "request-1",
            HitlResponse {
                cancelled: false,
                answers: vec![],
            },
        ),
    );
    assert_session_not_found(
        "send_fspec_result",
        handle.send_fspec_result(
            &sid,
            FspecResult {
                success: true,
                data: String::new(),
                error: None,
                system_reminder: None,
                tool_call_id: "tc1".into(),
            },
        ),
    );
    assert_session_not_found("destroy_session", handle.destroy_session(&sid));
    assert_session_not_found(
        "restore_session_messages",
        handle.restore_session_messages(&sid, Vec::new()),
    );
    assert_session_not_found(
        "restore_session_token_state",
        handle.restore_session_token_state(
            &sid,
            TokenRestoreState {
                current_context: 0,
                cumulative_billed_input: 0,
                cumulative_billed_output: 0,
                cumulative_billed_output_second: 0,
                cache_read: 0,
                cache_creation: 0,
            },
        ),
    );
    assert_session_not_found(
        "set_work_unit_context",
        handle.set_work_unit_context(
            &sid,
            Some(WorkUnitContext {
                id: "RPC-042".into(),
                title: "trait impl".into(),
                status: "testing".into(),
            }),
        ),
    );
    assert_session_not_found(
        "set_thinking_level",
        handle.set_thinking_level(&sid, ThinkingLevel::Off),
    );
    assert_session_not_found(
        "set_thinking_level_default",
        handle.set_thinking_level_default(&sid, ThinkingLevel::Off),
    );
    assert_session_not_found("set_role", handle.set_role(&sid, Some("reviewer".into())));
    assert_session_not_found(
        "set_model",
        handle.set_model(&sid, "anthropic", "claude-opus-4-5"),
    );

    // @step And "set_pending_input" and "set_debug_enabled" return "Err(...)" containing the substring "Session not found" (BUG-464)
    not_found(
        "set_pending_input",
        handle.set_pending_input(&sid, Some("draft".into())),
    );
    not_found("set_debug_enabled", handle.set_debug_enabled(&sid, true));
    // @step And "interrupt" returns "Err(...)" containing the substring "Session not found" (BUG-465)
    not_found("interrupt", handle.interrupt(&sid));
    // @step And "set_active_session", "send_input", "send_input_with_thinking" do NOT panic
    assert_eq!(handle.set_active_session(&sid), Ok(()));
    assert!(handle.send_input(&sid, "hello".into()).is_err());
    assert!(handle
        .send_input_with_thinking(&sid, "hello".into(), None)
        .is_err());
}
