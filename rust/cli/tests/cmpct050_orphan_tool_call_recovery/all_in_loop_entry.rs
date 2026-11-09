//! Feature: spec/features/orphan-tool-call-defeats-execute-compaction-trailing-user-pop-removes-a-toolresult-after-the-cmpct-029-cleanup-ran.feature
use super::*;

// Source-shape pins: every in-loop compaction entry path must funnel through
// begin_compaction_recovery (the choke point that now owns the safety net)
// and none may call execute_compaction directly.
#[test]
fn all_in_loop_entry_paths_funnel_through_the_choke_point() {
    let src = stream_loop_source_tree::stream_loop_source();

    // @step Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    // (Structural: the guarantee exists because the wiring is shared.)

    // @step When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    // The in-loop restart macro is the single place compaction executes
    // for paths B, C, D, 032-in-loop, and overflow; the CMPCT-032 POST-LOOP
    // safety net (stream_loop.rs, after the loop breaks) is the second site.
    // Both funnel through begin_compaction_recovery FIRST — that is the
    // contract this test pins.
    let real_call_sites: Vec<String> = src
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.contains("execute_compaction_and_capture_events(")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("///")
        })
        .map(|l| l.trim().to_string())
        .collect();
    assert_eq!(
        real_call_sites.len(),
        2,
        "stream_loop.rs must have exactly two call sites for \
         execute_compaction_and_capture_events — the in_loop_compaction_restart! \
         macro (paths B/C/D/032/overflow) and the CMPCT-032 POST-LOOP safety \
         net. Both funnel through begin_compaction_recovery first. Sites: \
         {real_call_sites:?}"
    );
    // And the CMPCT-032 POST-LOOP arm must call begin_compaction_recovery
    // (post_loop_text) BEFORE execute_compaction_and_capture_events — the
    // in-loop macro's caller contract pins the other site (its doc
    // comment requires begin_compaction_recovery upstream).
    // BUG-306: the CMPCT-032 POST-LOOP arm lives in stream_loop/finish.rs.
    let finish = stream_loop_source_tree::stream_loop_module("finish");
    let post_loop_helper = line_number_of(&finish, "let mut post_loop_text = String::new();")
        .expect("the CMPCT-032 POST-LOOP arm must exist");
    let post_loop_exec = line_number_of(&finish, "execute_compaction_and_capture_events(")
        .expect("the CMPCT-032 POST-LOOP execution site must exist");
    assert!(
        post_loop_helper < post_loop_exec,
        "the CMPCT-032 POST-LOOP arm must call begin_compaction_recovery \
         BEFORE execute_compaction_and_capture_events — helper at line \
         {post_loop_helper}, exec at line {post_loop_exec}"
    );
    // No stream-loop path may call execute_compaction directly: pre-prompt
    // compaction (Path A) now runs the compactor sub-agent, which reads the
    // persisted originals instead of building an in-view instruction.
    let direct_calls: Vec<String> = src
        .lines()
        .filter(|l| {
            l.trim().contains("execute_compaction(session,")
                || l.trim().contains("execute_compaction(&mut session,")
        })
        .map(|l| l.trim().to_string())
        .collect();
    assert!(
        direct_calls.is_empty(),
        "every stream-loop compaction path must go through begin_compaction_recovery \
         or the compactor sub-agent: {direct_calls:?}"
    );
    let preflight = stream_loop_source_tree::stream_loop_module("preflight");
    assert!(
        preflight.contains("run_compactor_sub_agent_round_cancellable("),
        "pre-prompt compaction (Path A) must run the compactor sub-agent"
    );
}

#[test]
fn begin_compaction_recovery_owns_the_safety_net_after_mutation() {
    let src = read_repo_file("rust/cli/src/interactive/recovery_compaction.rs");

    // @step Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    let body = function_body(&src, "begin_compaction_recovery");

    // @step When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    // The synthetic injection must be invoked from within
    // begin_compaction_recovery — after the pop + partial-text flush — so
    // the post-mutation state is the one guaranteed clean.
    assert!(
        body.contains("inject_synthetic_tool_results_for_orphans"),
        "CMPCT-050: begin_compaction_recovery must run the orphan safety net \
         (inject_synthetic_tool_results_for_orphans) so EVERY entry path \
         (B/C/D/032/overflow) leaves session.messages tool-pair-clean"
    );

    // And it must run AFTER the pop step, not before it.
    let pop_pos = body
        .find("session.messages.pop()")
        .expect("the trailing-User pop must still exist in begin_compaction_recovery");
    let inject_pos = body
        .find("inject_synthetic_tool_results_for_orphans")
        .expect("safety net must be present");
    assert!(
        inject_pos > pop_pos,
        "CMPCT-050: the safety net must run AFTER any pop/mutation — \
         cleanup-before-pop is exactly the ordering this bug fixes"
    );

    // @step Then the orphan call receives a synthetic User(ToolResult) marked "cancelled_by_context_limit"
    // (Guaranteed by the unit-level test above; the ordering pin is here.)

    // @step And execute_compaction's orphan guard passes for that path
    // (Verified end-to-end by prompt_cancelled_after_tool_result_compacts_cleanly.)
}

