/// Scenario: get_session_tokens / get_session_model / get_compaction_progress
/// return safe defaults via the stub on both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn defaults_for_tokens_model_compaction_match_across_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step Given an engineer holds a StubSessionManagerHandle that has not been seeded with custom token state
    // @step When the engineer calls backend.get_session_tokens(sid).await over the embedded transport
    let em_tokens = embedded
        .get_session_tokens(sid.clone())
        .await
        .expect("em tokens");
    let ws_tokens = websocket
        .get_session_tokens(sid.clone())
        .await
        .expect("ws tokens");
    // @step Then the call returns Ok(SessionTokens { input_tokens: 0, output_tokens: 0 })
    assert_eq!(
        em_tokens,
        SessionTokens {
            input_tokens: 0,
            output_tokens: 0,
        }
    );
    assert_eq!(em_tokens, ws_tokens, "tokens must agree across transports");

    // @step When the engineer calls backend.get_session_model(sid).await
    let em_model = embedded
        .get_session_model(sid.clone())
        .await
        .expect("em model");
    let ws_model = websocket
        .get_session_model(sid.clone())
        .await
        .expect("ws model");
    // @step Then the call returns Ok(SessionModel { provider_id: "", model_id: "", context_window: 0, max_output_tokens: 0, compaction_threshold: 0 })
    assert_eq!(
        em_model,
        SessionModel {
            provider_id: String::new(),
            model_id: String::new(),
            context_window: 0,
            max_output_tokens: 0,
            compaction_threshold: 0,
        }
    );
    assert_eq!(em_model, ws_model, "model must agree across transports");

    // @step When the engineer calls backend.get_compaction_progress(sid).await
    let em_progress = embedded
        .get_compaction_progress(sid.clone())
        .await
        .expect("em progress");
    let ws_progress = websocket
        .get_compaction_progress(sid)
        .await
        .expect("ws progress");
    // @step Then the call returns Ok(None)
    // @step And the same three calls over WebSocketFspecBackend against a server hosting the SAME StubSessionManagerHandle return identical values
    assert!(em_progress.is_none());
    assert!(ws_progress.is_none());
}

/// Scenario (RPC-074, supersedes original RPC-037 contract): clear_history
/// emits a `SessionStateChange { state: Cleared }` chunk and returns Ok.
/// The TS reference (`src/tui/components/AgentView.tsx:1554-1564`,
/// handleClearCommand → TUI-066) drives the conversation reset off the
/// `Cleared` state chunk — NOT off any UserNotification carrying the
/// literal string "history cleared". The parity test subscribes to
/// chunks_rx on BOTH transports BEFORE calling clear_history and then
/// observes the `Cleared` chunk on each transport within 1 second.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn clear_history_returns_ok_on_both_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step Given an engineer subscribes to backend.chunks_rx() before calling clear_history
    let mut em_rx = embedded.chunks_rx();
    let mut ws_rx = websocket.chunks_rx();

    // @step When the engineer calls backend.clear_history(sid).await on either transport
    embedded
        .clear_history(sid.clone())
        .await
        .expect("em clear_history");
    // @step Then the call returns Ok(())
    // @step And within 1 second a StreamChunk::SessionStateChange chunk with state SessionState::Cleared for that session is observed on chunks_rx (RPC-074: TS parity with TUI-066 contract; previously this was a UserNotification chunk, retired as a Rust-side invention)
    assert!(
        observe_session_state_cleared(&mut em_rx, &sid).await,
        "embedded must observe SessionStateChange {{ state: Cleared }} chunk"
    );

    websocket
        .clear_history(sid.clone())
        .await
        .expect("ws clear_history");
    assert!(
        observe_session_state_cleared(&mut ws_rx, &sid).await,
        "websocket must observe SessionStateChange {{ state: Cleared }} chunk"
    );
}

