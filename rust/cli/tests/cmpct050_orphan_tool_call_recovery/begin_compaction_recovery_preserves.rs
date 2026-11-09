//! Feature: spec/features/orphan-tool-call-defeats-execute-compaction-trailing-user-pop-removes-a-toolresult-after-the-cmpct-029-cleanup-ran.feature
use super::*;

#[test]
fn begin_compaction_recovery_preserves_trailing_user_tool_result() {
    // @step Given a session whose messages end with an Assistant(ToolCall) for call X and the matching User(ToolResult) for call X
    let mut session = fresh_session();
    let call = make_tool_call("toolu_cmpct050", Some("call_cmpct050_x"), "read_file");
    session.messages.push(user_text_message("do something"));
    session.messages.push(assistant_tool_call_message(call));
    session.messages.push(user_tool_result_message(
        "toolu_cmpct050",
        Some("call_cmpct050_x"),
        "ok",
    ));
    let len_before = session.messages.len();

    // Precondition mirrors the bug: the CMPCT-029 cleanup has already run —
    // the list is clean. The pop must not break that.
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "precondition: session starts tool-pair-clean"
    );

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::new();

    // @step When begin_compaction_recovery runs with pop_user_prompt=true
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("helper must succeed");

    // @step Then the trailing User(ToolResult) message is preserved in session.messages
    assert!(
        has_tool_result(session.messages.last().expect("tail must exist")),
        "CMPCT-050: a trailing User(ToolResult) must NOT be popped — it is \
         mid-turn conversation state, not an unconsumed prompt"
    );

    // @step And validate_no_orphan_tool_calls reports zero orphan call_ids for the session
    assert!(
        is_tool_result_with_call_id(
            session.messages.last().expect("tail must exist"),
            "call_cmpct050_x"
        ),
        "the matching tool result for call_cmpct050_x must still be at the tail"
    );
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "CMPCT-050: begin_compaction_recovery must leave session.messages \
         tool-pair-clean (the pop must not re-orphan a tool call)"
    );
    assert_eq!(
        session.messages.len(),
        len_before,
        "no message may be removed when the tail is a User(ToolResult)"
    );
}

#[test]
fn trailing_plain_user_text_prompt_is_still_popped() {
    // @step Given a session whose messages end with a plain User text prompt that the API has not consumed
    let mut session = fresh_session();
    session
        .messages
        .push(user_text_message("earlier conversation"));
    session.messages.push(user_text_message("Hi"));
    let len_before = session.messages.len();

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::new();

    // @step When begin_compaction_recovery runs with pop_user_prompt=true
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("helper must succeed");

    // @step Then the trailing User text prompt is removed from session.messages
    // @step And no User(ToolResult) message is altered or removed
    assert_eq!(
        session.messages.len(),
        len_before - 1,
        "the trailing plain text prompt must still be popped (original behavior)"
    );
    assert!(
        is_plain_user_text(
            session.messages.last().expect("tail must exist"),
            "earlier conversation"
        ),
        "after the pop the tail must be the earlier User text message"
    );
}

#[tokio::test]
async fn prompt_cancelled_after_tool_result_compacts_cleanly() {
    ensure_test_data_dir();

    // @step Given a sub-agent stream in which the CompactionHook cancels on the usage update immediately after a tool result
    // (Expressed as: the Path C state immediately after the CMPCT-029
    // cleanup — session tail is a complete tool pair.)

    // @step And session.messages ends with the matching Assistant(ToolCall) and User(ToolResult) pair
    let mut session = fresh_session();
    let call = make_tool_call(
        "toolu_660169",
        Some("call_66016913094b485a8b5b189d"),
        "deep_search",
    );
    session
        .messages
        .push(user_text_message("research the codebase"));
    session.messages.push(assistant_tool_call_message(call));
    session.messages.push(user_tool_result_message(
        "toolu_660169",
        Some("call_66016913094b485a8b5b189d"),
        "research findings",
    ));

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::from("partial analysis");

    // Run the Path C sequence exactly as stream_loop.rs does:
    // begin_compaction_recovery(..., true) then execute_compaction.
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("recovery must succeed");

    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // @step When the compaction-recovery path runs and execute_compaction builds the DAG instruction
    let result = execute_compaction(&mut session, compaction_in_progress, Some("Continue")).await;

    // @step Then no "Compaction failed" notification is emitted
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionFailed { .. })),
        0,
        "no CompactionFailed event may be emitted"
    );

    // @step And execute_compaction succeeds instead of refusing on the orphan guard
    assert!(
        result.is_ok(),
        "CMPCT-050: compaction after a tool-result-tail cancel must SUCCEED, \
         got: {result:?} — the trailing User(ToolResult) must not have been \
         popped (or the safety net must have closed the orphan)"
    );

    // @step And the post-compaction session is tool-pair-clean for the next turn
    // (In-view DAG compaction clears the compactable messages; the result is
    // folded into the DAG via SessionSearch. What must NOT happen is an
    // orphan leaking into the next turn's request.)
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "post-compaction session must be tool-pair-clean for the next turn"
    );
}

#[test]
fn safety_net_closes_orphans_after_all_mutations() {
    // @step Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    let mut session = fresh_session();
    let call = make_tool_call("toolu_orphan", Some("call_orphan_prior"), "bash");
    session.messages.push(user_text_message("prior prompt"));
    session.messages.push(assistant_tool_call_message(call));
    let orphans = validate_no_orphan_tool_calls(&session.messages).err();
    assert_eq!(
        orphans.as_ref().map(Vec::len),
        Some(1),
        "precondition: exactly one orphan tool_call must be present"
    );

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::new();

    // @step When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    // (Every entry path funnels through begin_compaction_recovery, so
    // running it with pop_user_prompt=true exercises the choke point all
    // of them share.)
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("helper must succeed");

    // @step Then the orphan call receives a synthetic User(ToolResult) marked "cancelled_by_context_limit"
    assert!(
        session
            .messages
            .iter()
            .any(|m| is_tool_result_with_call_id(m, "call_orphan_prior")),
        "a User(ToolResult) for call_orphan_prior must have been injected"
    );

    // @step And execute_compaction's orphan guard passes for that path
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "CMPCT-050: after begin_compaction_recovery the orphan must be closed \
         (synthetic cancelled tool_result injected) so execute_compaction's \
         guard passes on every entry path"
    );
}
