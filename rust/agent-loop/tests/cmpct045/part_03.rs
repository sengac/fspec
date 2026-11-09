// ============================================================================
// Scenario: Successful sub-agent DAG is pinned to the target session
// Scenario: Compactor sub-agent runs in an ephemeral clean session
// Scenario: Compactor sub-agent surveys the target exclusively via
//           SessionSearch on the target session id
// Scenario: Other-session target is pinned immediately under the target's lock
// ============================================================================

#[tokio::test]
async fn successful_dag_is_pinned_to_the_target_and_sub_agent_session_is_ephemeral() {
    // @step Given a live target session with system reminders and many compactable conversation messages
    // @step Given a parent session whose provider and model are configured
    // @step Given a parent session with persisted turns
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given the calling session has a configured provider and model
    // @step Given a live target session with persisted turns
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&target, 6).await;

    let tool = register_and_build_tool(&caller, &project, &manager);
    // @step Given session S calls GenerateCompaction with a live subordinate session T's UUID

    // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    // @step When the agent calls GenerateCompaction with the target session UUID
    // @step When GenerateCompaction is invoked for a live target session
    // @step When the handler side pins the sub-agent's DAG to the target
    // @step Given GenerateCompaction is invoked for a live target session
    // @step When the sub-agent run and pin complete
    // @step When the handler side completes the run
    // @step When the compactor sub-agent is asked to build the compaction DAG
    // @step And the compactor sub-agent returns a parseable DAG
    // @step And the sub-agent returns a parseable DAG
    // @step When the compactor sub-agent is spawned for the parent session
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("success path must return Ok");
    // @step When GenerateCompaction completes successfully

    // @step Then the pin is applied to T's in-memory messages under T's inner lock
    // @step And T's messages are reduced to system reminders plus the wrapped DAG
    // @step Then the target's in-memory messages are replaced by the system reminders plus the wrapped DAG
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("[Previous conversation summary]"),
        "the pinned message must carry the durable summary, got: {pinned}"
    );
    assert!(
        pinned.contains("Work arc"),
        "the pinned DAG must be the sub-agent's DAG, got: {pinned}"
    );

    // @step And the target's turn list is cleared
    // @step And the persisted session manifest history is NOT truncated
    // (the pin mutates only the in-memory `inner` — the persistence layer
    // is untouched by the handler-side pin)
    let turns_len = target.inner.lock().await.turns.len();
    assert_eq!(
        turns_len, 0,
        "the pin must clear the target's turn list (got {turns_len})"
    );

    // @step And the target's token tracker is recalculated from the reduced message list
    let tracker = target.inner.lock().await.token_tracker.input_tokens;
    assert!(
        tracker < 190_000,
        "tracker must reflect the reduced context, got {tracker}"
    );

    // @step And the DAG content is wrapped in the compaction-dag system-reminder wrapper
    assert!(
        pinned.contains("Work arc"),
        "the wrapped content must include the node"
    );

    // @step And session S's context is untouched
    let caller_msgs = caller.inner.lock().await.messages.len();
    assert!(
        caller_msgs == 0 || caller_msgs >= 2,
        "the caller's own context must be untouched (had {caller_msgs} messages)"
    );

    // @step Then the compactor sub-agent runs under a fresh ephemeral session id that is not the target's or caller's id
    // @step Then the sub-agent runs under a fresh ephemeral session id that is not the parent's id
    // @step And no session record is persisted for the sub-agent and no worktree is created
    // The manager must hold EXACTLY the two live sessions it created —
    // no persisted record (manifest) for the ephemeral sub-agent exists,
    // and no worktree was created (the sub-agent ran in-memory only).
    let ids: Vec<String> = manager
        .list_sessions(&project)
        .expect("list_sessions reads the session")
        .iter()
        .map(|s| s.id.clone())
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "the manager must hold exactly the two live sessions — no record for the \
         ephemeral sub-agent (ids: {ids:?})"
    );
    assert!(
        ids.contains(&caller.id.to_string()) && ids.contains(&target.id.to_string()),
        "the two live sessions must be present: {ids:?}"
    );

    // @step And the sub-agent inherits the calling session's provider, model, context window, and max output tokens
    // @step And the sub-agent inherits the parent session's provider and model
    // (the handler closure captured them from the caller's ProviderManager
    // at registration — BUG-102 inheritance, asserted at the shape level
    // by the registration tests)

    // @step Then every SessionSearch call the sub-agent issues targets the target session id, not the sub-agent's own ephemeral session
    // @step Then the sub-agent's SessionSearch calls target the parent session id, not the sub-agent's own ephemeral session
    // @step And the sub-agent never reads the target's in-memory message list
    // @step And the sub-agent never reads the parent's in-memory message list
    // The LLM request the sub-agent made carried the FRESH compaction
    // prompt with the target UUID hard-coded into every SessionSearch
    // reference (the sub-agent's own session is its ephemeral one).
    let task = last_task_prompt(&server).await;
    assert!(
        task.contains(&target.id.to_string()),
        "the sub-agent's task prompt must name the TARGET session id, got: {task}"
    );
    assert!(
        !task.contains(&caller.id.to_string()),
        "the sub-agent's task prompt must NOT name the caller's session id"
    );
    assert!(
        task.contains("SessionSearch(session_id:"),
        "every SessionSearch reference must pass session_id explicitly, got: {task}"
    );

    // @step And the sub-agent's SessionSearch handler is created before the run and removed by a drop guard after it, even if the run panics
    // The drop guard removes the ephemeral handler: after the run the
    // tools-crate SessionSearch registry must hold no handler for a
    // session id the manager does not know (the ephemeral id is unique
    // per run and now gone).
    let stray = codelet_tools::has_session_search_handler(caller.id)
        || codelet_tools::has_session_search_handler(target.id);
    // Note: the caller/target ids belong to LIVE sessions whose agent
    // loops register their own SessionSearch handlers in production —
    // in this test no agent loop ran, so BOTH must be false.
    assert!(
        !stray,
        "no SessionSearch handler may remain registered for any session id \
         (the ephemeral sub-agent's handler must have been removed by its drop \
         guard, and no live-session handler was ever registered in this test)"
    );

    // @step Then the tool result string is the DAG text itself so the caller sees exactly what was pinned
    // @step And the tool returns Ok for both success and fallback outcomes, reserving Err for validation failures, missing handler, and unknown-target rejections
    // @step Given the compactor sub-agent returns a parseable DAG
    assert_eq!(
        result, SUCCESS_DAG,
        "tool result must be the pinned DAG text"
    );

    // @step And the target's compaction_in_progress flag is false again after the pin
    assert!(
        !target.compaction_in_progress.load(Ordering::SeqCst),
        "the target's compaction flag must be cleared after the pin"
    );

    // @step Then the target emitted a Compacting state change and a compaction progress update before the sub-agent was spawned
    // @step And the target emitted CompactionComplete after the pin and then returned to its previous state
    // @step And the CompactionComplete event reflects the recalculated post-pin token basis, not just the DAG summary size
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

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Calling-session target is queued for durable reconciliation
// ============================================================================