/// Drain a chunks_rx receiver until a `SessionStateChange { state:
/// Cleared }` chunk for `sid` arrives, or the 1s budget is exhausted.
/// Returns true on success, false on timeout. Per RPC-074 this replaces
/// the previous `observe_user_notification_history_cleared` helper —
/// the `UserNotification("history cleared")` chunk was a TS-divergence
/// that has been removed from the stub.
async fn observe_session_state_cleared(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
) -> bool {
    for _ in 0..16 {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Ok((got, StreamChunk::SessionStateChange { state })))
                if got == *sid && matches!(state, codelet_rpc_types::SessionState::Cleared) =>
            {
                return true;
            }
            Ok(Ok(_)) => continue,
            _ => return false,
        }
    }
    false
}

/// Scenario: compact_session returns the canned CompactionResult on both
/// transports against the same stub AND emits a CompactionComplete chunk
/// on chunks_rx within 1 second per rule [5].
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compact_session_canned_result_matches_across_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step Given an engineer subscribes to backend.chunks_rx() before calling compact_session
    let mut em_rx = embedded.chunks_rx();
    let mut ws_rx = websocket.chunks_rx();

    // @step When the engineer calls backend.compact_session(sid).await on either transport
    let em_res = embedded
        .compact_session(sid.clone())
        .await
        .expect("em compact");
    let ws_res = websocket
        .compact_session(sid.clone())
        .await
        .expect("ws compact");
    // @step Then the call returns Ok(CompactionResult { compression_ratio: 50.0, original_tokens: 1000, compacted_tokens: 500, turns_summarized: 4, turns_kept: 2 })
    // CompactionResult has no PartialEq impl (f64 field); compare fields.
    assert!((em_res.compression_ratio - ws_res.compression_ratio).abs() < f64::EPSILON);
    assert_eq!(em_res.original_tokens, ws_res.original_tokens);
    assert_eq!(em_res.compacted_tokens, ws_res.compacted_tokens);
    assert_eq!(em_res.turns_summarized, ws_res.turns_summarized);
    assert_eq!(em_res.turns_kept, ws_res.turns_kept);
    // RPC-420: the canned value is 50.0 — percent of tokens removed.
    assert!((em_res.compression_ratio - 50.0).abs() < f64::EPSILON);
    assert_eq!(em_res.original_tokens, 1000);
    assert_eq!(em_res.compacted_tokens, 500);
    // @step And within 1 second a StreamChunk::CompactionComplete arrives on chunks_rx for that session carrying the same CompactionResult
    let em_chunk_result = observe_compaction_complete(&mut em_rx, &sid).await;
    let ws_chunk_result = observe_compaction_complete(&mut ws_rx, &sid).await;
    assert!(
        em_chunk_result.is_some(),
        "embedded must observe CompactionComplete chunk"
    );
    assert!(
        ws_chunk_result.is_some(),
        "websocket must observe CompactionComplete chunk"
    );
    let (em_cr, ws_cr) = (em_chunk_result.unwrap(), ws_chunk_result.unwrap());
    assert_eq!(em_cr.original_tokens, em_res.original_tokens);
    assert_eq!(em_cr.compacted_tokens, em_res.compacted_tokens);
    assert_eq!(em_cr.turns_summarized, em_res.turns_summarized);
    assert_eq!(em_cr.turns_kept, em_res.turns_kept);
    assert!((em_cr.compression_ratio - em_res.compression_ratio).abs() < f64::EPSILON);
    assert_eq!(ws_cr.original_tokens, em_cr.original_tokens);
    assert_eq!(ws_cr.compacted_tokens, em_cr.compacted_tokens);
}

/// Drain a chunks_rx receiver looking for a CompactionComplete chunk
/// for `sid`. Returns the inner CompactionResult on success.
async fn observe_compaction_complete(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
) -> Option<codelet_rpc_types::CompactionResult> {
    for _ in 0..16 {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Ok((got, StreamChunk::CompactionComplete { compaction_result }))) if got == *sid => {
                return Some(compaction_result);
            }
            Ok(Ok(_)) => continue,
            _ => return None,
        }
    }
    None
}
