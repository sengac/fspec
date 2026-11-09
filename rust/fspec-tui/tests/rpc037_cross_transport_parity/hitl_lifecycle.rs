/// Scenario: HITL request/response round-trips through both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hitl_request_response_round_trips_across_transports() -> TestResult {
    let (_temp, service, stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer seeds the StubSessionManagerHandle with a HitlRequest { id: "q-1", question: "Apply?", header: "Apply", options: [HitlOption{label:"Yes",..}, HitlOption{label:"No",..}], allow_text_input: true } for sid
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    // RPC-410: TS-parity wire shape — multi-question request.
    let req = HitlRequest {
        questions: vec![HitlQuestion {
            id: "q-1".into(),
            header: "Apply".into(),
            question: "Apply?".into(),
            options: vec![
                HitlOption {
                    label: "Yes".into(),
                    description: String::new(),
                },
                HitlOption {
                    label: "No".into(),
                    description: String::new(),
                },
            ],
        }],
    };
    let seeded = pending("question-1", &sid, RequestKind::Question, req);
    stub.seed_hitl_request(sid.clone(), seeded.clone());

    // @step When the engineer calls backend.get_hitl_request(sid).await
    let em_req = embedded
        .get_hitl_request(sid.clone())
        .await
        .or_fail("em get hitl")?;
    let ws_req = websocket
        .get_hitl_request(sid.clone())
        .await
        .or_fail("ws get hitl")?;
    // @step Then the call returns Ok(Some(<the seeded HitlRequest>))
    assert_eq!(em_req, ws_req);
    assert_eq!(em_req, Some(seeded));

    // @step When the engineer calls backend.send_hitl_response(sid, HitlResponse { id: "q-1".into(), value: "Yes".into() }).await
    // RPC-410: structured cancel-capable response shape.
    embedded
        .send_hitl_response(
            sid.clone(),
            "question-1".into(),
            HitlResponse {
                cancelled: false,
                answers: vec![HitlAnswer {
                    id: "q-1".into(),
                    selected: vec!["Yes".into()],
                    other: None,
                }],
            },
        )
        .await
        .or_fail("em send hitl")?;
    // @step Then the call returns Ok(())
    // @step And backend.get_hitl_request(sid).await subsequently returns Ok(None)
    assert_eq!(
        websocket
            .get_hitl_request(sid)
            .await
            .or_fail("ws get hitl after send")?,
        None
    );
    Ok(())
}

/// Scenario: send_fspec_result round-trips Ok on both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn send_fspec_result_round_trips_across_transports() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    let result = FspecResult {
        success: true,
        data: "{}".into(),
        error: None,
        system_reminder: None,
        tool_call_id: "tc-1".into(),
    };
    // @step When the engineer calls backend.send_fspec_result(sid, FspecResult { success: true, data: "{}".into(), error: None, system_reminder: None, tool_call_id: "tc-1".into() }).await on either transport
    embedded
        .send_fspec_result(sid.clone(), result.clone())
        .await
        .or_fail("em send_fspec_result")?;
    websocket
        .send_fspec_result(sid, result)
        .await
        .or_fail("ws send_fspec_result")?;
    // @step Then the call returns Ok(())
    Ok(())
}

/// Scenario: create_isolated_session returns IsolatedSessionInfo and the
/// session is listed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_isolated_session_appears_in_list_sessions() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    // @step Given an engineer holds an EmbeddedFspecBackend backed by the StubSessionManagerHandle
    let (embedded, websocket) = dual_backends(service).await?;

    // @step When the engineer calls backend.create_isolated_session(Some("reviewer".to_string())).await
    let em_info = embedded
        .create_isolated_session(Some("reviewer".to_string()))
        .await
        .or_fail("em create_isolated_session")?;
    // @step Then the call returns Ok(IsolatedSessionInfo { session_id: <minted SessionId>, worktree_path: non-empty String, base_commit: non-empty String })
    assert!(!em_info.worktree_path.is_empty());
    assert!(!em_info.base_commit.is_empty());

    // @step And backend.list_sessions().await contains a SessionInfo with id == iso_info.session_id.value and is_isolated == true
    let em_list = embedded
        .list_sessions(String::new())
        .await
        .or_fail("em list_sessions")?;
    assert!(em_list
        .iter()
        .any(|s| s.id == em_info.session_id.value && s.is_isolated));

    // @step And calling the same on WebSocketFspecBackend against a server hosting the SAME stub produces an IsolatedSessionInfo with the SAME deterministic shape
    let ws_info = websocket
        .create_isolated_session(Some("reviewer".to_string()))
        .await
        .or_fail("ws create_isolated_session")?;
    let ws_list = websocket
        .list_sessions(String::new())
        .await
        .or_fail("ws list_sessions")?;
    assert!(ws_list
        .iter()
        .any(|s| s.id == ws_info.session_id.value && s.is_isolated));
    Ok(())
}

/// Scenario: set_thinking_level_default closes the tarpc-side gap and
/// round-trips through both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn set_thinking_level_default_round_trips_across_transports() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    // @step When the engineer calls backend.set_thinking_level_default(sid, ThinkingLevel::Effort("high")).await on either transport
    embedded
        .set_thinking_level_default(sid.clone(), ThinkingLevel::Effort("high".to_string()))
        .await
        .or_fail("em set_thinking_level_default")?;
    websocket
        .set_thinking_level_default(sid, ThinkingLevel::Effort("high".to_string()))
        .await
        .or_fail("ws set_thinking_level_default")?;
    // @step Then the call returns Ok(())
    // @step And FspecService::set_thinking_level_default exists in rust/rpc/src/lib.rs as a new tarpc method on the service trait
    Ok(())
}

/// Scenario: destroy_session removes the session from list_sessions on
/// both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn destroy_session_removes_session_across_transports() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id via backend.create_session(None).await
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    // @step And backend.list_sessions().await contains sid
    assert!(embedded
        .list_sessions(String::new())
        .await
        .or_fail("em list pre")?
        .iter()
        .any(|s| s.id == sid.value));

    // @step When the engineer calls backend.destroy_session(sid).await
    embedded
        .destroy_session(sid.clone())
        .await
        .or_fail("em destroy")?;
    // @step Then the call returns Ok(())
    // @step And backend.list_sessions().await no longer contains sid
    assert!(websocket
        .list_sessions(String::new())
        .await
        .or_fail("ws list post")?
        .iter()
        .all(|s| s.id != sid.value));
    Ok(())
}
