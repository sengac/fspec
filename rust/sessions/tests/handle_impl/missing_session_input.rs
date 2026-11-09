//! Feature: spec/features/implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager.feature
//!
//! BUG-287 / BUG-328: input for a session the daemon does not hold is never
//! dropped silently and never resumes it implicitly: it is refused with an
//! error, and a persisted session accepts input once it is resumed.
use super::*;

/// An OpenAI-compatible endpoint that answers every request with `reply`.
async fn openai_endpoint(reply: &str) -> wiremock::MockServer {
    let server = wiremock::MockServer::start().await;
    let chunk = serde_json::json!({"id":"bug287","object":"chat.completion.chunk","created":1,"model":"o3",
        "choices":[{"index":0,"delta":{"role":"assistant","content":reply},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":20,"completion_tokens":2,"total_tokens":22}});
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n")),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_input_to_a_persisted_session_is_refused_until_resumed() -> support::TestResult {
    // @step Given a session persisted by one SessionManager and absent from a fresh one
    let (_directory, original, session_id, _) = support::manager().await?;
    codelet_core::persistence::update_session_manifest(
        uuid::Uuid::parse_str(&session_id.value)?,
        |manifest| {
            manifest.provider = "openai/o3".into();
            Ok(())
        },
    )?;
    drop(original);
    codelet_core::persistence::reset_stores_for_tests();
    let server = openai_endpoint("resumed").await;
    let previous: Vec<_> = ["OPENAI_API_KEY", "OPENAI_BASE_URL"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
    std::env::set_var("OPENAI_API_KEY", "offline");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    let fresh = SessionManager::new();
    fresh.set_hooks(Arc::new(codelet_agent_loop::FspecAgentHooks::new()));
    assert!(fresh.get_session(&session_id.value).is_err());
    let handle: Arc<dyn SessionManagerHandle> = fresh.clone();

    // @step When "send_input" is called for that session on the fresh manager's handle
    let refused = handle.send_input(&session_id, "pick up where we left off".into());

    // @step Then the call fails with an error telling the client to resume the session first
    let error = refused
        .err()
        .ok_or("input to a session that is not live must fail")?;
    assert!(error.contains(&session_id.value), "{error}");
    assert!(error.contains("resume_session"), "{error}");
    // @step And the fresh manager has not resumed the session
    assert!(fresh.get_session(&session_id.value).is_err());

    // @step And after "resume_session" the same input is accepted as a turn
    handle.resume_session(&session_id)?;
    handle.send_input(&session_id, "pick up where we left off".into())?;
    let resumed = fresh.get_session(&session_id.value);
    let reached_provider = codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            let requests = server.received_requests().await.unwrap_or_default();
            if requests.iter().any(|request| {
                String::from_utf8_lossy(&request.body).contains("pick up where we left off")
            }) {
                return;
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await;
    if let Ok(session) = &resumed {
        session.interrupt();
    }
    for (key, value) in previous {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    resumed?;
    assert!(
        reached_provider.is_ok(),
        "the resumed session's turn never reached the provider"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_input_to_an_unknown_session_is_refused() -> support::TestResult {
    // @step Given a session id with no persisted session
    let (_directory, manager, _, _) = support::manager().await?;
    let unknown = SessionId::new(uuid::Uuid::new_v4().to_string());
    let handle: Arc<dyn SessionManagerHandle> = manager.clone();

    // @step When "send_input", "send_input_with_thinking" and "submit_input" are called for it on the handle
    let results = [
        handle.send_input(&unknown, "hello".into()),
        handle.send_input_with_thinking(&unknown, "hello".into(), None),
        handle.submit_input(
            &unknown,
            "hello".into(),
            codelet_rpc_types::InputMode::Steer,
        ),
    ];

    // @step Then each call fails with an error naming the session
    for result in results {
        let error = result
            .err()
            .ok_or("input to an unknown session must fail")?;
        assert!(error.contains(&unknown.value), "{error}");
    }

    // @step And no session is created
    assert!(manager.get_session(&unknown.value).is_err());
    Ok(())
}
