/// Scenario: restore_session_messages + restore_session_token_state
/// round-trip Ok on both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn restore_session_messages_and_token_state_round_trip() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds a freshly-created session id from backend.create_session(None).await
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step When the engineer calls backend.restore_session_messages(sid, vec!["{}".to_string()]).await on either transport
    embedded
        .restore_session_messages(sid.clone(), vec!["{}".to_string()])
        .await
        .expect("em restore messages");
    websocket
        .restore_session_messages(sid.clone(), vec!["{}".to_string()])
        .await
        .expect("ws restore messages");
    // @step Then the call returns Ok(())

    // @step When the engineer calls backend.restore_session_token_state(sid, TokenRestoreState { current_context: 1, cumulative_billed_output: 2, cache_read: 3, cache_creation: 4, cumulative_billed_input: 5, cumulative_billed_output_second: 6 }).await
    let state = TokenRestoreState {
        current_context: 1,
        cumulative_billed_output: 2,
        cache_read: 3,
        cache_creation: 4,
        cumulative_billed_input: 5,
        cumulative_billed_output_second: 6,
    };
    embedded
        .restore_session_token_state(sid.clone(), state.clone())
        .await
        .expect("em restore token");
    websocket
        .restore_session_token_state(sid, state)
        .await
        .expect("ws restore token");
    // @step Then the call returns Ok(())
}

/// Scenario: work-unit context get/set round-trips through both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn work_unit_context_round_trips_across_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.expect("create_session");

    let ctx = WorkUnitContext {
        id: "AUTH-001".into(),
        title: "Login".into(),
        status: "implementing".into(),
    };
    // @step When the engineer calls backend.set_work_unit_context(sid, Some(WorkUnitContext { id: "AUTH-001".into(), title: "Login".into(), status: "implementing".into() })).await
    embedded
        .set_work_unit_context(sid.clone(), Some(ctx.clone()))
        .await
        .expect("em set ctx");
    // @step Then the call returns Ok(())
    // @step When the engineer calls backend.get_work_unit_context(sid).await
    let ws_ctx = websocket
        .get_work_unit_context(sid.clone())
        .await
        .expect("ws get ctx");
    // @step Then the call returns Ok(Some(WorkUnitContext { id: "AUTH-001", title: "Login", status: "implementing" }))
    assert_eq!(ws_ctx, Some(ctx));

    // @step When the engineer calls backend.set_work_unit_context(sid, None).await followed by backend.get_work_unit_context(sid).await
    websocket
        .set_work_unit_context(sid.clone(), None)
        .await
        .expect("ws clear ctx");
    let em_ctx = embedded
        .get_work_unit_context(sid)
        .await
        .expect("em get ctx after clear");
    // @step Then the second get call returns Ok(None)
    assert_eq!(em_ctx, None);
}

/// Scenario: pending_input draft text round-trips through both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pending_input_round_trips_across_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step When the engineer calls backend.set_pending_input(sid, Some("draft text".to_string())).await on either transport
    embedded
        .set_pending_input(sid.clone(), Some("draft text".to_string()))
        .await
        .expect("em set pending");
    // @step Then the call returns Ok(())
    // @step When the engineer calls backend.get_pending_input(sid).await
    let ws_text = websocket
        .get_pending_input(sid.clone())
        .await
        .expect("ws get pending");
    // @step Then the call returns Ok(Some("draft text".to_string()))
    assert_eq!(ws_text, Some("draft text".to_string()));

    // @step When the engineer calls backend.set_pending_input(sid, None).await followed by backend.get_pending_input(sid).await
    websocket
        .set_pending_input(sid.clone(), None)
        .await
        .expect("ws clear pending");
    let em_text = embedded
        .get_pending_input(sid)
        .await
        .expect("em get pending after clear");
    // @step Then the second get call returns Ok(None)
    assert_eq!(em_text, None);
}

/// Scenario: active session tracking get/set/clear round-trips.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn active_session_tracking_round_trips_across_transports() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds two distinct session ids minted via create_session
    let sid_a = embedded.create_session(None).await.expect("create a");
    let sid_b = embedded.create_session(None).await.expect("create b");

    // @step When the engineer calls backend.set_active_session(sid_a).await
    embedded
        .set_active_session(sid_a.clone())
        .await
        .expect("em set active a");
    // @step Then backend.get_active_session().await returns Ok(Some(sid_a))
    assert_eq!(
        websocket.get_active_session().await.expect("ws get active"),
        Some(sid_a)
    );

    // @step When the engineer calls backend.set_active_session(sid_b).await
    websocket
        .set_active_session(sid_b.clone())
        .await
        .expect("ws set active b");
    // @step Then backend.get_active_session().await returns Ok(Some(sid_b))
    assert_eq!(
        embedded.get_active_session().await.expect("em get active"),
        Some(sid_b)
    );

    // @step When the engineer calls backend.clear_active_session().await
    embedded
        .clear_active_session()
        .await
        .expect("em clear active");
    // @step Then backend.get_active_session().await returns Ok(None)
    assert_eq!(
        websocket
            .get_active_session()
            .await
            .expect("ws get active after clear"),
        None
    );
}

/// Scenario: get_effective_cwd / get_supervisors return safe defaults.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn effective_cwd_and_supervisors_safe_defaults() {
    let (_temp, service, _stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.expect("create_session");

    // @step When the engineer calls backend.get_effective_cwd(sid).await
    let em_cwd = embedded
        .get_effective_cwd(sid.clone())
        .await
        .expect("em cwd");
    let ws_cwd = websocket
        .get_effective_cwd(sid.clone())
        .await
        .expect("ws cwd");
    // @step Then the call returns Ok(PathBuf::from("")) (the stub default — an empty PathBuf)
    assert_eq!(em_cwd, ws_cwd, "effective_cwd must match across transports");
    assert!(em_cwd.is_empty(), "stub default is empty string");

    // @step When the engineer calls backend.get_supervisors(sid).await
    let em_sup = embedded.get_supervisors(sid.clone()).await.expect("em sup");
    let ws_sup = websocket.get_supervisors(sid).await.expect("ws sup");
    // @step Then the call returns Ok(Vec::new())
    assert_eq!(em_sup, ws_sup);
    assert!(em_sup.is_empty());
}
