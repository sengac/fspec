// ============================================================================
// Scenario: Self-target capture never awaits the caller's inner lock
// (CMPCT-046 deadlock guard — the crucial regression: the OLD code
// `target_bg.inner.lock().await` here deadlocks when the lock is held for
// the whole turn, so this test would HANG on pre-fix code. The fix
// degrades the capture to lock-free signals.)
// ============================================================================

#[tokio::test]
async fn self_target_capture_never_awaits_the_callers_inner_lock() {
    // @step Given the agent calls GenerateCompaction with no arguments on its own session A mid-turn
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (server, _data_dir, project, manager, caller, _target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&caller, 4).await;
    // Seed a pre-existing wrapped DAG into the caller's context so the
    // capture would find it if it could read inner (the lock-free FRESH
    // degradation must NOT see it).
    seed_existing_dag(&caller, "Pre-seeded architecture DAG", 0).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step And session A's inner lock is held by the calling agent loop for the whole turn
    let _inner_held = caller
        .inner
        .try_lock()
        .expect("test can hold the caller's inner lock (simulating the agent loop's turn lock)");

    // @step When the handler captures the existing DAG and the pre-compaction token basis
    // The tool call must COMPLETE — on pre-fix code it awaits the caller's
    // inner lock forever (self-deadlock) and this test hangs.
    let result = codelet_test_helpers::deadlock_guard(
        "tool.call(GenerateCompactionArgs { session_id: None })",
        tool.call(GenerateCompactionArgs { session_id: None }),
    )
    .await
    .expect("self-target compaction must NOT self-deadlock (lock held for the whole turn)")
    .expect("calling-session compaction must return Ok");
    drop(_inner_held);

    // @step Then the capture does not await session A's inner lock
    // (proof: the call completed while the lock was held for the whole run —
    // a deadlock would have timed out above)
    assert_eq!(
        queued_content(&result),
        SUCCESS_DAG,
        "tool result must carry the queued DAG text"
    );

    // @step And the capture degrades to the lock-free cached token basis, a FRESH rebuild, and the completed-turn count
    // FRESH rebuild: the sub-agent's task prompt must NOT embed the
    // pre-seeded existing DAG (the lock-free capture could not read it).
    let task = last_task_prompt(&server).await;
    assert!(
        !task.contains("Pre-seeded architecture DAG"),
        "the lock-free self-target capture must degrade to FRESH — the \
         pre-seeded existing DAG must NOT be embedded in the prompt, got: {task}"
    );
    assert!(
        task.contains("Build a hierarchical summary DAG"),
        "the FRESH compaction instruction must be selected (no readable DAG state), got: {task}"
    );

    // @step And the sub-agent still runs and the DAG is queued as a candidate
    // (asserted above: the tool result carries the queued DAG)

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: A zero tracker basis falls back to a content estimate
// (CMPCT-046b honest zero-basis: the provider did not report usage, so the
// tracker's input_tokens is 0 for the whole session — the pre-compaction
// basis must be the per-message content estimate, never 0)
// ============================================================================

/// Seed `n` compactable conversation turns but leave the token tracker at 0
/// (simulating a provider that never reported usage — e.g. OpenAI
/// streaming without `include_usage`).
async fn seed_turns_no_usage(session: &BackgroundSession, n: usize) {
    let mut inner = session.inner.lock().await;
    install_selected(&mut inner, session.id).await;
    // The selected-preparer install reports usage; the seeded turns do not.
    inner.token_tracker.input_tokens = 0;
    for i in 0..n {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("user turn {i} do work"))),
        });
        inner.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
                text: format!("assistant turn {i} did work"),
            })),
        });
        inner.turns.push(ConversationTurn {
            user_message: format!("user turn {i}"),
            tool_calls: vec![],
            tool_results: vec![],
            assistant_response: format!("assistant turn {i}"),
            tokens: 500,
            timestamp: std::time::SystemTime::now(),
            previous_error: None,
        });
    }
    persist_covered_turns(session);
    // Tracker deliberately left at 0 — the provider reported no usage.
    assert_eq!(
        inner.token_tracker.input_tokens, 0,
        "the fixture must leave the tracker at 0 (provider reported no usage)"
    );
}

#[tokio::test]
async fn a_zero_tracker_basis_falls_back_to_a_content_estimate() {
    // @step Given a live target session whose token tracker reads 0 tokens because the provider did not report usage
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns_no_usage(&target, 10).await;
    let tracker = target.inner.lock().await.token_tracker.input_tokens;
    assert_eq!(tracker, 0, "precondition: the target's tracker must read 0");

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step And the target has compactable conversation messages
    let msg_count = target.inner.lock().await.messages.len();
    assert!(
        msg_count >= 20,
        "the target must hold compactable messages (got {msg_count})"
    );

    // @step When GenerateCompaction completes and pins a DAG to the target
    tool.call(GenerateCompactionArgs {
        session_id: Some(target.id.to_string()),
    })
    .await
    .expect("zero-basis compaction must still pin — Ok");

    // @step Then the pre-compaction token basis is a content estimate greater than 0
    // The shared primitive behind the fallback (CMPCT-046b) must agree with
    // the event: estimating from the pre-pin message list yields a > 0
    // basis on the same count_tokens accounting.
    {
        let msgs = target.inner.lock().await.messages.clone();
        let est = codelet_cli::interactive_helpers::estimate_message_tokens(&msgs);
        assert!(
            est > 0,
            "the content estimate must be > 0 for compactable messages"
        );
        let basis = codelet_cli::interactive_helpers::pre_compaction_basis(0, &msgs);
        assert_eq!(
            basis, est,
            "pre_compaction_basis(0, msgs) must be the content estimate"
        );
        let basis_tracker = codelet_cli::interactive_helpers::pre_compaction_basis(500, &msgs);
        assert_eq!(
            basis_tracker, 500,
            "pre_compaction_basis must prefer the tracker when it reported usage"
        );
    }
    let original = match last_compaction_event(&target) {
        StreamChunk::CompactionComplete { compaction_result } => compaction_result.original_tokens,
        other => panic!("expected CompactionComplete, got {other:?}"),
    };
    assert!(
        original > 0,
        "the pre-compaction basis must be the content estimate (> 0), not the dead 0 tracker value"
    );

    // @step And the CompactionComplete event's original_tokens is never 0
    assert!(
        original > 0,
        "a 0 original_tokens must never reach CompactionComplete for a target with compactable messages"
    );

    drop(_guard);
    let _ = server;
}