#[tokio::test]
async fn manual_compact_recovers_persisted_orphan() {
    ensure_test_data_dir();

    // @step Given a session file whose messages contain an orphan tool_call from a previously failed compaction
    // (Expressed as: a live Session with a persisted orphan — exactly what
    // compact_session finds after a failed in-loop compaction.)
    let mut session = fresh_session();
    let call = make_tool_call("toolu_persisted", Some("call_persisted_01"), "bash");
    session.messages.push(user_text_message("earlier prompt"));
    session.messages.push(assistant_tool_call_message(call));
    session.messages.push(user_text_message("follow-up prompt"));
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_err(),
        "precondition: session must start with one persisted orphan"
    );

    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // @step When the user runs /compact
    // (The manual /compact twins — codelet-sessions compact_session and the
    // NAPI session_compact — must close persisted orphans with
    // inject_synthetic_tool_results_for_orphans before calling
    // execute_compaction. We exercise the shared helper contract they must
    // call.)
    inject_synthetic_tool_results_for_orphans(&mut session.messages);
    let result = execute_compaction(&mut session, compaction_in_progress, None).await;

    // @step Then compaction succeeds and DAG construction begins
    // @step And no "Compaction failed: execute_compaction refuses to proceed" error is returned to the user
    assert!(
        result.is_ok(),
        "CMPCT-050: manual /compact on a session with a persisted orphan must \
         SUCCEED after preflight sanitize, got: {result:?}"
    );
    // The /compact wiring itself is pinned structurally in
    // manual_compact_twins_sanitize_before_execute below.
}

#[test]
fn manual_compact_twins_sanitize_before_execute() {
    // @step Given a session file whose messages contain an orphan tool_call from a previously failed compaction
    // @step When the user runs /compact
    // The manual /compact entry must close tool-pair orphans (via
    // inject_synthetic_tool_results_for_orphans) BEFORE it queues the
    // compactor request, so a persisted orphan cannot wedge the session.
    // BUG-516: manual compaction lives in
    // BackgroundSession::start_manual_compaction and queues the compactor
    // sub-agent round instead of calling execute_compaction.

    let sessions_src = read_repo_file("rust/sessions/src/background_session/compaction_handoff.rs");

    let sanitize = "inject_synthetic_tool_results_for_orphans";

    let sessions_call = line_number_of(&sessions_src, sanitize).expect(
        "CMPCT-050: start_manual_compaction must call \
                 inject_synthetic_tool_results_for_orphans before queueing compaction",
    );
    let sessions_queue = line_number_of(&sessions_src, "self.queue_manual_compaction(")
        .expect("start_manual_compaction must queue the compactor request");
    assert!(
        sessions_call < sessions_queue,
        "the preflight orphan-closing must run BEFORE the compactor request is \
         queued (sanitize at line {sessions_call}, queue at line {sessions_queue})"
    );
}

#[tokio::test]
async fn execute_compaction_still_refuses_unrecovered_orphan() {
    ensure_test_data_dir();

    // @step Given a session whose messages contain an orphan Assistant(ToolCall) with no matching User(ToolResult)
    let mut session = fresh_session();
    let call = make_tool_call("toolu_guard", Some("call_guard_still"), "bash");
    session.messages.push(user_text_message("prompt"));
    session.messages.push(assistant_tool_call_message(call));
    let messages_len_before = session.messages.len();

    // @step And no reconciliation, drain, or synthetic injection has been performed
    // (We call execute_compaction directly — bypassing all recovery paths.)

    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is invoked directly
    let result = execute_compaction(&mut session, compaction_in_progress.clone(), None).await;

    // @step Then it returns an error listing the orphan call_id
    assert!(
        result.is_err(),
        "the defensive guard must still refuse on an unrecovered orphan"
    );
    let err = result.err().unwrap();
    assert!(
        err.to_string().contains("call_guard_still"),
        "the refusal error must list the offending call_id, got: {err}"
    );
    assert_eq!(
        session.messages.len(),
        messages_len_before,
        "no compaction instruction may be pushed when the guard refuses"
    );

    // @step And the defensive guard behavior of CMPCT-029 is unchanged
    // The guard fires before any mutation, so the flag is untouched.
    assert!(!compaction_in_progress.load(Ordering::SeqCst));
}
