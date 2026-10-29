/// Scenario: debug capture toggle is wired through both transports
/// AND emits a DebugStateChange chunk on chunks_rx per rule [5] / the
/// feature's "And a StreamChunk::DebugStateChange chunk is observed"
/// Then-step.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn debug_toggle_round_trips_across_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds a freshly-created session id and subscribes to chunks_rx
    let sid = embedded.create_session(None).await.expect("create_session");
    let mut em_rx = embedded.chunks_rx();
    let mut ws_rx = websocket.chunks_rx();

    // @step When the engineer calls backend.get_debug_enabled(sid).await
    // @step Then the call returns Ok(false)
    assert!(!embedded
        .get_debug_enabled(sid.clone())
        .await
        .expect("em get debug"));
    // @step When the engineer calls backend.set_debug_enabled(sid, true).await
    embedded
        .set_debug_enabled(sid.clone(), true)
        .await
        .expect("em set debug true");
    // @step Then the call returns Ok(())
    // @step When the engineer calls backend.get_debug_enabled(sid).await
    // @step Then the call returns Ok(true)
    assert!(websocket
        .get_debug_enabled(sid.clone())
        .await
        .expect("ws get debug"));

    // @step When the engineer calls backend.toggle_debug(sid, "/tmp/debug").await
    let em_path = embedded
        .toggle_debug(sid.clone(), "/tmp/debug-em".to_string())
        .await
        .expect("em toggle_debug");
    // @step Then the call returns Ok(<some path string>) and a StreamChunk::DebugStateChange chunk is observed on chunks_rx for that session
    assert!(!em_path.is_empty());
    assert!(
        observe_debug_state_change(&mut em_rx, &sid).await,
        "embedded must observe DebugStateChange chunk after toggle_debug"
    );

    let ws_path = websocket
        .toggle_debug(sid.clone(), "/tmp/debug-ws".to_string())
        .await
        .expect("ws toggle_debug");
    assert!(!ws_path.is_empty());
    assert!(
        observe_debug_state_change(&mut ws_rx, &sid).await,
        "websocket must observe DebugStateChange chunk after toggle_debug"
    );
}

/// Drain a chunks_rx receiver until a DebugStateChange chunk for `sid`
/// arrives, or the 1s budget is exhausted.
async fn observe_debug_state_change(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
) -> bool {
    for _ in 0..16 {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Ok((got, StreamChunk::DebugStateChange { .. }))) if got == *sid => {
                return true;
            }
            Ok(Ok(_)) => continue,
            _ => return false,
        }
    }
    false
}

/// Scenario: pause_confirm / pause_triple / pause_resume update pause state
/// AND emit a `SessionStateChange { state: SessionState::Running }` chunk
/// on chunks_rx per rule [5] / the feature's "And a StreamChunk::SessionStateChange"
/// Then-step.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pause_state_round_trips_across_transports() {
    use codelet_rpc_types::SessionState;
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer seeds the StubSessionManagerHandle with a PauseState { kind: PauseKind::Confirm, prompt: "Apply?", tool_call_id: None } for sid and subscribes to chunks_rx
    let sid = embedded.create_session(None).await.expect("create_session");
    let mut em_rx = embedded.chunks_rx();
    let mut ws_rx = websocket.chunks_rx();

    stub.seed_pause_state(
        sid.clone(),
        PauseState {
            kind: PauseKind::Confirm,
            prompt: "Apply?".into(),
            tool_call_id: None,
        },
    );

    // @step When the engineer calls backend.get_pause_state(sid).await
    let em_ps = embedded
        .get_pause_state(sid.clone())
        .await
        .expect("em get pause");
    let ws_ps = websocket
        .get_pause_state(sid.clone())
        .await
        .expect("ws get pause");
    // @step Then the call returns Ok(Some(PauseState { kind: PauseKind::Confirm, prompt: "Apply?", tool_call_id: None }))
    assert_eq!(em_ps, ws_ps);
    assert!(em_ps.is_some());

    // @step When the engineer calls backend.pause_confirm(sid, true).await
    embedded
        .pause_confirm(sid.clone(), true)
        .await
        .expect("em pause_confirm");
    // @step Then the call returns Ok(())
    // @step And a StreamChunk::SessionStateChange { state: SessionState::Running } arrives on chunks_rx for sid within 1 second
    assert!(
        observe_session_state_change_running(&mut em_rx, &sid).await,
        "embedded must observe SessionStateChange(Running) after pause_confirm"
    );
    assert!(
        observe_session_state_change_running(&mut ws_rx, &sid).await,
        "websocket must observe SessionStateChange(Running) after pause_confirm"
    );
    // @step And backend.get_pause_state(sid).await returns Ok(None)
    assert_eq!(
        websocket
            .get_pause_state(sid.clone())
            .await
            .expect("ws get pause after confirm"),
        None
    );

    // @step When the engineer seeds a PauseKind::Triple pause and calls backend.pause_triple(sid, ApprovalChoice::Approve).await
    stub.seed_pause_state(
        sid.clone(),
        PauseState {
            kind: PauseKind::Triple,
            prompt: "Apply?".into(),
            tool_call_id: None,
        },
    );
    websocket
        .pause_triple(sid.clone(), ApprovalChoice::Approve)
        .await
        .expect("ws pause_triple");
    // @step Then the call returns Ok(()) and backend.get_pause_state(sid).await returns Ok(None)
    assert!(
        observe_session_state_change_running(&mut em_rx, &sid).await,
        "embedded must observe SessionStateChange(Running) after pause_triple"
    );
    assert!(
        observe_session_state_change_running(&mut ws_rx, &sid).await,
        "websocket must observe SessionStateChange(Running) after pause_triple"
    );
    assert_eq!(
        embedded
            .get_pause_state(sid.clone())
            .await
            .expect("em get pause after triple"),
        None
    );

    // @step When the engineer seeds another pause and calls backend.pause_resume(sid).await
    stub.seed_pause_state(
        sid.clone(),
        PauseState {
            kind: PauseKind::Confirm,
            prompt: "Apply?".into(),
            tool_call_id: None,
        },
    );
    embedded
        .pause_resume(sid.clone())
        .await
        .expect("em pause_resume");
    // @step Then the call returns Ok(()) and backend.get_pause_state(sid).await returns Ok(None)
    assert!(
        observe_session_state_change_running(&mut em_rx, &sid).await,
        "embedded must observe SessionStateChange(Running) after pause_resume"
    );
    assert!(
        observe_session_state_change_running(&mut ws_rx, &sid).await,
        "websocket must observe SessionStateChange(Running) after pause_resume"
    );
    assert_eq!(
        websocket
            .get_pause_state(sid)
            .await
            .expect("ws get pause after resume"),
        None
    );

    // Silence unused-import warning when SessionState isn't otherwise
    // referenced; the match arm above pulls it in for clarity.
    let _ = SessionState::Running;
}

/// Drain a chunks_rx receiver until a `SessionStateChange { state:
/// SessionState::Running }` chunk for `sid` arrives, or the 1s budget
/// is exhausted.
async fn observe_session_state_change_running(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
) -> bool {
    use codelet_rpc_types::SessionState;
    for _ in 0..16 {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Ok((got, StreamChunk::SessionStateChange { state })))
                if got == *sid && matches!(state, SessionState::Running) =>
            {
                return true;
            }
            Ok(Ok(_)) => continue,
            _ => return false,
        }
    }
    false
}
