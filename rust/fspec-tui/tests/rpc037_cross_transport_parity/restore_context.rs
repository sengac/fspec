/// Scenario: restore_session_messages + restore_session_token_state
/// round-trip Ok on both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn restore_session_messages_and_token_state_round_trip() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id from backend.create_session(None).await
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    // @step When the engineer calls backend.restore_session_messages(sid, vec!["{}".to_string()]).await on either transport
    embedded
        .restore_session_messages(sid.clone(), vec!["{}".to_string()])
        .await
        .or_fail("em restore messages")?;
    websocket
        .restore_session_messages(sid.clone(), vec!["{}".to_string()])
        .await
        .or_fail("ws restore messages")?;
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
        .or_fail("em restore token")?;
    websocket
        .restore_session_token_state(sid, state)
        .await
        .or_fail("ws restore token")?;
    // @step Then the call returns Ok(())
    Ok(())
}

/// Scenario: work-unit context get/set round-trips through both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn work_unit_context_round_trips_across_transports() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    let ctx = WorkUnitContext {
        id: "AUTH-001".into(),
        title: "Login".into(),
        status: "implementing".into(),
    };
    // @step When the engineer calls backend.set_work_unit_context(sid, Some(WorkUnitContext { id: "AUTH-001".into(), title: "Login".into(), status: "implementing".into() })).await
    embedded
        .set_work_unit_context(sid.clone(), Some(ctx.clone()))
        .await
        .or_fail("em set ctx")?;
    // @step Then the call returns Ok(())
    // @step When the engineer calls backend.get_work_unit_context(sid).await
    let ws_ctx = websocket
        .get_work_unit_context(sid.clone())
        .await
        .or_fail("ws get ctx")?;
    // @step Then the call returns Ok(Some(WorkUnitContext { id: "AUTH-001", title: "Login", status: "implementing" }))
    assert_eq!(ws_ctx, Some(ctx));

    // @step When the engineer calls backend.set_work_unit_context(sid, None).await followed by backend.get_work_unit_context(sid).await
    websocket
        .set_work_unit_context(sid.clone(), None)
        .await
        .or_fail("ws clear ctx")?;
    let em_ctx = embedded
        .get_work_unit_context(sid)
        .await
        .or_fail("em get ctx after clear")?;
    // @step Then the second get call returns Ok(None)
    assert_eq!(em_ctx, None);
    Ok(())
}

/// Scenario: pending_input draft text round-trips through both transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pending_input_round_trips_across_transports() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    // @step When the engineer calls backend.set_pending_input(sid, Some("draft text".to_string())).await on either transport
    embedded
        .set_pending_input(sid.clone(), Some("draft text".to_string()))
        .await
        .or_fail("em set pending")?;
    // @step Then the call returns Ok(())
    // @step When the engineer calls backend.get_pending_input(sid).await
    let ws_text = websocket
        .get_pending_input(sid.clone())
        .await
        .or_fail("ws get pending")?;
    // @step Then the call returns Ok(Some("draft text".to_string()))
    assert_eq!(ws_text, Some("draft text".to_string()));

    // @step When the engineer calls backend.set_pending_input(sid, None).await followed by backend.get_pending_input(sid).await
    websocket
        .set_pending_input(sid.clone(), None)
        .await
        .or_fail("ws clear pending")?;
    let em_text = embedded
        .get_pending_input(sid)
        .await
        .or_fail("em get pending after clear")?;
    // @step Then the second get call returns Ok(None)
    assert_eq!(em_text, None);
    Ok(())
}

/// Scenario: active session tracking get/set/clear round-trips.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn active_session_tracking_round_trips_across_transports() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds two distinct session ids minted via create_session
    let sid_a = embedded.create_session(None).await.or_fail("create a")?;
    let sid_b = embedded.create_session(None).await.or_fail("create b")?;

    // @step When the engineer calls backend.set_active_session(sid_a).await
    embedded
        .set_active_session(sid_a.clone())
        .await
        .or_fail("em set active a")?;
    // @step Then backend.get_active_session().await returns Ok(Some(sid_a))
    assert_eq!(
        websocket.get_active_session().await.or_fail("ws get active")?,
        Some(sid_a)
    );

    // @step When the engineer calls backend.set_active_session(sid_b).await
    websocket
        .set_active_session(sid_b.clone())
        .await
        .or_fail("ws set active b")?;
    // @step Then backend.get_active_session().await returns Ok(Some(sid_b))
    assert_eq!(
        embedded.get_active_session().await.or_fail("em get active")?,
        Some(sid_b)
    );

    // @step When the engineer calls backend.clear_active_session().await
    embedded
        .clear_active_session()
        .await
        .or_fail("em clear active")?;
    // @step Then backend.get_active_session().await returns Ok(None)
    assert_eq!(
        websocket
            .get_active_session()
            .await
            .or_fail("ws get active after clear")?,
        None
    );
    Ok(())
}

/// Scenario: get_effective_cwd / get_supervisors return safe defaults.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn effective_cwd_and_supervisors_safe_defaults() -> TestResult {
    let (_temp, service, _stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;
    // @step Given an engineer holds a freshly-created session id
    let sid = embedded.create_session(None).await.or_fail("create_session")?;

    // @step When the engineer calls backend.get_effective_cwd(sid).await
    let em_cwd = embedded.get_effective_cwd(sid.clone()).await;
    let ws_cwd = websocket.get_effective_cwd(sid.clone()).await;
    // @step Then the call fails on both transports because the stub has no working directory for the session
    // BUG-437 / BUG-459: an unknown working directory is an error, never "".
    for (transport, cwd) in [("embedded", em_cwd), ("websocket", ws_cwd)] {
        let error = cwd.err().map(|error| error.to_string()).unwrap_or_default();
        assert!(
            error.contains("no working directory"),
            "{transport}: {error}"
        );
    }

    // @step When the engineer calls backend.get_supervisors(sid).await
    let em_sup = embedded.get_supervisors(sid.clone()).await.or_fail("em sup")?;
    let ws_sup = websocket.get_supervisors(sid).await.or_fail("ws sup")?;
    // @step Then the call returns Ok(Vec::new())
    assert_eq!(em_sup, ws_sup);
    assert!(em_sup.is_empty());
    Ok(())
}