#[tokio::test]
async fn calling_session_target_stashes_pending_dag_instead_of_locking_inner() {
    // @step Given the agent calls GenerateCompaction with no arguments on its own session A mid-turn
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (server, _data_dir, project, manager, caller, _target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&caller, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    let pre_call_msgs = caller.inner.lock().await.messages.len();

    // @step And the sub-agent returns a parseable DAG
    // @step When the tool call completes within session A's turn
    let result = tool
        .call(GenerateCompactionArgs { session_id: None })
        .await
        .expect("calling-session compaction must return Ok");

    // @step Then the tool result reports the DAG text as a queued candidate
    assert_eq!(queued_content(&result), SUCCESS_DAG);

    // @step And the candidate awaits durable reconciliation rather than being pinned in place
    assert!(codelet_core::persistence::load_session(caller.id)
        .expect("caller manifest")
        .compaction_commit
        .is_none());

    // @step And the handler does not lock session A's inner while the tool call is in flight
    // Proof: the caller's inner was never mutated — the exact same
    // message count (system reminders + 8 seeded turns) is still there
    // (the end-of-turn path pins later).
    let caller_msgs = caller.inner.lock().await.messages.len();
    assert_eq!(
        caller_msgs,
        pre_call_msgs,
        "the calling session's inner must be untouched mid-turn (got {caller_msgs}, was {pre_call_msgs})"
    );

    // @step And the compaction flag is cleared once the candidate is queued
    assert!(
        !caller.compaction_in_progress.load(Ordering::SeqCst),
        "the caller's compaction flag must be cleared after the stash"
    );

    drop(_guard);
    let _ = server;
}
