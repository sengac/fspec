//! Feature: spec/features/copilot-live-session-opaque-lineage.feature

#[path = "support/bug268_environment.rs"]
mod environment;
#[path = "support/bug268_live.rs"]
mod support;

use environment::{Environment, TestResult};
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn same_account_short_token_refresh_preserves_live_ownership() -> TestResult {
    // @step Given a live Copilot session owns compatible opaque and its short token expires
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let server = support::server().await;
    support::auth(root, &server, "synthetic-account-one").await?;
    let mut live = support::Live::new(root, None)?;
    live.turn("first genuine task before refresh").await?;
    let issuer = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({
            "token":"synthetic-refreshed-copilot","expires_at":u64::MAX,"endpoints":{"api":server.uri()}
        }))).expect(1).mount(&issuer).await;
    let _guard =
        codelet_providers::copilot::test_support::TestIssuerGuard::install(root, &issuer.uri())?;
    let mut auth = codelet_providers::copilot::read_copilot_auth()
        .await?
        .ok_or("missing temporary auth")?;
    auth.copilot_token_expires_at = Some(0);
    codelet_providers::copilot::write_copilot_auth(&auth).await?;
    // @step When the next genuine turn refreshes against a validated local synthetic issuer
    live.turn("second distinct task after expiry").await?;
    // @step Then inference uses refreshed Authorization and retains the exact owned opaque
    let requests = server.received_requests().await.ok_or("missing capture")?;
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].headers["authorization"],
        "Bearer synthetic-refreshed-copilot"
    );
    assert_eq!(
        support::request(&server, 1).await?["reasoning_opaque"],
        support::OPAQUE
    );
    // @step And no extra inference replay or cross-account state is introduced
    assert_eq!(
        issuer
            .received_requests()
            .await
            .ok_or("missing issuer capture")?
            .len(),
        1
    );
    let refreshed = codelet_providers::copilot::read_copilot_auth()
        .await?
        .ok_or("missing refreshed auth")?;
    assert_eq!(refreshed.github_oauth_token, "synthetic-account-one");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn distinct_genuine_turns_preserve_owned_opaque_through_reconstruction() -> TestResult {
    // @step Given a real Copilot background session receives owned encrypted opaque on its first genuine user turn
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let server = support::server().await;
    support::auth(root, &server, "synthetic-account-one").await?;
    let mut live = support::Live::new(root, None)?;
    live.turn("first genuine user task").await?;
    assert!(support::request(&server, 0)
        .await?
        .get("reasoning_opaque")
        .is_none());
    // @step When a second distinct send_input reconstructs its provider through normal shared dispatch
    live.turn("second distinct genuine task").await?;
    let second = support::request(&server, 1).await?;
    // @step Then request two includes the exact owned opaque value unchanged
    assert_eq!(second["reasoning_opaque"], support::OPAQUE);
    // @step And ordinary history tools attribution and armed Done remain valid
    let serialized = serde_json::to_string(&second)?;
    assert!(
        serialized.contains("first genuine user task")
            && serialized.contains("second distinct genuine task")
    );
    assert!(serialized.contains("owned-call") && serialized.contains("Done"));
    let persisted = codelet_core::persistence::load_session(live.background.id)?;
    assert!(!serde_json::to_string(&persisted)?.contains(support::OPAQUE));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn changed_provenance_or_incompatible_history_invalidates_opaque() -> TestResult {
    // @step Given a live Copilot session has successfully committed opaque lineage
    let environment = Environment::new()?;
    let root = environment.directory.path();
    for change in [
        "model",
        "account",
        "deployment",
        "history",
        "generation",
        "provider",
        "preamble",
        "rewrite",
        "tool_ids",
    ] {
        let server = support::server().await;
        let other = support::server().await;
        support::auth(root, &server, "synthetic-account-one").await?;
        let mut live = support::Live::new(root, None)?;
        live.turn("first genuine task").await?;
        // @step When model account deployment provider or compatible history changes or compaction resets generation
        match change {
            "model" => live
                .background
                .inner
                .lock()
                .await
                .provider_manager_mut()
                .set_model_direct("github-copilot", "gpt-6", Some(200000), Some(4096), None)?,
            "account" => support::auth(root, &server, "synthetic-account-two").await?,
            "deployment" => support::auth(root, &other, "synthetic-account-one").await?,
            "history" => live.background.inner.lock().await.messages.clear(),
            "provider" => {
                let mut inner = live.background.inner.lock().await;
                inner.switch_provider("openai")?;
                inner.switch_provider("github-copilot")?;
                inner.provider_manager_mut().set_model_direct(
                    "github-copilot",
                    "gpt-5",
                    Some(200000),
                    Some(4096),
                    None,
                )?;
            }
            "preamble" => live
                .background
                .set_role("changed Copilot session preamble".into()),
            "rewrite" => {
                let mut inner = live.background.inner.lock().await;
                let mut rewritten = false;
                for message in &mut inner.messages {
                    if let rig::message::Message::User { content } = message {
                        for item in content.iter_mut() {
                            if let rig::message::UserContent::Text(text) = item {
                                if text.text.contains("first genuine task") {
                                    text.text = text
                                        .text
                                        .replace("first genuine task", "rewritten prior task");
                                    rewritten = true;
                                }
                            }
                        }
                    }
                }
                assert!(rewritten, "fixture did not find the original genuine task");
            }
            "tool_ids" => {
                let mut inner = live.background.inner.lock().await;
                let mut calls = 0;
                let mut results = 0;
                for message in &mut inner.messages {
                    match message {
                        rig::message::Message::Assistant { content, .. } => {
                            for item in content.iter_mut() {
                                if let rig::message::AssistantContent::ToolCall(call) = item {
                                    call.id = "rewritten-item".into();
                                    call.call_id = Some("rewritten-call".into());
                                    calls += 1;
                                }
                            }
                        }
                        rig::message::Message::User { content } => {
                            for item in content.iter_mut() {
                                if let rig::message::UserContent::ToolResult(result) = item {
                                    result.id = "rewritten-item".into();
                                    result.call_id = Some("rewritten-call".into());
                                    results += 1;
                                }
                            }
                        }
                    }
                }
                assert!(
                    calls > 0 && results > 0,
                    "fixture lacks a paired settled tool result"
                );
            }
            _ => live
                .background
                .inner
                .lock()
                .await
                .provider_session
                .transition_generation(9)?,
        }
        live.turn("next task after change").await?;
        // @step Then no old opaque is replayed and late failed cancelled or superseded settlement cannot restore it
        let outgoing = if change == "deployment" {
            support::request(&other, 0).await?
        } else {
            support::request(&server, 1).await?
        };
        assert!(outgoing.get("reasoning_opaque").is_none(), "{change}");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn new_and_restored_sessions_cannot_adopt_opaque_lineage() -> TestResult {
    // @step Given another or restored session has caller supplied transcripts or checkpoints mentioning opaque data
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let server = support::server().await;
    support::auth(root, &server, "synthetic-account-one").await?;
    let mut original = support::Live::new(root, None)?;
    original.turn("original live task").await?;
    let inner = original.background.inner.lock().await;
    let mut fork = support::Live::new(root, Some(&inner))?;
    drop(inner);
    fork.background
        .inner
        .lock()
        .await
        .provider_session
        .restore(&[json!({"reasoning_opaque":support::OPAQUE}).to_string()])?;
    // @step When its actual Copilot request is prepared
    fork.turn("separate restored task").await?;
    // @step Then it does not adopt or send that opaque data
    assert!(support::request(&server, 1)
        .await?
        .get("reasoning_opaque")
        .is_none());
    // @step And serialization diagnostics checkpoint writers and persisted history exclude owned opaque
    for id in [original.background.id, fork.background.id] {
        let persisted = codelet_core::persistence::load_session(id)?;
        assert!(!serde_json::to_string(&persisted)?.contains(support::OPAQUE));
    }
    // @step And the independent original live session still retains its own lineage
    original
        .turn("original session remains independent")
        .await?;
    assert_eq!(
        support::request(&server, 2).await?["reasoning_opaque"],
        support::OPAQUE
    );
    Ok(())
}
