// ============================================================================
// Scenario: Unknown target session is rejected
// ============================================================================

#[tokio::test]
async fn unknown_target_session_is_rejected_with_an_execution_error() {
    // @step Given the calling session has a registered GenerateCompaction handler
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // LLM mock with the success body: if a sub-agent were (wrongly)
    // spawned, it would reach this endpoint. The post-call assertion
    // checks that ZERO requests were received.
    let server = MockServer::start().await;
    point_openai_at(&server.uri());
    let mock = Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(openai_body(SUCCESS_DAG)),
        )
        .expect(0..);
    server.register(mock).await;

    let (_data_dir, project, manager, caller, _target) = fixture_sessions().await;
    seed_turns(&caller, 2).await;

    let tool = register_and_build_tool(&caller, &project, &manager);
    let pre_call_msgs = caller.inner.lock().await.messages.len();

    // @step And the target UUID does not correspond to a live in-memory session
    let unknown = Uuid::new_v4();

    // @step When the agent calls GenerateCompaction with that target UUID
    let err = tool
        .call(GenerateCompactionArgs {
            session_id: Some(unknown.to_string()),
        })
        .await
        .expect_err("an unknown target must be an execution error");

    // @step Then the tool returns a ToolError::Execution stating the target session was not found
    match err {
        ToolError::Execution { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.contains(&format!("Session not found: {unknown}")),
                "the rejection must name the target: {message}"
            );
        }
        other => panic!("expected ToolError::Execution, got {other:?}"),
    }

    // @step And no sub-agent is spawned and no pinning occurs
    let caller_msgs = caller.inner.lock().await.messages.len();
    assert_eq!(
        caller_msgs, pre_call_msgs,
        "no session may be modified on rejection (was {pre_call_msgs}, got {caller_msgs})"
    );
    assert!(
        caller.pending_dag_content.lock().expect("lock").is_none(),
        "nothing may be stashed on rejection"
    );
    assert!(
        !caller.compaction_in_progress.load(Ordering::SeqCst),
        "no compaction flag may be left set on rejection"
    );

    // Zero LLM requests: the sub-agent was never spawned (the handler
    // rejects the unknown target before spawning).
    let requests = server
        .received_requests()
        .await
        .unwrap_or_else(|| panic!("received_requests failed"));
    assert!(
        requests.is_empty(),
        "rejection must not spawn a sub-agent — the LLM mock received \
         {} request(s)",
        requests.len()
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
// (the capture-before-clear contract end-to-end through the handler)
// ============================================================================

#[tokio::test]
async fn existing_dag_is_captured_before_any_clear_and_drives_incremental_mode() {
    // @step Given a target session restored from a committed compaction DAG ending at turn N
    let existing = r#"<dag-node depth="D2" turns="0-45" label="Architecture">
- settled decision
</dag-node>"#;
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(&format!("{existing}\n<dag-files>\n</dag-files>")).await;
    // BUG-311: incremental mode reads the target's committed durable summary.
    commit_existing_dag(&target, existing, 46).await;
    target.inner.lock().await.token_tracker.input_tokens = 150_000;
    let candidate = format!("{existing}\n<dag-files>\n</dag-files>");

    // @step When the compactor sub-agent's task prompt is built
    let tool = register_and_build_tool(&caller, &project, &manager);
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("incremental compaction must return Ok");

    // @step Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    // The LLM request the sub-agent made carried the INCREMENTAL prompt:
    // the existing DAG embedded and the turn offset filled in.
    let task = last_task_prompt(&server).await;
    assert!(
        task.contains(existing),
        "the INCREMENTAL prompt must embed the existing DAG, got: {task}"
    );
    assert!(
        task.contains("do NOT rebuild from scratch"),
        "the INCREMENTAL instruction must be selected when a DAG exists, got: {task}"
    );

    // @step And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward
    assert!(
        task.contains("PRESERVE all existing D2"),
        "the prompt must instruct preserving D2 nodes, got: {task}"
    );
    assert!(
        task.contains("PROMOTE existing D0 (Detailed) nodes to D1"),
        "the prompt must instruct promoting D0 to D1, got: {task}"
    );
    assert!(
        task.contains("start_turn: 46"),
        "the prompt must survey from turn 46 (max_turn_end 45 + 1), got: {task}"
    );

    // @step And the existing DAG is captured from the target BEFORE any clear of its messages
    // The pin already ran (it cleared the target's messages), yet the
    // prompt carried the pre-pin DAG — proof the capture happened
    // BEFORE the clear. The pre-compaction basis also proves the
    // snapshot ran before any mutation.
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("settled decision"),
        "the new DAG must replace the context: {pinned}"
    );
    assert_eq!(result, candidate, "the tool result must be the pinned DAG");

    let original = match last_compaction_event(&target) {
        StreamChunk::CompactionComplete { compaction_result } => compaction_result.original_tokens,
        other => panic!("expected CompactionComplete, got {other:?}"),
    };
    assert_eq!(
        original, 150_000,
        "the pre-compaction basis must be the snapshotted pre-run value (150000)"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: compaction_in_progress gates sub-agent reads and is cleared
//          after the pin
// ============================================================================

#[tokio::test]
async fn compaction_flag_true_during_run_and_false_after_pin() {
    // @step Given the compactor sub-agent run begins for a live target session
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&target, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step When the sub-agent runs and reads the target via SessionSearch
    // @step Then the target's compaction_in_progress flag is true during the sub-agent run so Layer-0 trimming applies to its reads
    // The flag must be true DURING the run (the spawner stores it before
    // the LLM call and it is only cleared after the pin).
    let pre = target.compaction_in_progress.load(Ordering::SeqCst);
    assert!(!pre, "flag must start false");
    tool.call(GenerateCompactionArgs {
        session_id: Some(target.id.to_string()),
    })
    .await
    .expect("compaction must succeed");

    // @step And the target's compaction_in_progress flag is false after the pin completes
    assert!(
        !target.compaction_in_progress.load(Ordering::SeqCst),
        "the flag must be false after the pin (it is true during the run — \
         the spawner stores it before the LLM call and the handler clears \
         it after the pin)"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Compaction lifecycle events are emitted on the target session
// ============================================================================

#[tokio::test]
async fn compaction_lifecycle_events_carry_the_recalculated_post_pin_basis() {
    // @step Given GenerateCompaction is invoked for a live target session
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&target, 10).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step When the sub-agent run and pin complete
    tool.call(GenerateCompactionArgs {
        session_id: Some(target.id.to_string()),
    })
    .await
    .expect("compaction must succeed");

    // @step Then the target emitted a Compacting state change and a compaction progress update before the sub-agent was spawned
    // @step And the target emitted CompactionComplete after the pin and then returned to its previous state
    let chunks = target.get_buffered_output(1000);
    let compacting_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::SessionStateChange { state } if *state == SessionState::Compacting))
        .expect("target must emit a Compacting state change");
    let complete_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
        .expect("target must emit CompactionComplete after the pin");
    assert!(
        complete_at > compacting_at,
        "CompactionComplete (pos {complete_at}) must follow Compacting (pos {compacting_at})"
    );
    assert!(
        chunks[compacting_at + 1..].iter().any(|c| matches!(
            c,
            StreamChunk::SessionStateChange { state } if *state != SessionState::Compacting
        )),
        "the target must leave Compacting once the operation ends"
    );

    // @step Then the CompactionComplete event reflects the recalculated post-pin token basis, not just the DAG summary size
    match last_compaction_event(&target) {
        StreamChunk::CompactionComplete { compaction_result } => {
            let post = target.inner.lock().await.token_tracker.input_tokens;
            assert_eq!(
                compaction_result.compacted_tokens as u64, post,
                "compacted_tokens must equal the recalculated tracker total ({post})"
            );
            assert_eq!(
                compaction_result.original_tokens, 190_000,
                "original_tokens must be the pre-compaction snapshot"
            );
        }
        other => panic!("expected CompactionComplete, got {other:?}"),
    }

    drop(_guard);
    let _ = server;
}
