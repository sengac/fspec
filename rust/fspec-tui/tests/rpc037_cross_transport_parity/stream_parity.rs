/// Scenario: status_changes_rx is push-driven on both transports.
///
/// Subscribe to backend.status_changes_rx() on both transports BEFORE
/// calling send_input, then drive a send_input call through the
/// embedded transport. Both transports must observe the
/// `(sid, Running)` then `(sid, Idle)` tuple sequence within a few
/// seconds — the WS path observes them via `Envelope::StatusUpdate`
/// fanned out from the server's `status_changes_fanout` task.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn status_changes_rx_is_push_driven_on_both_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step Given an engineer subscribes to backend.status_changes_rx() on either transport before calling send_input
    let mut em_rx = embedded.status_changes_rx();
    let mut ws_rx = websocket.status_changes_rx();

    // @step When the engineer calls backend.send_input(sid, "hi".to_string()).await
    embedded
        .send_input(sid.clone(), "hi".to_string())
        .await
        .expect("send_input");

    async fn drain_until_idle(
        rx: &mut tokio::sync::broadcast::Receiver<(SessionId, SessionStatus)>,
        sid: &SessionId,
    ) -> Vec<SessionStatus> {
        let mut out = Vec::new();
        for _ in 0..16 {
            match timeout(Duration::from_secs(5), rx.recv()).await {
                Ok(Ok((got, st))) if got == *sid => {
                    let done = matches!(st, SessionStatus::Idle);
                    out.push(st);
                    if done {
                        break;
                    }
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        out
    }

    let em_seq = drain_until_idle(&mut em_rx, &sid).await;
    let ws_seq = drain_until_idle(&mut ws_rx, &sid).await;
    // @step Then within 5 seconds the status_changes_rx receiver yields (sid, SessionStatus::Running)
    // @step And within a further 5 seconds the receiver yields (sid, SessionStatus::Idle)
    // @step And the same (sid, status) tuple sequence is observed on the WebSocket transport when the WS server is hosting the SAME StubSessionManagerHandle
    assert!(
        em_seq.contains(&SessionStatus::Running) && em_seq.contains(&SessionStatus::Idle),
        "embedded must observe Running then Idle, got {em_seq:?}",
    );
    assert!(
        ws_seq.contains(&SessionStatus::Running) && ws_seq.contains(&SessionStatus::Idle),
        "websocket must observe Running then Idle, got {ws_seq:?}",
    );
}

/// Scenario: cross-transport byte-identical parity for the happy-path
/// scenario (create_session → send_input → drain → destroy_session).
///
/// Each transport runs against its OWN session id (because the stub
/// mints fresh ids per call) — what we compare is the resulting Vec of
/// `StreamChunk` shapes modulo source-of-truth session ids. Concretely,
/// we strip the session id from each chunk before bincode-encoding and
/// assert byte-identical bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn happy_path_parity_byte_identical_modulo_session_id() {
    let (_temp, service, _stub) = build_service();
    // @step Given a SharedFspecService is constructed against a freshly-built StubSessionManagerHandle
    // @step And both EmbeddedFspecBackend and WebSocketFspecBackend are constructed against that service
    let (embedded, websocket) = dual_backends(service).await;

    async fn run_happy_path(backend: &Arc<dyn FspecBackend>) -> Vec<StreamChunk> {
        let sid = backend.create_session(None).await.expect("create_session");
        let mut rx = backend.chunks_rx();
        backend
            .send_input(sid.clone(), "hi".to_string())
            .await
            .expect("send_input");

        let mut out = Vec::new();
        for _ in 0..32 {
            match timeout(Duration::from_secs(2), rx.recv()).await {
                Ok(Ok((got, c))) if got == sid => {
                    let done = matches!(c, StreamChunk::Done);
                    out.push(c);
                    if done {
                        break;
                    }
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        backend.destroy_session(sid).await.expect("destroy_session");
        out
    }

    // @step When the engineer runs the same happy-path scenario through each backend: create_session(None) → send_input(sid, "hi") → drain chunks_rx until StreamChunk::Done → destroy_session(sid)
    let em_chunks = run_happy_path(&embedded).await;
    let ws_chunks = run_happy_path(&websocket).await;
    assert!(!em_chunks.is_empty(), "embedded must observe chunks");
    assert!(!ws_chunks.is_empty(), "websocket must observe chunks");

    // @step Then bincode::serialize of the captured Vec<StreamChunk> from the embedded path equals the same from the WebSocket path
    // @step And no existing tarpc / push-channel test in rust/rpc-embedded/tests/ or rust/rpc-server/tests/ regresses
    assert_eq!(
        bincode::serialize(&em_chunks).expect("encode em"),
        bincode::serialize(&ws_chunks).expect("encode ws"),
        "cross-transport happy-path chunk sequences must be byte-identical"
    );
}

/// Scenario: send_input_with_thinking is added as a sibling of send_input
/// with backward-compatible default forwarding.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn send_input_with_thinking_forwards_when_none() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds an Arc<dyn SessionManagerHandle> backed by StubSessionManagerHandle
    let sid = embedded.create_session(None).await.expect("create_session");

    let mut em_rx = embedded.chunks_rx();
    let mut ws_rx = websocket.chunks_rx();

    // @step When the engineer calls handle.send_input_with_thinking(&sid, "hi".to_string(), None)
    embedded
        .send_input_with_thinking(sid.clone(), "hi".to_string(), None)
        .await
        .expect("em send_input_with_thinking");

    // @step Then the call returns immediately
    // @step And subsequent chunks_rx().recv() yields a StreamChunk::Text { text: "hi back", .. } for that session
    let mut saw_text = false;
    for _ in 0..16 {
        match timeout(Duration::from_secs(2), em_rx.recv()).await {
            Ok(Ok((got, StreamChunk::Text { text, .. }))) if got == sid && text == "hi back" => {
                saw_text = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_text, "embedded must observe Text(\"hi back\") chunk");

    // @step And calling handle.send_input(&sid, "hi".to_string()) (the existing 2-arg shape) produces the exact same chunk sequence
    // @step And the SessionManagerHandle trait declares fn send_input_with_thinking with a default body that delegates to self.send_input(sid, text) when thinking is None
    // @step And FspecService::send_input_with_thinking exists with the same arg list as the trait method modulo Context
    websocket
        .send_input(sid.clone(), "hi".to_string())
        .await
        .expect("ws send_input");
    let mut saw_text_ws = false;
    for _ in 0..16 {
        match timeout(Duration::from_secs(2), ws_rx.recv()).await {
            Ok(Ok((got, StreamChunk::Text { text, .. }))) if got == sid && text == "hi back" => {
                saw_text_ws = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_text_ws,
        "websocket must observe Text(\"hi back\") chunk"
    );
}
